use std::{f32, usize::MAX};

use glam::{U16Vec3, U16Vec4, Vec3A, Vec3Swizzles as _, u16vec3};
use thiserror::Error;

use crate::{
    Aabb3d, CompactHeightfield, PolygonMesh, RegionId,
    math::{
        dir_offset, dir_offset_x, dir_offset_z, distance_squared_between_point_and_line_u16vec2,
        distance_squared_between_point_and_line_vec2, distance_squared_between_point_and_line_vec3,
        next, prev,
    },
    poly_mesh::RC_MESH_NULL_IDX,
};

/// Contains triangle meshes that represent detailed height data associated
/// with the polygons in its associated polygon mesh object.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct DetailPolygonMesh {
    /// The sub-mesh data
    pub meshes: Vec<SubMesh>,
    /// The mesh vertices
    pub vertices: Vec<Vec3A>,
    /// The mesh triangles and their associated metadata
    pub triangles: Vec<(U16Vec3, usize)>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct SubMesh {
    pub first_vertex_index: usize,
    pub vertex_count: usize,
    pub first_triangle_index: usize,
    pub triangle_count: usize,
}

impl DetailPolygonMesh {
    /// Builds a detail mesh from the provided polygon mesh.
    pub fn new(
        mesh: &PolygonMesh,
        heightfield: &CompactHeightfield,
        sample_distance: f32,
        sample_max_error: f32,
    ) -> Result<Self, DetailPolygonMeshError> {
        let mut dmesh = DetailPolygonMesh::default();
        if mesh.vertices.is_empty() || mesh.polygon_count() == 0 {
            return Ok(dmesh);
        }
        let chf = heightfield;
        let nvp = mesh.vertices_per_polygon;
        let cs = mesh.cell_size;
        let ch = mesh.cell_height;
        let orig = mesh.aabb.max;
        let border_size = mesh.border_size;
        let height_search_radius = 1.max(mesh.max_edge_error.ceil() as u32);

        let mut edges = Vec::with_capacity(64 / 4);
        let mut tris = Vec::with_capacity(512 / 4);
        let mut arr = Vec::with_capacity(512 / 3);
        let mut samples = Vec::with_capacity(512 / 4);
        let mut verts = [Vec3A::default(); 256];
        let mut hp = HeightPatch::default();
        let mut poly_vert_count = 0;
        let mut maxhw = 0;
        let mut maxhh = 0;

        let mut bounds = vec![Bounds::default(); mesh.polygon_count()];
        let mut poly = vec![Vec3A::default(); nvp];

        // Find max size for a polygon area.
        for i in 0..mesh.polygon_count() {
            let p = &mesh.polygons[i * nvp * 2..];
            let Bounds {
                xmin,
                xmax,
                zmin,
                zmax,
            } = &mut bounds[i];
            *xmin = chf.width;
            *xmax = 0;
            *zmin = chf.height;
            *zmax = 0;
            for pj in &p[..nvp] {
                if *pj == RC_MESH_NULL_IDX {
                    break;
                }
                let v = &mesh.vertices[*pj as usize];
                *xmin = (*xmin).min(v.x);
                *xmax = (*xmax).max(v.x);
                *zmin = (*zmin).min(v.z);
                *zmax = (*zmax).max(v.z);
                poly_vert_count += 1;
            }
            *xmin = 0.max(*xmin - 1);
            *xmax = chf.width.min(*xmax + 1);
            *zmin = 0.max(*zmin - 1);
            *zmax = chf.height.min(*zmax + 1);
            if xmin >= xmax || zmin >= zmax {
                continue;
            }
            maxhw = maxhw.max(*xmax - *xmin);
            maxhh = maxhh.max(*zmax - *zmin);
        }
        hp.data = vec![0; maxhw as usize * maxhh as usize];
        dmesh.meshes = vec![SubMesh::default(); mesh.polygon_count()];

        let mut vcap = poly_vert_count + poly_vert_count / 2;
        let mut tcap = vcap * 2;

        dmesh.vertices = Vec::with_capacity(vcap);
        dmesh.triangles = Vec::with_capacity(tcap);

        for i in 0..mesh.polygon_count() {
            let p = &mesh.polygons[i * nvp * 2..];

            // Store polygon vertices for processing.
            let mut npoly = 0;
            for j in 0..nvp {
                if p[j] == RC_MESH_NULL_IDX {
                    break;
                }
                let v = mesh.vertices[p[j] as usize].as_vec3();
                poly[j].x = v.x * cs;
                poly[j].y = v.y * ch;
                poly[j].z = v.z * cs;
                npoly += 1;
            }

            // Get the height data from the area of the polygon.
            let bounds_i = &bounds[i];
            hp.xmin = bounds_i.xmin;
            hp.zmin = bounds_i.zmin;
            hp.width = bounds_i.width();
            hp.height = bounds_i.height();
            hp.get_height_data(
                chf,
                p,
                npoly,
                &verts,
                border_size,
                &mut arr,
                mesh.regions[i],
            );

            // Build detail mesh.
            let mut nverts = 0;
            build_poly_detail(
                &poly,
                npoly,
                sample_distance,
                sample_max_error,
                height_search_radius,
                chf,
                &hp,
                &mut verts,
                &mut nverts,
                &mut tris,
                &mut edges,
                &mut samples,
            )?;

            // Move detail verts to world space.
            for vert in &mut verts[..nverts] {
                *vert += orig;
                // [sic] Is this offset necessary?
                vert.y += chf.cell_height;
            }
            // Offset poly too, will be used to flag checking.
            for poly in &mut poly[..npoly] {
                *poly += orig;
            }

            // Store detail submesh
            {
                let submesh = &mut dmesh.meshes[i];
                submesh.first_vertex_index = dmesh.vertices.len();
                submesh.vertex_count = nverts;
                submesh.first_triangle_index = dmesh.triangles.len();
                submesh.triangle_count = tris.len();
            }

            // Store vertices, allocate more memory if necessary.
            if dmesh.vertices.len() + nverts > vcap {
                while dmesh.vertices.len() + nverts > vcap {
                    vcap += 256;
                }
                dmesh.vertices.reserve(vcap - dmesh.vertices.capacity());
            }
            for vert in &verts[..nverts] {
                dmesh.vertices.push(*vert);
            }

            // Store triangles, allocate more memory if necessary.
            if dmesh.triangles.len() + tris.len() > tcap {
                while dmesh.triangles.len() + tris.len() > tcap {
                    tcap += 256;
                }
                dmesh.triangles.reserve(tcap - dmesh.triangles.capacity());
            }
            for tri in &tris {
                dmesh.triangles.push(*tri);
            }
        }

        Ok(dmesh)
    }
}

fn build_poly_detail(
    in_: &[Vec3A],
    nin: usize,
    sample_dist: f32,
    sample_max_error: f32,
    height_search_radius: u32,
    chf: &CompactHeightfield,
    hp: &HeightPatch,
    verts: &mut [Vec3A],
    nverts: &mut usize,
    tris: &mut Vec<(U16Vec3, usize)>,
    edges: &mut Vec<[Option<u16>; 4]>,
    samples: &mut Vec<(U16Vec3, bool)>,
) -> Result<(), DetailPolygonMeshError> {
    const MAX_VERTS: usize = 127;
    // Max tris for delaunay is 2n-2-k (n=num verts, k=num hull verts).
    const MAX_TRIS: usize = 255;
    const MAX_VERTS_PER_EDGE: usize = 32;
    let mut edge = [Vec3A::default(); MAX_VERTS_PER_EDGE + 1];
    let mut hull = [0; MAX_VERTS];
    let mut nhull = 0;

    *nverts = nin;

    verts[..nin].clone_from_slice(&in_[..nin]);
    edges.clear();
    tris.clear();

    let cs = chf.cell_size;
    let ics = 1.0 / cs;

    // Calculate minimum extents of the polygon based on input data.
    let min_extent_squared = poly_min_extent_squared(verts, *nverts);

    // Tessellate outlines.
    // This is done in separate pass in order to ensure
    // seamless height values across the ply boundaries.
    if sample_dist > 0.0 {
        let mut j = nin - 1;
        for i in 0..nin {
            let mut vj = in_[j];
            let mut vi = in_[i];
            let mut swapped = false;
            // Make sure the segments are always handled in same order
            // using lexological sort or else there will be seams.
            if (vj.x - vi.x).abs() < 1.0e-6 {
                if vj.z > vi.z {
                    std::mem::swap(&mut vj, &mut vi);
                    swapped = true;
                }
            } else if vj.x > vi.x {
                std::mem::swap(&mut vj, &mut vi);
                swapped = true;
            }
            // Create samples along the edge.
            let dij = vi - vj;
            let d = dij.length();
            let mut nn = 1 + (d / sample_dist).floor() as usize;
            if nn >= MAX_VERTS_PER_EDGE {
                nn = MAX_VERTS_PER_EDGE - 1;
            }
            if *nverts + nn >= MAX_VERTS {
                nn = MAX_VERTS - 1 - *nverts;
            }
            for k in 0..=nn {
                let u = k as f32 / nn as f32;
                let pos = &mut edge[k];
                *pos = vj + dij * u;
                pos.y = get_height(*pos, ics, chf.cell_height, height_search_radius, &hp) as f32
                    * chf.cell_height;
            }
            // Simplify samples.
            let mut idx = [0; MAX_VERTS_PER_EDGE];
            idx[1] = nn;
            let mut nidx = 2;
            let mut k = 0;
            while k < nidx - 1 {
                let a = idx[k];
                let b = idx[k + 1];
                let va = edge[a];
                let vb = edge[b];
                // Find maximum deviation along the segment.
                let mut maxd = 0.0;
                let mut maxi = None;
                for m in (a + 1)..b {
                    let dev = distance_squared_between_point_and_line_vec3(edge[m], (va, vb));
                    if dev > maxd {
                        maxd = dev;
                        maxi = Some(m);
                    }
                }
                // If the max deviation is larger than accepted error,
                // add new point, else continue to next segment.
                if let Some(maxi) = maxi
                    && maxd > sample_max_error * sample_max_error
                {
                    for m in ((k + 1)..=nidx).rev() {
                        idx[m] = idx[m - 1];
                    }
                    idx[k + 1] = maxi;
                    nidx += 1;
                } else {
                    k += 1;
                }
            }

            hull[nhull + 1] = j;
            nhull += 1;
            // Add new vertices.
            if swapped {
                for k in (1..nidx - 1).rev() {
                    verts[*nverts] = edge[idx[k]];
                    hull[nhull] = *nverts;
                    nhull += 1;
                    *nverts += 1;
                }
            } else {
                for k in 1..nidx - 1 {
                    verts[*nverts] = edge[idx[k]];
                    hull[nhull] = *nverts;
                    nhull += 1;
                    *nverts += 1;
                }
            }
            j = i;
        }
    }

    // If the polygon minimum extent is small (sliver or small triangle), do not try to add internal points.
    if min_extent_squared < (sample_dist * 2.0) * (sample_dist * 2.0) {
        triangulate_hull(verts, nhull, &hull, nin, tris);
        set_tri_flags(tris, nhull, &hull);
        return Ok(());
    }

    // Tessellate the base mesh.
    // We're using the triangulateHull instead of delaunayHull as it tends to
    // create a bit better triangulation for long thin triangles when there
    // are no internal points.
    triangulate_hull(verts, nhull, &hull, nin, tris);

    if tris.is_empty() {
        // Could not triangulate the poly, make sure there is some valid data there.
        tracing::warn!("Could not triangulate polygon ({nverts} verts)");
        // Jan: how is this not an Err?
        return Ok(());
    }

    if sample_dist > 0.0 {
        // Create sample locations in a grid.
        let mut aabb = Aabb3d::default();
        aabb.min = in_[0];
        aabb.max = in_[0];
        for in_ in in_[..nin].iter().copied() {
            aabb.min = aabb.min.min(in_);
            aabb.max = aabb.max.max(in_);
        }
        let x0 = (aabb.min.x / sample_dist).floor() as i32;
        let x1 = (aabb.max.x / sample_dist).ceil() as i32;
        let z0 = (aabb.min.z / sample_dist).floor() as i32;
        let z1 = (aabb.max.z / sample_dist).ceil() as i32;
        samples.clear();
        for z in z0..z1 {
            for x in x0..x1 {
                let mut pt = Vec3A::default();
                pt.x = x as f32 * sample_dist;
                pt.y = (aabb.max.y + aabb.min.y) * 0.5;
                pt.z = z as f32 * sample_dist;
                // Make sure the samples are not too close to the edges.
                // Jan: I believe this check is bugged, see https://github.com/recastnavigation/recastnavigation/issues/788
                if dist_to_poly(nin, in_, pt) > -sample_dist / 2.0 {
                    continue;
                }
                let y = get_height(pt, ics, chf.cell_height, height_search_radius, hp);
                samples.push((u16vec3(x as u16, y, z as u16), false));
            }
        }

        // Add the samples starting from the one that has the most
        // error. The procedure stops when all samples are added
        // or when the max error is within treshold.
        for _iter in 0..samples.len() {
            if *nverts >= MAX_VERTS {
                break;
            }

            // Find sample with most error.
            let mut bestpt = Vec3A::default();
            let mut bestd = 0.0;
            let mut besti = None;
            for (i, (s, added)) in samples.iter().enumerate() {
                if *added {
                    continue;
                }
                let mut pt = Vec3A::default();
                // The sample location is jittered to get rid of some bad triangulations
                // which are cause by symmetrical data from the grid structure.
                pt.x = s.x as f32 * sample_dist + get_jitter_x(i) * cs * 0.1;
                pt.y = s.y as f32 * chf.cell_height;
                pt.z = s.z as f32 * sample_dist + get_jitter_y(i) * cs * 0.1;
                let d = dist_to_tri_mesh(pt, verts, tris);
                let Some(d) = d else {
                    // did not hit the mesh.
                    continue;
                };
                if d > bestd {
                    bestd = d;
                    besti = Some(i);
                    bestpt = pt;
                }
            }
            // If the max error is within accepted threshold, stop tesselating.
            if bestd <= sample_max_error {
                break;
            }
            let Some(besti) = besti else {
                break;
            };
            // Mark sample as added.
            samples[besti].1 = true;
            // Add the new sample point.
            verts[*nverts] = bestpt;
            *nverts += 1;

            // Create new triangulation.
            // [sic] TODO: Incremental add instead of full rebuild.
            edges.clear();
            tris.clear();
            delaunay_hull(*nverts, verts, nhull, &mut hull, tris, edges);
        }
    }
    if tris.len() > MAX_TRIS {
        // Jan: why do we need this?
        tris.truncate(MAX_TRIS);
        tracing::error!(
            "Too many triangles! Shringking triangle count from {} to {MAX_TRIS}",
            tris.len()
        );
    }
    set_tri_flags(tris, nhull, &hull);
    Ok(())
}

fn delaunay_hull(
    npts: usize,
    pts: &[Vec3A],
    nhull: usize,
    hull: &mut [usize],
    tris: &mut Vec<(U16Vec3, usize)>,
    edges: &mut Vec<[Option<u16>; 4]>,
) {
    let mut nfaces = 0;
    let mut nedges = 0;
    let max_edges = npts * 10;
    edges.resize(max_edges, todo!());

    let mut j = nhull - 1;
    for i in 0..nhull {
        todo!("add_edge");
        j = i;
    }

    let mut current_edge = 0;
    while current_edge < nedges {
        todo!();
    }
    todo!()
}

fn dist_to_tri_mesh(p: Vec3A, verts: &[Vec3A], tris: &[(U16Vec3, usize)]) -> Option<f32> {
    let mut dmin = f32::MAX;
    for (tri, _) in tris {
        let va = verts[tri.x as usize];
        let vb = verts[tri.y as usize];
        let vc = verts[tri.z as usize];
        let d = dist_pt_tri(p, va, vb, vc);
        if let Some(d) = d
            && d < dmin
        {
            dmin = d;
        }
    }
    if dmin == f32::MAX { None } else { Some(dmin) }
}

/// Distance from point p to triangle defined by vertices a, b, and c.
/// Returns None if the point is outside the triangle.
fn dist_pt_tri(p: Vec3A, a: Vec3A, b: Vec3A, c: Vec3A) -> Option<f32> {
    let v0 = c - a;
    let v1 = b - a;
    let v2 = p - a;

    let dot00 = v0.xz().dot(v0.xz());
    let dot01 = v0.xz().dot(v1.xz());
    let dot02 = v0.xz().dot(v2.xz());
    let dot11 = v1.xz().dot(v1.xz());
    let dot12 = v1.xz().dot(v2.xz());

    // Compute barycentric coordinates
    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    // If point lies inside the triangle, return interpolated y-coord.
    const EPS: f32 = 1.0e-4;
    if u >= -EPS && v >= -EPS && (u + v) <= 1.0 + EPS {
        let y = a.y + v0.y * u + v1.y * v;
        Some((y - p.y).abs())
    } else {
        None
    }
}

fn get_jitter_x(i: usize) -> f32 {
    (((i * 0x8da6b343) & 0xffff) as f32 / 65535.0 * 2.0) - 1.0
}

fn get_jitter_y(i: usize) -> f32 {
    (((i * 0xd8163841) & 0xffff) as f32 / 65535.0 * 2.0) - 1.0
}

fn dist_to_poly(nvert: usize, verts: &[Vec3A], p: Vec3A) -> f32 {
    let mut dmin = f32::MAX;
    let mut c = false;
    let mut j = nvert - 1;
    for i in 0..nvert {
        let vi = verts[i];
        let vj = verts[j];
        if (vi.z > p.z) != (vj.z > p.z) && p.x < (vj.x - vi.x) * (p.z - vi.z) / (vj.z - vi.z) + vi.x
        {
            c = !c;
        }
        dmin = dmin.min(distance_squared_between_point_and_line_vec2(
            p.xz(),
            (vj.xz(), vi.xz()),
        ));
        j = i;
    }
    if c { -dmin } else { dmin }
}

/// Find edges that lie on hull and mark them as such.
fn set_tri_flags(tris: &mut Vec<(U16Vec3, usize)>, nhull: usize, hull: &[usize]) {
    // Matches DT_DETAIL_EDGE_BOUNDARY
    const DETAIL_EDGE_BOUNDARY: usize = 0x1;

    for (tri, tri_flags) in tris {
        let mut flags = 0;
        flags |= if on_hull(tri.x as usize, tri.y as usize, nhull, hull) {
            DETAIL_EDGE_BOUNDARY
        } else {
            0
        } << 0;
        flags |= if on_hull(tri.y as usize, tri.z as usize, nhull, hull) {
            DETAIL_EDGE_BOUNDARY
        } else {
            0
        } << 2;
        flags |= if on_hull(tri.z as usize, tri.x as usize, nhull, hull) {
            DETAIL_EDGE_BOUNDARY
        } else {
            0
        } << 4;
        *tri_flags = flags;
    }
}

fn on_hull(a: usize, b: usize, nhull: usize, hull: &[usize]) -> bool {
    // All internal sampled points come after the hull so we can early out for those.
    if a >= nhull || b >= nhull {
        return false;
    }
    let mut j = nhull - 1;
    for i in 0..nhull {
        if a == hull[j] && b == hull[i] {
            return true;
        }
        j = i;
    }
    false
}

fn triangulate_hull(
    verts: &[Vec3A],
    nhull: usize,
    hull: &[usize],
    nin: usize,
    tris: &mut Vec<(U16Vec3, usize)>,
) {
    let mut start = 0;
    let mut left = 1;
    let mut right = nhull - 1;

    // Start from an ear with shortest perimeter.
    // This tends to favor well formed triangles as starting point.
    let mut dmin = f32::MAX;
    for i in 0..nhull {
        if hull[i] >= nin {
            // Ears are triangles with original vertices as middle vertex while others are actually line segments on edges
            continue;
        }
        let pi = prev(i, nhull);
        let ni = next(i, nhull);
        let pv = verts[hull[pi]].xz();
        let cv = verts[hull[i]].xz();
        let nv = verts[hull[ni]].xz();
        let d = pv.distance(cv) + cv.distance(nv) + nv.distance(pv);
        if d < dmin {
            start = i;
            left = ni;
            right = pi;
            dmin = d;
        }
    }

    // Add first triangle
    tris.push((
        u16vec3(hull[start] as u16, hull[left] as u16, hull[right] as u16),
        0,
    ));

    // Triangulate the polygon by moving left or right,
    // depending on which triangle has shorter perimeter.
    // This heuristic was chose empirically, since it seems
    // handle tessellated straight edges well.
    while next(left, nhull) != right {
        // Check to see if se should advance left or right.
        let nleft = next(left, nhull);
        let nright = prev(right, nhull);

        let cvleft = verts[hull[left]].xz();
        let nvleft = verts[hull[nleft]].xz();
        let cvright = verts[hull[right]].xz();
        let nvright = verts[hull[nright]].xz();
        let dleft = cvleft.distance(nvleft) + nvleft.distance(cvright);
        let dright = cvright.distance(nvright) + cvleft.distance(nvright);
        if dleft < dright {
            tris.push((
                u16vec3(hull[left] as u16, hull[nleft] as u16, hull[right] as u16),
                0,
            ));
            left = nleft;
        } else {
            tris.push((
                u16vec3(hull[left] as u16, hull[nright] as u16, hull[right] as u16),
                0,
            ));
            right = nright;
        }
    }
}

fn get_height(f: Vec3A, ics: f32, ch: f32, radius: u32, hp: &HeightPatch) -> u16 {
    let mut ix = (f.x * ics + 0.01).floor() as i32;
    let mut iz = (f.z * ics + 0.01).floor() as i32;
    ix = (ix - hp.xmin as i32).clamp(0, hp.width as i32 - 1);
    iz = (iz - hp.zmin as i32).clamp(0, hp.height as i32 - 1);
    let mut h = hp.data[(ix + iz * hp.width as i32) as usize];
    if h == RC_UNSET_HEIGHT {
        // Special case when data might be bad.
        // Walk adjacent cells in a spiral up to 'radius', and look
        // for a pixel which has a valid height.
        let mut x = 1;
        let mut z = 0;
        let mut dx = 1;
        let mut dz = 0;
        let max_size = radius * 2 + 1;
        let max_iter = max_size * max_size - 1;

        let mut next_ring_iter_start = 8;
        let mut next_ring_iters = 16;

        let mut dmin = f32::MAX;
        for i in 0..max_iter {
            let nx = ix + x;
            let nz = iz + z;
            if nx >= 0 && nz >= 0 && nx < hp.width as i32 && nz < hp.height as i32 {
                let nh = hp.data[(nx + nz * hp.width as i32) as usize];
                if nh != RC_UNSET_HEIGHT {
                    let d = (nh as f32 * ch - f.y).abs();
                    if d < dmin {
                        h = nh;
                        dmin = d;
                    }
                }
            }
            // We are searching in a grid which looks approximately like this:
            //  __________
            // |2 ______ 2|
            // | |1 __ 1| |
            // | | |__| | |
            // | |______| |
            // |__________|
            // We want to find the best height as close to the center cell as possible. This means that
            // if we find a height in one of the neighbor cells to the center, we don't want to
            // expand further out than the 8 neighbors - we want to limit our search to the closest
            // of these "rings", but the best height in the ring.
            // For example, the center is just 1 cell. We checked that at the entrance to the function.
            // The next "ring" contains 8 cells (marked 1 above). Those are all the neighbors to the center cell.
            // The next one again contains 16 cells (marked 2). In general each ring has 8 additional cells, which
            // can be thought of as adding 2 cells around the "center" of each side when we expand the ring.
            // Here we detect if we are about to enter the next ring, and if we are and we have found
            // a height, we abort the search.
            if i + 1 == next_ring_iter_start {
                if h != RC_UNSET_HEIGHT {
                    break;
                }
                next_ring_iter_start += next_ring_iters;
                next_ring_iters += 8;
            }

            if x == z || (x < 0 && x == -z) || (x > 0 && x == 1 - z) {
                let tmp = dx;
                dx = -dz;
                dz = tmp;
            }
            x += dx;
            z += dz;
        }
    }
    h
}

/// Calculate minimum extend of the polygon.
fn poly_min_extent_squared(verts: &[Vec3A], nverts: usize) -> f32 {
    let mut min_dist = f32::MAX;
    for i in 0..nverts {
        let ni = next(i, nverts);
        let p1 = verts[i];
        let p2 = verts[ni];
        let mut max_edge_dist = 0.0_f32;
        for j in 0..nverts {
            if j == i || j == ni {
                continue;
            }
            let d = distance_squared_between_point_and_line_vec2(verts[j].xz(), (p1.xz(), p2.xz()));
            max_edge_dist = max_edge_dist.max(d);
        }
        min_dist = min_dist.min(max_edge_dist);
    }
    // Jan: original returns sqrt, but doesn't actually need to
    min_dist
}

#[derive(Error, Debug)]
pub enum DetailPolygonMeshError {}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct HeightPatch {
    data: Vec<u16>,
    xmin: u16,
    zmin: u16,
    width: u16,
    height: u16,
}

impl HeightPatch {
    fn get_height_data(
        &mut self,
        chf: &CompactHeightfield,
        poly: &[u16],
        npoly: usize,
        verts: &[Vec3A],
        bs: u16,
        queue: &mut Vec<(i32, i32, usize)>,
        region: RegionId,
    ) {
        // Note: Reads to the compact heightfield are offset by border size (bs)
        // since border size offset is already removed from the polymesh vertices.
        queue.clear();
        let data_len = self.data_len();
        // Set all heights to RC_UNSET_HEIGHT.
        self.data[..data_len].fill(0xffff);

        let mut empty = true;

        // We cannot sample from this poly if it was created from polys
        // of different regions. If it was then it could potentially be overlapping
        // with polys of that region and the heights sampled here could be wrong.
        if region != RegionId::NONE {
            // Copy the height from the same region, and mark region borders
            // as seed points to fill the rest.
            for hz in 0..self.height {
                let z = self.zmin + hz + bs;
                for hx in 0..self.width {
                    let x = self.xmin + hx + bs;
                    let c = &chf.cells[(x + z * chf.width) as usize];
                    for i in c.index_range() {
                        let s = &chf.spans[i];
                        if s.region == region {
                            // Store height
                            *self.data_at_mut(hx as i32, hz as i32) = s.y;
                            empty = false;

                            // If any of the neighbours is not in same region,
                            // add the current location as flood fill start
                            let mut border = false;
                            for dir in 0..4 {
                                if let Some(con) = s.con(dir) {
                                    let (_ax, _az, ai) =
                                        chf.con_indices(x as i32, z as i32, dir, con);
                                    let as_ = &chf.spans[ai];
                                    if as_.region != region {
                                        border = true;
                                        break;
                                    }
                                }
                            }
                            if border {
                                queue.push((x as i32, z as i32, i));
                            }
                            break;
                        }
                    }
                }
            }
        }
        // if the polygon does not contain any points from the current region (rare, but happens)
        // or if it could potentially be overlapping polygons of the same region,
        // then use the center as the seed point.
        if empty {
            self.seed_array_with_poly_center(chf, poly, npoly, verts, bs, queue);
        }
        const RETRACT_SIZE: usize = 256;
        let mut head = 0;

        // We assume the seed is centered in the polygon, so a BFS to collect
        // height data will ensure we do not move onto overlapping polygons and
        // sample wrong heights.
        while head < queue.len() {
            let (cx, cz, ci) = queue[head];
            head += 1;
            if head >= RETRACT_SIZE {
                head = 0;
                if queue.len() > RETRACT_SIZE {
                    queue.copy_within(RETRACT_SIZE.., 0);
                }
                queue.truncate(queue.len() - RETRACT_SIZE);
            }
            let cs = &chf.spans[ci];
            for dir in 0..4 {
                let Some(con) = cs.con(dir) else {
                    continue;
                };
                let ax = cx + dir_offset_x(dir) as i32;
                let az = cz + dir_offset_z(dir) as i32;
                let hx = ax - self.xmin as i32 - bs as i32;
                let hz = az - self.zmin as i32 - bs as i32;

                if hx as u16 > self.width || hz as u16 >= self.height {
                    continue;
                }

                if *self.data_at(hx, hz) != RC_UNSET_HEIGHT {
                    continue;
                }
                let ai = chf.cells[(ax + az * chf.width as i32) as usize].index() as usize
                    + con as usize;
                let as_ = &chf.spans[ai];

                *self.data_at_mut(hx, hz) = as_.y;
                queue.push((ax, az, ai));
            }
        }
    }

    fn seed_array_with_poly_center(
        &mut self,
        chf: &CompactHeightfield,
        poly: &[u16],
        npoly: usize,
        verts: &[Vec3A],
        bs: u16,
        array: &mut Vec<(i32, i32, usize)>,
    ) {
        // Note: Reads to the compact heightfield are offset by border size (bs)
        // since border size offset is already removed from the polymesh vertices.
        const OFFSET: [i32; 9 * 2] = [0, 0, -1, -1, 0, -1, 1, -1, 1, 0, 1, 1, 0, 1, -1, 1, -1, 0];

        // Find cell closest to a poly vertex
        let mut start_cell_x = 0;
        let mut start_cell_z = 0;
        let mut start_span_index = None;
        let mut dmin = RC_UNSET_HEIGHT as i32;
        for poly_j in poly[..npoly].iter().map(|p| *p as usize) {
            if dmin <= 0 {
                break;
            }
            for k in 0..9 {
                if dmin <= 0 {
                    break;
                }
                let ax = verts[poly_j].x as i32 + OFFSET[k * 2];
                let ay = verts[poly_j].y as i32;
                let az = verts[poly_j].z as i32 + OFFSET[k * 2 + 1];
                if ax < self.xmin as i32
                    || ax >= self.xmin as i32 + self.width as i32
                    || az < self.zmin as i32
                    || az >= self.zmin as i32 + self.height as i32
                {
                    continue;
                };
                let c =
                    &chf.cells[((ax + bs as i32) + (az + bs as i32) * chf.width as i32) as usize];
                for i in c.index_range() {
                    let s = &chf.spans[i];
                    let d = (ay - s.y as i32).abs();
                    if d < dmin {
                        start_cell_x = ax;
                        start_cell_z = az;
                        start_span_index = Some(i);
                        dmin = d;
                    }
                }
            }
        }

        // Jan: Original code also asserts this.
        let start_span_index = start_span_index.expect("Internal error: found no start span");
        // Find center of the polygon
        let mut pcx = 0;
        let mut pcz = 0;
        for poly_j in poly[..npoly].iter().map(|p| *p as usize) {
            // Jan: shouldn't the type conversion happen only at the final value?
            pcx += verts[poly_j].x as i32;
            pcz += verts[poly_j].z as i32;
        }
        pcx /= npoly as i32;
        pcz /= npoly as i32;

        // Use seeds array as a stack for DFS
        array.clear();
        array.push((start_cell_x, start_cell_z, start_span_index));

        let mut dirs = [0, 1, 2, 3];
        let data_len = self.data_len();
        self.data[..data_len].fill(0);
        // DFS to move to the center. Note that we need a DFS here and can not just move
        // directly towards the center without recording intermediate nodes, even though the polygons
        // are convex. In very rare we can get stuck due to contour simplification if we do not
        // record nodes.
        let mut cx = None;
        let mut cz = None;
        let mut ci = None;
        loop {
            if array.is_empty() {
                tracing::warn!("Walk towards polygon center failed to reach center");
                break;
            }

            let (cx_raw, cz_raw, ci_raw) = array.pop().unwrap();
            cx = Some(cx_raw as i32);
            cz = Some(cz_raw as i32);
            ci = Some(ci_raw);
            let cx = cx.unwrap();
            let cz = cz.unwrap();
            let ci = ci.unwrap();

            if cx == pcx && cz == pcz {
                break;
            }

            // If we are already at the correct X-position, prefer direction
            // directly towards the center in the Y-axis; otherwise prefer
            // direction in the X-axis
            let direct_dir = if cx as i32 == pcx {
                dir_offset(0, if pcz > cz { 1 } else { -1 })
            } else {
                dir_offset(if pcx > cx { 1 } else { -1 }, 0)
            } as usize;

            // Push the direct dir last so we start with this on next iteration
            dirs.swap(direct_dir, 3);

            let cs = &chf.spans[ci];
            for i in 0..4 {
                let dir = dirs[i];
                let Some(con) = cs.con(dir) else {
                    continue;
                };

                let new_x = cx + dir_offset_x(dir) as i32;
                let new_z = cz + dir_offset_z(dir) as i32;

                let hpx = new_x - self.xmin as i32;
                let hpz = new_z - self.zmin as i32;
                if hpx < 0 || hpx >= self.width as i32 || hpz < 0 || hpz >= self.height as i32 {
                    continue;
                }
                if *self.data_at(hpx, hpz) != 0 {
                    continue;
                }
                *self.data_at_mut(hpx, hpz) = 1;
                let new_index = chf.cells
                    [((new_x + bs as i32) + (new_z + bs as i32) * chf.width as i32) as usize]
                    .index() as i32
                    + con as i32;
                array.push((new_x, new_z, new_index as usize));
            }
            dirs.swap(direct_dir, 3);
        }

        array.clear();
        // getHeightData seeds are given in coordinates with borders
        let (Some(cx), Some(cz), Some(ci)) = (cx, cz, ci) else {
            // Jan: We panic earlier in the loop before this could even happen.
            unreachable!()
        };
        array.push((cx + bs as i32, cz + bs as i32, ci));
        self.data[..data_len].fill(0xffff);
        let cs = &chf.spans[ci];
        self.data[(cx - self.xmin as i32 + (cz - self.zmin as i32) * self.width as i32) as usize] =
            cs.y;
    }

    #[inline]
    fn data_len(&self) -> usize {
        self.width as usize * self.height as usize
    }

    #[inline]
    fn data_at(&self, x: i32, z: i32) -> &u16 {
        &self.data[(x + z * self.width as i32) as usize]
    }

    #[inline]
    fn data_at_mut(&mut self, x: i32, z: i32) -> &mut u16 {
        &mut self.data[(x + z * self.width as i32) as usize]
    }
}

const RC_UNSET_HEIGHT: u16 = 0xffff;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Bounds {
    xmin: u16,
    xmax: u16,
    zmin: u16,
    zmax: u16,
}
impl Bounds {
    #[inline]
    fn width(&self) -> u16 {
        self.xmax - self.xmin
    }

    #[inline]
    fn height(&self) -> u16 {
        self.zmax - self.zmin
    }
}
