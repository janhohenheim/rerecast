use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_mesh::{Mesh, PrimitiveTopology};
use bevy_render::prelude::*;
use bevy_transform::components::GlobalTransform;
use glam::{UVec3, Vec3A};
use rerecast::{AreaType, TriMesh};

use crate::NavmeshApp as _;

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

/// Used to add [`TriMeshFromBevyMesh::from_mesh`] to [`TriMesh`].
pub trait TriMeshFromBevyMesh {
    /// Converts a [`Mesh`] into a [`TriMesh`].
    fn from_mesh(mesh: &Mesh) -> Option<TriMesh>;
}

impl TriMeshFromBevyMesh for TriMesh {
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
