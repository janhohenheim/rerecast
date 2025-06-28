//! Contains traits and methods for converting [`Collider`]s into trimeshes, expressed as [`TrimeshedCollider`]s.

#[cfg(feature = "bevy")]
use bevy::render::mesh::{Mesh, PrimitiveTopology};
use glam::{UVec3, Vec3A};

use crate::{math::Aabb3d, span::AreaType};

/// A mesh used as input for [`Heightfield`](crate::Heightfield) rasterization.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TriMesh {
    /// The vertices composing the collider.
    /// Follows the convention of a triangle list.
    pub vertices: Vec<Vec3A>,

    /// The indices composing the collider.
    /// Follows the convention of a triangle list.
    pub indices: Vec<UVec3>,

    /// The area types of the trimesh. Each index corresponds 1:1 to the [`TriMesh::indices`].
    pub area_types: Vec<AreaType>,
}

impl TriMesh {
    /// Extends the trimesh with the vertices and indices of another trimesh.
    /// The indices of `other` will be offset by the number of vertices in `self`.
    pub fn extend(&mut self, other: TriMesh) {
        if self.vertices.len() > u32::MAX as usize {
            panic!("Cannot extend a trimesh with more than 2^32 vertices");
        }
        let next_vertex_index = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        self.indices
            .extend(other.indices.iter().map(|i| i + next_vertex_index));
        self.area_types.extend(other.area_types);
    }

    /// Computes the AABB of the trimesh.
    /// Returns `None` if the trimesh is empty.
    pub fn compute_aabb(&self) -> Option<Aabb3d> {
        Aabb3d::from_verts(&self.vertices)
    }
}

impl TriMesh {
    #[cfg(feature = "bevy")]
    /// Converts a [`Mesh`] into a [`TriMesh`].
    pub fn from_mesh(mesh: &Mesh) -> Option<TriMesh> {
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            return None;
        }

        let mut trimesh = TriMesh::default();
        let position = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?;
        let float = position.as_float3()?;
        trimesh.vertices = float.iter().map(|v| Vec3A::from(*v)).collect();

        let indices: Vec<_> = mesh.indices()?.iter().collect();
        trimesh.indices = indices
            .windows(3)
            .map(|indices| {
                UVec3::from_array([indices[0] as u32, indices[1] as u32, indices[2] as u32])
            })
            .collect();
        // TODO: accept vertex attributes for this?
        trimesh.area_types = vec![AreaType::NOT_WALKABLE; trimesh.indices.len()];
        Some(trimesh)
    }
}
