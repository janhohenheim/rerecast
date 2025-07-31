use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_mesh::{Mesh, PrimitiveTopology};
use bevy_render::prelude::*;
use bevy_transform::components::GlobalTransform;
use glam::{UVec3, Vec3A};
use rerecast::{AreaType, TriMesh};

use crate::NavmeshAffectorBackendAppExt as _;

/// A backend for [`crate::NavmeshPlugins`].
/// Uses all entities with a [`Mesh3d`] component to generate navmeshes.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Mesh3dNavmeshPlugin;

impl Plugin for Mesh3dNavmeshPlugin {
    fn build(&self, app: &mut App) {
        app.set_navmesh_affector_backend(mesh3d_backend);
    }
}

fn mesh3d_backend(
    meshes: Res<Assets<Mesh>>,
    affectors: Query<(&GlobalTransform, &Mesh3d)>,
) -> Vec<(GlobalTransform, TriMesh)> {
    affectors
        .iter()
        .filter_map(|(transform, mesh)| {
            let transform = *transform;
            let mesh = meshes.get(mesh)?;
            let proxy_mesh = TriMesh::from_mesh(mesh)?;
            Some((transform, proxy_mesh))
        })
        .collect::<Vec<_>>()
}

/// Used to add [`TriMeshExt::from_mesh`] to [`TriMesh`].
trait TriMeshExt {
    /// Converts a [`Mesh`] into a [`TriMesh`].
    fn from_mesh(mesh: &Mesh) -> Option<TriMesh>;
}

impl TriMeshExt for TriMesh {
    fn from_mesh(mesh: &Mesh) -> Option<TriMesh> {
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            return None;
        }

        let mut trimesh = TriMesh::default();
        let position = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?;
        let float = position.as_float3()?;
        trimesh.vertices = float.iter().map(|v| Vec3A::from(*v)).collect();

        let indices: Vec<_> = mesh.indices()?.iter().collect();
        if !indices.len().is_multiple_of(3) {
            return None;
        }
        trimesh.indices = indices
            .chunks(3)
            .map(|indices| {
                UVec3::from_array([indices[0] as u32, indices[1] as u32, indices[2] as u32])
            })
            .collect();
        // TODO: accept vertex attributes for this?
        trimesh.area_types = vec![AreaType::NOT_WALKABLE; trimesh.indices.len()];
        Some(trimesh)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use bevy_render::{mesh::Indices, render_asset::RenderAssetUsages};
    use bevy_rerecast_transmission::SerializedMesh;

    use super::*;

    #[test]
    fn roundtrip_trimesh() {
        let mesh = star();
        let serialized_mesh = SerializedMesh::from_mesh(&mesh);
        let deserialized_mesh = serialized_mesh.into_mesh();
        let trimesh = TriMesh::from_mesh(&deserialized_mesh).unwrap();

        let expected_pos = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        assert_eq!(expected_pos.len(), trimesh.vertices.len(), "vertex len");

        let Indices::U32(expected_indices) = mesh.indices().unwrap() else {
            panic!("Expected U32 indices");
        };
        assert_eq!(
            expected_indices.len() / 3,
            trimesh.indices.len(),
            "index len"
        );

        for (expected_vert, vert) in expected_pos.iter().zip(trimesh.vertices.iter()) {
            assert_eq!(expected_vert, &vert.to_array());
        }

        for (expected_index, index) in expected_indices.chunks(3).zip(trimesh.indices.iter()) {
            assert_eq!(expected_index, index.to_array());
        }
    }

    /// Taken from <https://bevy.org/examples/2d-rendering/mesh2d-manual/>
    fn star() -> Mesh {
        let mut star = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

        let mut v_pos = vec![[0.0, 0.0, 0.0]];
        for i in 0..10 {
            let a = i as f32 * PI / 5.0;
            let r = (1 - i % 2) as f32 * 100.0 + 100.0;
            v_pos.push([r * a.sin(), r * a.cos(), 0.0]);
        }
        star.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

        let mut indices = vec![0, 1, 10];
        for i in 2..=10 {
            indices.extend_from_slice(&[0, i, i - 1]);
        }
        star.insert_indices(Indices::U32(indices));
        star
    }
}
