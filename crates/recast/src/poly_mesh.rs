use glam::U16Vec3;

use crate::{Aabb3d, AreaType, CompactHeightfield, RegionId, contours::ContourSet};

/// Represents a polygon mesh suitable for use in building a navigation mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonMesh {
    /// The mesh vertices.
    vertices: Vec<U16Vec3>,
    /// Polygon and neighbor data. [Length: [`Self::max_polygons`] * 2 * `Self::vertices_per_polygon`]
    polygons: Vec<U16Vec3>,
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
    bounding_box: Aabb3d,
    /// The size of each cell. (On the xz-plane.)
    cell_size: f32,
    /// The height of each cell. (The minimum increment along the y-axis.)
    cell_height: f32,
    /// The AABB border size used to generate the source data from which the mesh was derived.
    border_size: u16,
    /// The max error of the polygon edges in the mesh.
    max_edge_error: f32,
}

impl ContourSet {
    /// Builds a polygon mesh from the provided contours.
    pub fn into_polygon_mesh(self) -> PolygonMesh {
        PolygonMesh {
            cell_size: self.cell_size,
            cell_height: self.cell_height,
            border_size: self.border_size,
            max_edge_error: self.max_error,
            ..todo!()
        }
    }
}
