use glam::{U16Vec3, Vec3};

use crate::{CompactHeightfield, PolygonMesh};

/// Contains triangle meshes that represent detailed height data associated
/// with the polygons in its associated polygon mesh object.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct DetailPolygonMesh {
    /// The sub-mesh data
    pub meshes: Vec<SubMesh>,
    /// The mesh vertices
    pub vertices: Vec<Vec3>,
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
        todo!()
    }
}
