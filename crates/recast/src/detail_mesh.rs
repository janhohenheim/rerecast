use glam::{U16Vec3, U16Vec4, Vec3A, u16vec3};

use crate::{CompactHeightfield, PolygonMesh, RegionId, poly_mesh::RC_MESH_NULL_IDX};

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
    ) -> Self {
        let mut dmesh = DetailPolygonMesh::default();
        if mesh.vertices.is_empty() || mesh.polygon_count() == 0 {
            return dmesh;
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
        let mut arr = Vec::with_capacity(512);
        let mut samples = Vec::with_capacity(512);
        let verts = [Vec3A::default(); 256];
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
            todo!();
        }

        todo!()
    }
}

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
        queue: &mut Vec<(u16, u16, usize)>,
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
                            *self.data_at_mut(hx, hz) = s.y;
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
                                queue.push((x, z, i));
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
        todo!()
    }

    #[inline]
    fn data_len(&self) -> usize {
        (self.width * self.height) as usize
    }

    #[inline]
    fn data_at(&self, x: u16, z: u16) -> &u16 {
        &self.data[(x + z * self.width) as usize]
    }

    #[inline]
    fn data_at_mut(&mut self, x: u16, z: u16) -> &mut u16 {
        &mut self.data[(x + z * self.width) as usize]
    }
}

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
