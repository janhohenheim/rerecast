use glam::{U16Vec3, U16Vec4, Vec3A, u16vec3};
use thiserror::Error;

use crate::{
    CompactHeightfield, PolygonMesh, RegionId,
    math::{dir_offset, dir_offset_x, dir_offset_z},
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
    /// The mesh triangles
    pub triangles: Vec<(U16Vec3, usize)>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct SubMesh {
    first_vertex_index: usize,
    vertex_count: usize,
    first_triangle_index: usize,
    triangle_count: usize,
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

        let mut edges = Vec::with_capacity(64);
        let mut tris = Vec::with_capacity(512);
        let mut arr = Vec::with_capacity(512 / 3);
        let mut samples = Vec::with_capacity(512);
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

        let vcap = poly_vert_count + poly_vert_count / 2;
        let tcap = vcap * 2;

        dmesh.vertices = vec![Vec3A::default(); vcap];
        dmesh.triangles = vec![Default::default(); tcap];

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
        }

        todo!()
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
    tris: &mut Vec<usize>,
    edges: &mut Vec<usize>,
    samples: &mut Vec<usize>,
) -> Result<(), DetailPolygonMeshError> {
    // Implementation of build_poly_detail function
    todo!()
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
