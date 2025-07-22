use glam::{U16Vec2, U16Vec3, Vec3Swizzles as _, u16vec3, uvec3};
use thiserror::Error;

use crate::{
    Aabb3d, AreaType, RegionId,
    contours::{ContourSet, RegionVertexId},
    math::{next, prev},
};

/// Represents a polygon mesh suitable for use in building a navigation mesh.
#[derive(Debug, Default, Clone, PartialEq)]
struct InternalPolygonMesh {
    /// The mesh vertices.
    vertices: Vec<U16Vec3>,
    /// The number of vertices
    nvertices: u16,
    /// Polygon and neighbor data. [Length: [`Self::polygon_count`] * 2 * [`Self::vertices_per_polygon`]
    polygons: Vec<u16>,
    /// The number of polygons.
    npolys: usize,
    /// The region id assigned to each polygon.
    regions: Vec<RegionId>,
    /// The flags assigned to each polygon.
    flags: Vec<u16>,
    /// The area id assigned to each polygon.
    areas: Vec<AreaType>,
    /// The number of allocated polygons
    max_polygons: usize,
    /// The maximum number of vertices per polygon
    vertices_per_polygon: usize,
    /// The bounding box of the mesh in world space.
    aabb: Aabb3d,
    /// The size of each cell. (On the xz-plane.)
    cell_size: f32,
    /// The height of each cell. (The minimum increment along the y-axis.)
    cell_height: f32,
    /// The AABB border size used to generate the source data from which the mesh was derived.
    border_size: u16,
    /// The max error of the polygon edges in the mesh.
    max_edge_error: f32,
}

/// Represents a polygon mesh suitable for use in building a navigation mesh.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PolygonMesh {
    /// The mesh vertices.
    pub vertices: Vec<U16Vec3>,
    /// Polygon and neighbor data. [Length: [`Self::polygon_count`] * 2 * [`Self::vertices_per_polygon`]
    pub polygons: Vec<u16>,
    /// The region id assigned to each polygon.
    pub regions: Vec<RegionId>,
    /// The flags assigned to each polygon.
    pub flags: Vec<u16>,
    /// The area id assigned to each polygon.
    pub areas: Vec<AreaType>,
    /// The number of allocated polygons
    pub max_polygons: usize,
    /// The maximum number of vertices per polygon
    pub vertices_per_polygon: usize,
    /// The bounding box of the mesh in world space.
    pub aabb: Aabb3d,
    /// The size of each cell. (On the xz-plane.)
    pub cell_size: f32,
    /// The height of each cell. (The minimum increment along the y-axis.)
    pub cell_height: f32,
    /// The AABB border size used to generate the source data from which the mesh was derived.
    pub border_size: u16,
    /// The max error of the polygon edges in the mesh.
    pub max_edge_error: f32,
}

impl PolygonMesh {
    /// The number of polygons in the mesh. Note that this is different from `polygons.len()`.
    pub fn polygon_count(&self) -> usize {
        self.polygons.len() / (2 * self.vertices_per_polygon)
    }
}

impl From<InternalPolygonMesh> for PolygonMesh {
    fn from(mut value: InternalPolygonMesh) -> Self {
        value.polygons.truncate(value.npolys);
        value.vertices.truncate(value.nvertices as usize);
        PolygonMesh {
            vertices: value.vertices,
            polygons: value.polygons,
            regions: value.regions,
            flags: value.flags,
            areas: value.areas,
            max_polygons: value.max_polygons,
            vertices_per_polygon: value.vertices_per_polygon,
            aabb: value.aabb,
            cell_size: value.cell_size,
            cell_height: value.cell_height,
            border_size: value.border_size,
            max_edge_error: value.max_edge_error,
        }
    }
}

impl ContourSet {
    /// Builds a polygon mesh from the provided contours.
    pub fn into_polygon_mesh(
        self,
        max_vertices_per_polygon: usize,
    ) -> Result<PolygonMesh, PolygonMeshError> {
        let mut mesh = InternalPolygonMesh {
            aabb: self.aabb,
            cell_size: self.cell_size,
            cell_height: self.cell_height,
            border_size: self.border_size,
            max_edge_error: self.max_error,
            vertices_per_polygon: max_vertices_per_polygon,
            ..Default::default()
        };
        let nvp = max_vertices_per_polygon;

        let mut max_vertices = 0;
        let mut max_tris = 0;
        let mut max_verts_per_cont = 0;
        for contour in &self.contours {
            // Skip null contours.
            if contour.vertices.len() < 3 {
                continue;
            }
            max_vertices += contour.vertices.len();
            max_tris += contour.vertices.len() - 2;
            max_verts_per_cont = max_verts_per_cont.max(contour.vertices.len());
        }

        if max_vertices > u16::MAX as usize {
            // Jan: Is this sensible? It's the original, but I suspect u32 is fine
            return Err(PolygonMeshError::TooManyVertices {
                actual: max_vertices,
                max: u16::MAX as usize,
            });
        }

        let mut vflags = vec![false; max_vertices];
        mesh.vertices = vec![U16Vec3::ZERO; max_vertices];
        // Jan: no clue why this might be initialized to 255 specifically??????
        mesh.polygons = vec![u8::MAX as u16; max_tris * nvp * 2];
        mesh.regions = vec![RegionId::default(); max_tris];
        mesh.areas = vec![AreaType::default(); max_tris];

        let mut next_vert = vec![Some(0); max_vertices / 3];
        let mut first_vert = [None; VERTEX_BUCKET_COUNT];
        let mut indices = vec![0; max_verts_per_cont];
        let mut tris = vec![U16Vec3::ZERO; max_verts_per_cont];
        // Jan: the original code initializes this later, but there's not really a reason to do so.
        let mut polys = vec![u8::MAX as u16; (max_verts_per_cont + 1) * nvp];

        let temp_poly_index = max_verts_per_cont * nvp;

        for cont in &self.contours {
            // Skip null contours.
            if cont.vertices.len() < 3 {
                continue;
            }

            // Triangulate contour
            #[expect(clippy::needless_range_loop)]
            for j in 0..cont.vertices.len() {
                indices[j] = j;
            }

            // Jan: we treat an invalid triangulation as an error instead of a warning.
            let ntris = triangulate(&cont.vertices, &mut indices, &mut tris)?;

            // Add and merge vertices.
            for j in 0..(mesh.nvertices as usize) {
                let (v, region) = &cont.vertices[j];
                indices[j] = add_vertex(
                    *v,
                    &mut mesh.vertices,
                    &mut first_vert,
                    &mut next_vert,
                    &mut mesh.nvertices,
                ) as usize;
                if (region & RegionVertexId::BORDER_VERTEX.bits() as usize) != 0 {
                    // This vertex should be removed.
                    vflags[indices[j]] = true;
                }
            }
            // Build initial polygons.
            let mut npolys = 0;
            for t in tris.iter().take(ntris) {
                if t.x != t.y && t.x != t.z && t.y != t.z {
                    polys[npolys * nvp] = indices[t.x as usize] as u16;
                    polys[npolys * nvp + 1] = indices[t.y as usize] as u16;
                    polys[npolys * nvp + 2] = indices[t.z as usize] as u16;
                    npolys += 1;
                }
            }
            if npolys == 0 {
                continue;
            }

            // Merge polygons.
            if nvp > 3 {
                loop {
                    // Find best polygons to merge.
                    let mut best_merge_val = 0;
                    let mut best_pa = 0;
                    let mut best_pb = 0;
                    let mut best_ea = 0;
                    let mut best_eb = 0;
                    for j in 0..(npolys - 1) {
                        let pj = &polys[(j * nvp)..];
                        for k in (j + 1)..npolys {
                            let pk = &polys[(k * nvp)..];
                            let result = get_poly_merge_value(pj, pk, &mesh.vertices, nvp);
                            if let Some(PolyMergeValue {
                                length_squared: v,
                                edge_a: ea,
                                edge_b: eb,
                            }) = result
                                && v > best_merge_val
                            {
                                best_merge_val = v;
                                best_pa = j;
                                best_pb = k;
                                best_ea = ea;
                                best_eb = eb;
                            };
                        }
                    }
                    if best_merge_val > 0 {
                        // Found best, merge.
                        let pa_index = best_pa * nvp;
                        let pb_index = best_pb * nvp;
                        merge_poly_verts(
                            &mut polys,
                            pa_index,
                            pb_index,
                            best_ea,
                            best_eb,
                            temp_poly_index,
                            nvp,
                        );
                        let last_poly = (npolys - 1) * nvp;
                        if pb_index != last_poly {
                            polys.copy_within(last_poly..last_poly + nvp, pb_index);
                        }
                        npolys -= 1;
                    } else {
                        // Could not merge any polygons, stop.
                        break;
                    }
                }
            }

            // Store polygons.
            for j in 0..npolys {
                let p = &mut mesh.polygons[mesh.npolys * nvp * 2..];
                let q = &polys[j * nvp..];
                p[..nvp].copy_from_slice(&q[..nvp]);
                mesh.regions[mesh.npolys] = cont.region;
                mesh.areas[mesh.npolys] = cont.area;
                mesh.npolys += 1;
                if mesh.npolys > max_tris {
                    // Jan: we are comparing polys with tris. Why? Shouldn't we compare polys with polys?
                    return Err(PolygonMeshError::TooManyPolygons {
                        actual: mesh.npolys,
                        max: max_tris,
                    });
                }
            }
        }

        mesh.polygons.truncate(mesh.npolys * nvp * 2);
        // Remove edge vertices.
        let mut i = 0;
        while i < mesh.nvertices as usize {
            if !vflags[i] {
                i += 1;
                continue;
            };
            if !mesh.can_remove_vertex(i as u16) {
                i += 1;
                continue;
            }
            mesh.remove_vertex(i as u16, max_tris)?;
            // Remove vertex
            // Note: nverts is already decremented inside removeVertex()!
            // Fixup vertex flags
            vflags.copy_within((i + 1)..=mesh.nvertices as usize, i);
        }
        // Calculate adjacency.
        mesh.build_mesh_adjacency()?;

        // Find portal edges
        if self.border_size > 0 {
            let w = self.width;
            let h = self.height;
            for i in 0..mesh.npolys {
                let p = &mut mesh.polygons[i * 2 * nvp..];
                for j in 0..nvp {
                    if p[j] == RC_MESH_NULL_IDX {
                        break;
                    }
                    // Skip connected edges.
                    if p[nvp + j] != RC_MESH_NULL_IDX {
                        continue;
                    }
                    let nj = j + 1;
                    let nj = if nj >= nvp || p[nj] == RC_MESH_NULL_IDX {
                        0
                    } else {
                        nj
                    };
                    let va = mesh.vertices[p[j] as usize];
                    let vb = mesh.vertices[p[nj] as usize];
                    if va.x == 0 && vb.x == 0 {
                        p[nvp + j] = RegionId::BORDER_REGION.bits();
                    } else if va.z == h && vb.z == h {
                        p[nvp + j] = RegionId::BORDER_REGION.bits() | 1;
                    } else if va.x == w && vb.x == w {
                        p[nvp + j] = RegionId::BORDER_REGION.bits() | 2;
                    } else if va.z == 0 && vb.z == 0 {
                        p[nvp + j] = RegionId::BORDER_REGION.bits() | 3;
                    }
                }
            }
        }
        // Just allocate the mesh flags array. The user is resposible to fill it.
        mesh.flags = vec![0; mesh.npolys];
        // Jan: Rust's type system makes it impossible for the number of verts and polys to be greater than the max index.

        Ok(mesh.into())
    }
}

#[derive(Debug, Default, Clone)]
struct Edge {
    vert: U16Vec2,
    poly_edge: U16Vec2,
    poly: U16Vec2,
}

impl InternalPolygonMesh {
    fn build_mesh_adjacency(&mut self) -> Result<(), PolygonMeshError> {
        let nvp = self.vertices_per_polygon;
        // Based on code by Eric Lengyel from:
        // https://web.archive.org/web/20080704083314/http://www.terathon.com/code/edges.php
        let max_edge_count = self.npolys * nvp;
        let mut first_edge = vec![RC_MESH_NULL_IDX; self.nvertices as usize + max_edge_count];
        let next_edge_index = self.nvertices as usize;
        let mut edge_count = 0;
        let mut edges = vec![Edge::default(); max_edge_count];
        for i in 0..self.npolys {
            let t = &self.polygons[i * nvp * 2..];
            for j in 0..nvp {
                if t[j] == RC_MESH_NULL_IDX {
                    break;
                }
                let v0 = t[j];
                let v1 = if j + 1 >= nvp || t[j + 1] == RC_MESH_NULL_IDX {
                    t[0]
                } else {
                    t[j + 1]
                };
                if v0 < v1 {
                    let edge = &mut edges[edge_count];
                    edge.vert.x = v0;
                    edge.vert.y = v1;
                    edge.poly.x = i as u16;
                    edge.poly_edge.x = j as u16;
                    edge.poly.y = i as u16;
                    edge.poly_edge.y = 0;
                    // Insert edge
                    first_edge[next_edge_index + edge_count] = first_edge[v0 as usize];
                    first_edge[v0 as usize] = edge_count as u16;
                    edge_count += 1;
                }
            }
        }
        for i in 0..self.npolys {
            let t = &self.polygons[i * nvp * 2..];
            let nv = count_poly_verts(t, nvp);
            for j in 0..nv {
                if t[j] == RC_MESH_NULL_IDX {
                    break;
                }
                let v0 = t[j];
                let v1 = if j + 1 >= nvp || t[j + 1] == RC_MESH_NULL_IDX {
                    t[0]
                } else {
                    t[j + 1]
                };
                if v0 > v1 {
                    let mut e = first_edge[v1 as usize];
                    while e != RC_MESH_NULL_IDX {
                        let edge = &mut edges[e as usize];
                        if edge.vert.y == v0 && edge.poly.x == edge.poly.y {
                            edge.poly.y = i as u16;
                            edge.poly_edge.y = j as u16;
                            break;
                        }
                        e = first_edge[next_edge_index + e as usize];
                    }
                }
            }
        }

        // Store adjacency
        for e in edges.iter().take(edge_count) {
            if e.poly.x != e.poly.y {
                {
                    let p0 = &mut self.polygons[e.poly.x as usize * nvp * 2..];
                    p0[nvp + e.poly_edge.x as usize] = e.poly.y;
                }
                let p1 = &mut self.polygons[e.poly.y as usize * nvp * 2..];
                p1[nvp + e.poly_edge.y as usize] = e.poly.x;
            }
        }
        Ok(())
    }

    fn remove_vertex(&mut self, rem: u16, max_tris: usize) -> Result<(), PolygonMeshError> {
        let nvp = self.vertices_per_polygon;

        // Count number of polygons to remove.
        let mut num_removed_verts = 0;
        for i in 0..self.npolys {
            let p = &self.polygons[i * nvp * 2..];
            let nv = count_poly_verts(p, nvp);
            for pj in p.iter().take(nv) {
                if *pj == rem {
                    num_removed_verts += 1;
                }
            }
        }

        let mut nedges = 0;
        // Format: [polygon1, polygon2, region, area]
        #[derive(Debug, Clone, Default)]
        struct Edge {
            polygon1: u16,
            polygon2: u16,
            region: RegionId,
            area: AreaType,
        }
        let mut edges = vec![Edge::default(); num_removed_verts * nvp];
        let mut nhole = 0;
        let mut hole = vec![0; num_removed_verts * nvp];
        let mut nhreg = 0;
        let mut hreg = vec![RegionId::default(); num_removed_verts * nvp];
        let mut nharea = 0;
        let mut harea = vec![AreaType::default(); num_removed_verts * nvp];

        let mut i = 0;
        while i < self.npolys {
            let i1 = i * nvp * 2;
            {
                // Scope p immutable in this scope
                let p = &self.polygons[i1..];
                let nv = count_poly_verts(p, nvp);
                let has_rem = (0..nv).any(|j| p[j] == rem);
                if !has_rem {
                    i += 1;
                    continue;
                }
                // Collect edges which does not touch the removed vertex.
                for (j, k) in (0..nv).zip(nv - 1..) {
                    if p[j] != rem && p[k] != rem {
                        let e = &mut edges[nedges];
                        e.polygon1 = p[k];
                        e.polygon2 = p[j];
                        e.region = self.regions[i];
                        e.area = self.areas[i];
                        nedges += 1;
                    }
                }
            }
            // Remove the polygon.
            let i2 = (self.npolys - 1) * nvp * 2;
            if i1 != i2 {
                self.polygons.copy_within(i2..(i2 + nvp), i1);
            }
            self.polygons[i1 + nvp..(i1 + 2 * nvp)].fill(u8::MAX as u16);
            self.regions[i] = self.regions[self.npolys - 1];
            self.areas[i] = self.areas[self.npolys - 1];
            self.npolys -= 1;
        }

        // Remove vertex.
        for i in rem..self.nvertices - 1 {
            let i = i as usize;
            self.vertices[i] = self.vertices[i + 1];
        }
        self.nvertices -= 1;

        // Adjust indices to match the removed vertex layout.
        for i in 0..self.npolys {
            let p = &mut self.polygons[i * nvp * 2..];
            let nv = count_poly_verts(p, nvp);
            for pj in p.iter_mut().take(nv) {
                if *pj > rem {
                    *pj -= 1;
                }
            }
        }
        for edge in edges.iter_mut().take(nedges) {
            if edge.polygon1 > rem {
                edge.polygon1 -= 1;
            }
            if edge.polygon2 > rem {
                edge.polygon2 -= 1;
            }
        }

        if nedges == 0 {
            return Ok(());
        }

        // Start with one vertex, keep appending connected
        // segments to the start and end of the hole.
        push_back(edges[0].polygon1, &mut hole, &mut nhole);
        push_back(edges[0].region, &mut hreg, &mut nhreg);
        push_back(edges[0].area, &mut harea, &mut nharea);

        while nedges != 0 {
            let mut match_ = false;
            let mut i = 0;
            while i < nedges {
                let edge = edges[i].clone();
                let ea = edge.polygon1;
                let eb = edge.polygon2;
                let r = edge.region;
                let a = edge.area;
                let mut add = false;
                if hole[0] == eb {
                    // The segment matches the beginning of the hole boundary.
                    push_front(ea, &mut hole, &mut nhole);
                    push_front(r, &mut hreg, &mut nhreg);
                    push_front(a, &mut harea, &mut nharea);
                    add = true;
                    i += 1;
                } else if hole[nhole - 1] == ea {
                    // The segment matches the end of the hole boundary.
                    push_back(eb, &mut hole, &mut nhole);
                    push_back(r, &mut hreg, &mut nhreg);
                    push_back(a, &mut harea, &mut nharea);
                    add = true;
                    i += 1;
                }
                if add {
                    // The edge segment was added, remove it.
                    edges[i] = edges[nedges - 1].clone();
                    nedges -= 1;
                    match_ = true;
                }
            }
            if !match_ {
                break;
            }
        }
        let mut tris = vec![U16Vec3::default(); nhole];
        let mut tverts = vec![(U16Vec3::default(), 0); nhole];
        let mut thole = vec![0; nhole];

        // Generate temp vertex array for triangulation.
        for i in 0..nhole {
            let pi = hole[i] as usize;
            tverts[i].0 = self.vertices[pi];
            thole[i] = i;
        }

        // Triangulate the hole.
        // Jan: we treat errors here as a hard error instead of printing a warning.
        let ntris = triangulate(&tverts, &mut thole, &mut tris)?;

        // Merge the hole triangles back to polygons.
        let mut polys = vec![0; (ntris + 1) * nvp];
        let mut pregs = vec![RegionId::default(); ntris];
        let mut pareas = vec![AreaType::default(); ntris];
        let tmp_poly_index = ntris * nvp;

        // Build initial polygons.
        let mut npolys = 0;
        polys[..ntris * nvp].fill(u8::MAX as u16);
        for t in tris.iter().take(ntris) {
            if t.x != t.y && t.x != t.z && t.y != t.z {
                let t_x = t.x as usize;
                let t_y = t.y as usize;
                let t_z = t.z as usize;
                polys[npolys * nvp] = hole[t_x];
                polys[npolys * nvp + 1] = hole[t_y];
                polys[npolys * nvp + 2] = hole[t_z];

                // If this polygon covers multiple region types then
                // mark it as such
                if hreg[t_x] != hreg[t_y] || hreg[t_y] != hreg[t_z] {
                    pregs[npolys] = RegionId::NONE;
                } else {
                    pregs[npolys] = hreg[t_x];
                }
                pareas[npolys] = harea[t_x];
                npolys += 1;
            }
        }
        if npolys == 0 {
            return Ok(());
        }

        // Merge polygons.
        if nvp > 3 {
            loop {
                // Find best polygons to merge.
                let mut best_merge_val = 0;
                let mut best_pa = 0;
                let mut best_pb = 0;
                let mut best_ea = 0;
                let mut best_eb = 0;
                for j in 0..npolys - 1 {
                    let pj = &polys[j * nvp..];
                    for k in (j + 1)..npolys {
                        let pk = &polys[k * nvp..];
                        let value = get_poly_merge_value(pj, pk, &self.vertices, nvp);
                        if let Some(value) = value
                            && value.length_squared > best_merge_val
                        {
                            best_merge_val = value.length_squared;
                            best_pa = j;
                            best_pb = k;
                            best_ea = value.edge_a;
                            best_eb = value.edge_b;
                        }
                    }
                }

                if best_merge_val > 0 {
                    // Found best, merge.
                    let pa_index = best_pa * nvp;
                    let pb_index = best_pb * nvp;
                    merge_poly_verts(
                        &mut polys,
                        pa_index,
                        pb_index,
                        best_ea,
                        best_eb,
                        tmp_poly_index,
                        nvp,
                    );
                    if pregs[best_pa] != pregs[best_pb] {
                        pregs[best_pa] = RegionId::NONE;
                    }

                    let last_index = (npolys - 1) * nvp;
                    if pb_index != last_index {
                        polys.copy_within(last_index..last_index + nvp, pb_index);
                    }
                    pregs[best_pb] = pregs[npolys - 1];
                    pareas[best_pb] = pareas[npolys - 1];
                    npolys -= 1;
                } else {
                    // Cound not merge any polygons, stop.
                    break;
                }
            }
        }

        // Store polygons.
        for i in 0..npolys {
            if self.npolys >= max_tris {
                break;
            }
            let p = &mut self.polygons[self.npolys * nvp * 2..self.npolys * nvp * 2 + nvp * 2];
            p[..nvp * 2].fill(u8::MAX as u16);
            for j in 0..nvp {
                p[j] = polys[i * nvp + j];
            }
            self.regions[self.npolys] = pregs[i];
            self.areas[self.npolys] = pareas[i];
            self.npolys += 1;
            if self.npolys >= max_tris {
                return Err(PolygonMeshError::TooManyPolygons {
                    actual: self.npolys,
                    max: max_tris,
                });
            }
        }

        Ok(())
    }

    fn can_remove_vertex(&self, rem: u16) -> bool {
        let nvp = self.vertices_per_polygon;

        // Count number of polygons to remove.
        let mut num_touched_verts = 0;
        let mut num_remaining_edges = 0;
        for i in 0..self.npolys {
            let p = &self.polygons[i * nvp * 2..];
            let nv = count_poly_verts(p, nvp);
            let mut num_removed = 0;
            let mut num_verts = 0;
            for pj in p.iter().take(nv) {
                if *pj == rem {
                    num_touched_verts += 1;
                    num_removed += 1;
                }
                num_verts += 1;
            }
            if num_removed != 0 {
                num_remaining_edges += num_verts - (num_removed + 1);
            }
        }

        // There would be too few edges remaining to create a polygon.
        // This can happen for example when a tip of a triangle is marked
        // as deletion, but there are no other polys that share the vertex.
        // In this case, the vertex should not be removed.
        if num_remaining_edges <= 2 {
            return false;
        }
        // Find edges which share the removed vertex.
        let max_edges = num_touched_verts * 2;
        let mut nedges = 0;
        // Format: [poly1, poly2, vertex share count]
        let mut edges = vec![U16Vec3::ZERO; max_edges];
        for i in 0..self.npolys {
            let p = &self.polygons[i * nvp * 2..];
            let nv = count_poly_verts(p, nvp);

            // Collect edges which touches the removed vertex.
            for (j, k) in (0..nv).zip((nv - 1)..) {
                if !(p[j] == rem || p[k] == rem) {
                    continue;
                }
                // Arrange edge so that a=rem.
                let a = p[j];
                let b = p[k];
                let (a, b) = if b != rem { (a, b) } else { (b, a) };

                // Check if the edge exists
                let mut exists = false;
                for e in edges.iter_mut().take(nedges) {
                    if e[1] == b {
                        // Exists, increment vertex share count.
                        e[2] += 1;
                        exists = true;
                    }
                }
                // Add new edge
                if !exists {
                    let e = &mut edges[nedges];
                    e[0] = a;
                    e[1] = b;
                    e[2] = 1;
                    nedges += 1;
                }
            }
        }

        // There should be no more than 2 open edges.
        // This catches the case that two non-adjacent polygons
        // share the removed vertex. In that case, do not remove the vertex.
        let num_open_edges = edges.iter().filter(|e| e[2] < 2).count();
        num_open_edges <= 2
    }
}

fn push_back<T>(value: T, vec: &mut [T], index: &mut usize) {
    vec[*index] = value;
    *index += 1;
}

fn push_front<T: Clone>(value: T, vec: &mut [T], index: &mut usize) {
    *index += 1;
    for i in (1..*index).rev() {
        vec[i] = vec[i - 1].clone();
    }
    vec[0] = value;
}

// Jan: signature changed to align with the borrow checker :)
fn merge_poly_verts(
    polys: &mut [u16],
    pa_index: usize,
    pb_index: usize,
    ea: usize,
    eb: usize,
    tmp_index: usize,
    nvp: usize,
) {
    let na = count_poly_verts(&polys[pa_index..], nvp);
    let nb = count_poly_verts(&polys[pb_index..], nvp);

    // Merge polygons.
    polys[tmp_index..tmp_index + nvp].fill(u8::MAX as u16);
    let mut n = 0;
    // Add pa
    for i in 0..na - 1 {
        polys[tmp_index + n] = polys[pa_index + (ea + 1 + i) % na];
        n += 1;
    }
    // Add pb
    for i in 0..nb - 1 {
        polys[tmp_index + n] = polys[pb_index + (eb + 1 + i) % nb];
        n += 1;
    }

    // Implicit assumption of the original code
    polys.copy_within(tmp_index..tmp_index + nvp, pa_index);
}

fn get_poly_merge_value(
    pa: &[u16],
    pb: &[u16],
    verts: &[U16Vec3],
    nvp: usize,
) -> Option<PolyMergeValue> {
    let na = count_poly_verts(pa, nvp);
    let nb = count_poly_verts(pb, nvp);

    // If the merged polygon would be too big, do not merge.
    if na + nb - 2 > nvp {
        return None;
    }

    // Check if the polygons share an edge.
    let mut ea = None;
    let mut eb = None;

    for i in 0..na {
        let va0 = pa[i];
        let va1 = pa[next(i, na)];
        let (va0, va1) = if va0 <= va1 { (va0, va1) } else { (va1, va0) };
        for j in 0..nb {
            let vb0 = pb[j];
            let vb1 = pb[next(j, nb)];
            let (vb0, vb1) = if vb0 <= vb1 { (vb0, vb1) } else { (vb1, vb0) };
            if va0 == vb0 && va1 == vb1 {
                ea = Some(i);
                eb = Some(j);
                break;
            }
        }
    }

    // No common edge, cannot merge.
    let (ea, eb) = (ea?, eb?);

    // Check to see if the merged polygon would be convex.
    let mut va = pa[(ea + na - 1) % na] as usize;
    let mut vb = pa[ea] as usize;
    let mut vc = pb[(eb + 2) % nb] as usize;
    if !uleft(verts[va], verts[vb], verts[vc]) {
        return None;
    }

    va = pb[(eb + nb - 1) % nb] as usize;
    vb = pb[eb] as usize;
    vc = pa[(ea + 2) % na] as usize;
    if !uleft(verts[va], verts[vb], verts[vc]) {
        return None;
    };

    va = pa[ea] as usize;
    vb = pa[(ea + 1) % na] as usize;

    let d = verts[va] - verts[vb];
    let length_squared = d.xz().as_uvec2().length_squared();
    Some(PolyMergeValue {
        length_squared,
        edge_a: ea,
        edge_b: eb,
    })
}

#[inline]
fn uleft(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    let cross = (b.x as i32 - a.x as i32) * (c.z as i32 - a.z as i32)
        - (c.x as i32 - a.x as i32) * (b.z as i32 - a.z as i32);
    cross < 0
}

fn count_poly_verts(p: &[u16], nvp: usize) -> usize {
    p.iter()
        .take(nvp)
        .position(|p| *p == RC_MESH_NULL_IDX)
        .unwrap_or(nvp)
}

/// A value which indicates an invalid index within a mesh.
/// This does not necessarily indicate an error.
const RC_MESH_NULL_IDX: u16 = 0xffff;

struct PolyMergeValue {
    length_squared: u32,
    edge_a: usize,
    edge_b: usize,
}

fn add_vertex(
    vertex: U16Vec3,
    verts: &mut [U16Vec3],
    first_vert: &mut [Option<u16>],
    next_vert: &mut [Option<u16>],
    nverts: &mut u16,
) -> u16 {
    let bucket = compute_vertex_hash(u16vec3(vertex.x, 0, vertex.z));
    let mut i_iter = first_vert[bucket];

    while let Some(i) = i_iter {
        let v = verts[i as usize];
        if v.x == vertex.x && (v.y as i32 - vertex.y as i32).abs() <= 2 && v.z == vertex.z {
            return i;
        }
        i_iter = next_vert[i as usize];
    }

    // Could not find, create new.
    let i = *nverts;
    *nverts += 1;
    verts[i as usize] = vertex;
    next_vert[i as usize] = first_vert[bucket];
    first_vert[bucket] = Some(i);

    i
}

fn compute_vertex_hash(vertex: U16Vec3) -> usize {
    let h = uvec3(
        0x8da6b343, // Large multiplicative constants;
        0xd8163841, // here arbitrarily chosen primes
        0xcb1ab31f,
    );
    let n = h.dot(vertex.as_uvec3());
    n as usize & (VERTEX_BUCKET_COUNT - 1)
}

const VERTEX_BUCKET_COUNT: usize = 1 << 12;

fn triangulate(
    verts: &[(U16Vec3, usize)],
    indices: &mut [usize],
    tris: &mut [U16Vec3],
) -> Result<usize, PolygonMeshError> {
    let mut n = verts.len();
    let mut ntris = 0;

    // The last bit of the index is used to indicate if the vertex can be removed.
    for i in 0..n {
        let i1 = next(i, n);
        let i2 = next(i1, n);
        if is_diagonal(i, i2, verts, indices) {
            indices[i1] |= CAN_REMOVE;
        }
    }
    while n > 3 {
        let mut min_len = None;
        let mut mini = None;
        for i in 0..n {
            let i1 = next(i, n);
            if (indices[i1] & CAN_REMOVE) != 0 {
                let p0 = verts[indices[i] & INDEX_MASK].0;
                let p2 = verts[indices[next(i1, n)] & INDEX_MASK].0;

                let d = p2.as_ivec3() - p0.as_ivec3();
                let len = d.xz().length_squared() as u16;
                if min_len.is_none() || min_len.is_none_or(|min| len < min) {
                    min_len = Some(len);
                    mini = Some(i);
                }
            }
        }
        if mini.is_none() {
            // We might get here because the contour has overlapping segments, like this:
            //
            //  A o-o=====o---o B
            //   /  |C   D|    \.
            //  o   o     o     o
            //  :   :     :     :
            // We'll try to recover by loosing up the inCone test a bit so that a diagonal
            // like A-B or C-D can be found and we can continue.
            min_len = None;
            for i in 0..n {
                let i1 = next(i, n);
                let i2 = next(i1, n);
                if is_diagonal_loose(i, i2, verts, indices) {
                    let p0 = verts[indices[i] & INDEX_MASK].0;
                    let p2 = verts[indices[next(i2, n)] & INDEX_MASK].0;
                    let d = p2.as_ivec3() - p0.as_ivec3();
                    let len = d.xz().length_squared() as u16;
                    if min_len.is_none() || min_len.is_none_or(|min| len < min) {
                        min_len = Some(len);
                        mini = Some(i);
                    }
                }
            }
        }

        let Some(mini) = mini else {
            // The contour is messed up. This sometimes happens
            // if the contour simplification is too aggressive.
            return Err(PolygonMeshError::InvalidContour);
        };

        let mut i = mini;
        let mut i1 = next(i, n);
        let i2 = next(i1, n);

        tris[ntris].x = (indices[i] & CAN_REMOVE) as u16;
        tris[ntris].y = (indices[i1] & CAN_REMOVE) as u16;
        tris[ntris].z = (indices[i2] & CAN_REMOVE) as u16;
        ntris += 1;

        // Removes P[i1] by copying P[i+1]...P[n-1] left one index.
        n -= 1;
        for k in i1..n {
            indices[k] = indices[k + 1];
        }

        if i1 >= n {
            i1 = 0;
        }
        i = prev(i1, n);
        // Update diagonal flags.
        if is_diagonal(prev(i, n), i1, verts, indices) {
            indices[i] |= CAN_REMOVE;
        } else {
            indices[i] &= INDEX_MASK;
        }

        if is_diagonal(i, next(i1, n), verts, indices) {
            indices[i1] |= CAN_REMOVE;
        } else {
            indices[i1] &= INDEX_MASK;
        }
    }
    // Append the remaining triangle.
    tris[ntris].x = (indices[0] & INDEX_MASK) as u16;
    tris[ntris].y = (indices[1] & INDEX_MASK) as u16;
    tris[ntris].z = (indices[2] & INDEX_MASK) as u16;
    ntris += 1;

    Ok(ntris)
}

const CAN_REMOVE: usize = 0x80000000;

/// Returns true iff (v_i, v_j) is a proper internal diagonal of P.
fn is_diagonal(i: usize, j: usize, verts: &[(U16Vec3, usize)], indices: &[usize]) -> bool {
    in_cone(i, j, verts, indices) && is_diagonal_internal_or_external(i, j, verts, indices)
}

/// Returns true iff the diagonal (i,j) is strictly internal to the
/// polygon P in the neighborhood of the i endpoint.
fn in_cone(i: usize, j: usize, verts: &[(U16Vec3, usize)], indices: &[usize]) -> bool {
    let n = verts.len();
    let pi = verts[indices[i] & INDEX_MASK].0;
    let pj = verts[indices[j] & INDEX_MASK].0;
    let pi1 = verts[indices[next(i, n)] & INDEX_MASK].0;
    let pin1 = verts[indices[prev(i, n)] & INDEX_MASK].0;

    // If P[i] is a convex vertex [ i+1 left or on (i-1,i) ].
    if is_left_on(pin1, pi, pi1) {
        is_left(pi, pj, pin1) && is_left(pj, pi, pi1)
    } else {
        // Assume (i-1,i,i+1) not collinear.
        // else P[i] is reflex.
        !(is_left_on(pi, pj, pi1) && is_left_on(pj, pi, pin1))
    }
}

#[inline]
fn is_left_on(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) <= 0
}

/// Returns true iff c is strictly to the left of the directed line through a to b.
#[inline]
fn is_left(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) < 0
}

#[inline]
fn area2(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> i32 {
    let a = a.as_ivec3();
    let b = b.as_ivec3();
    let c = c.as_ivec3();
    (b.x - a.x) * (c.z - a.z) - (c.x - a.x) * (b.z - a.z)
}

// Returns T iff (v_i, v_j) is a proper internal *or* external
// diagonal of P, *ignoring edges incident to v_i and v_j*.
fn is_diagonal_internal_or_external(
    i: usize,
    j: usize,
    verts: &[(U16Vec3, usize)],
    indices: &[usize],
) -> bool {
    let n = verts.len();
    let d0 = verts[indices[i] & INDEX_MASK].0;
    let d1 = verts[indices[j] & INDEX_MASK].0;

    // For each edge (k,k+1) of P
    for k in 0..n {
        let k1 = next(k, n);
        // Skip edges incident to i or j
        if !((k == i) || (k1 == i) || (k == j) || (k1 == j)) {
            let p0 = verts[indices[k] & INDEX_MASK].0;
            let p1 = verts[indices[k1] & INDEX_MASK].0;
            if vequal(d0, p0) || vequal(d1, p0) || vequal(d0, p1) || vequal(d1, p1) {
                continue;
            }
            if intersect(d0, d1, p0, p1) {
                return false;
            }
        }
    }
    true
}

const INDEX_MASK: usize = 0x0fffffff;

#[inline]
fn vequal(a: U16Vec3, b: U16Vec3) -> bool {
    a.xz() == b.xz()
}

/// Returns true iff segments ab and cd intersect, properly or improperly.
#[inline]
fn intersect(a: U16Vec3, b: U16Vec3, c: U16Vec3, d: U16Vec3) -> bool {
    if intersect_prop(a, b, c, d) {
        return true;
    }
    between(a, b, c) || between(a, b, d) || between(c, d, a) || between(c, d, b)
}

/// Returns true iff ab properly intersects cd: they share
/// a point interior to both segments.  The properness of the
/// intersection is ensured by using strict leftness.
#[inline]
fn intersect_prop(a: U16Vec3, b: U16Vec3, c: U16Vec3, d: U16Vec3) -> bool {
    // Eliminate improper cases.
    if collinear(a, b, c) || collinear(a, b, d) || collinear(c, d, a) || collinear(c, d, b) {
        return false;
    }
    (left(a, b, c) ^ left(a, b, d)) && (left(c, d, a) ^ left(c, d, b))
}

#[inline]
fn collinear(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) == 0
}

/// Returns true iff c is strictly to the left of the directed
/// line through a to b.
#[inline]
fn left(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) < 0
}

#[inline]
fn left_on(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    area2(a, b, c) <= 0
}

/// Returns T iff (a,b,c) are collinear and point c lies
/// on the closed segment ab.
#[inline]
fn between(a: U16Vec3, b: U16Vec3, c: U16Vec3) -> bool {
    if !collinear(a, b, c) {
        return false;
    }
    // If ab not vertical, check betweenness on x; else on z.
    if a.x != b.x {
        (a.x <= c.x && c.x <= b.x) || (a.x >= c.x && c.x >= b.x)
    } else {
        (a.z <= c.z && c.z <= b.z) || (a.z >= c.z && c.z >= b.z)
    }
}

fn is_diagonal_loose(i: usize, j: usize, verts: &[(U16Vec3, usize)], indices: &[usize]) -> bool {
    in_cone_loose(i, j, verts, indices)
        && is_diagonal_internal_or_external_loose(i, j, verts, indices)
}

fn in_cone_loose(i: usize, j: usize, verts: &[(U16Vec3, usize)], indices: &[usize]) -> bool {
    let n = verts.len();
    let pi = verts[indices[i] & INDEX_MASK].0;
    let pj = verts[indices[j] & INDEX_MASK].0;
    let pi1 = verts[indices[next(i, n)] & INDEX_MASK].0;
    let pin1 = verts[indices[prev(i, n)] & INDEX_MASK].0;

    // If P[i] is a convex vertex [ i+1 left or on (i-1,i) ].
    if left_on(pin1, pi, pi1) {
        left_on(pi, pj, pin1) && left_on(pj, pi, pi1)
    } else {
        !(left_on(pi, pj, pi1) && left_on(pj, pi, pin1))
    }
}

fn is_diagonal_internal_or_external_loose(
    i: usize,
    j: usize,
    verts: &[(U16Vec3, usize)],
    indices: &[usize],
) -> bool {
    let n = verts.len();
    let d0 = verts[indices[i] & INDEX_MASK].0;
    let d1 = verts[indices[j] & INDEX_MASK].0;

    // For each edge (k,k+1) of P
    for k in 0..n {
        let k1 = next(k, n);
        // Skip edges incident to i or j
        if !(k == i || k1 == i || k == j || k1 == j) {
            let p0 = verts[indices[k] & INDEX_MASK].0;
            let p1 = verts[indices[k1] & INDEX_MASK].0;
            if vequal(d0, p0) || vequal(d1, p0) || vequal(d0, p1) || vequal(d1, p1) {
                continue;
            }
            if intersect_prop(d0, d1, p0, p1) {
                return false;
            }
        }
    }
    true
}

#[derive(Error, Debug)]
pub enum PolygonMeshError {
    #[error("Too many vertices: {actual} > {max}")]
    TooManyVertices { actual: usize, max: usize },
    #[error("Too many polygons: {actual} > {max}")]
    TooManyPolygons { actual: usize, max: usize },
    #[error(
        "Invalid contour. This sometimes happens if the contour simplification is too aggressive."
    )]
    InvalidContour,
}
