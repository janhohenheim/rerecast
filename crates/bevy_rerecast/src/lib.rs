#![doc = include_str!("../readme.md")]

use std::marker::PhantomData;

use bevy_app::{PluginGroupBuilder, prelude::*};
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

#[cfg(feature = "from_mesh")]
use bevy_mesh::PrimitiveTopology;
#[cfg(feature = "from_mesh")]
use bevy_render::prelude::*;
#[cfg(feature = "from_mesh")]
use glam::{UVec3, Vec3A};

#[cfg(feature = "editor_integration")]
pub mod editor_integration;

pub use rerecast;
#[cfg(feature = "from_mesh")]
use rerecast::{AreaType, TriMesh};

/// Everything you need to get started with the NavMesh plugin.
pub mod prelude {
    #[cfg(feature = "from_mesh")]
    pub use crate::TriMeshExt as _;
    pub use crate::{NavmeshAffector, NavmeshPlugins};
}

/// The plugin group of the crate.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NavmeshPlugins;

impl PluginGroup for NavmeshPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>().add(RerecastPlugin::default());
        #[cfg(feature = "editor_integration")]
        let builder = builder.add(editor_integration::RerecastEditorIntegrationPlugin::default());
        builder
    }
}

/// The plugin of the crate.
#[non_exhaustive]
#[derive(Default)]
pub struct RerecastPlugin;

impl Plugin for RerecastPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "from_mesh")]
        app.register_type::<NavmeshAffector<Mesh3d>>();
    }
}

/// Component used to mark [`Mesh`]es as navmesh affectors.
/// Only meshes with this component will be considered when building the navmesh.
#[derive(Debug, Component, Reflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
#[reflect(Component)]
pub struct NavmeshAffector<T> {
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T> Default for NavmeshAffector<T> {
    fn default() -> Self {
        Self { _pd: PhantomData }
    }
}

#[cfg(feature = "from_mesh")]
/// Used to add [`TriMeshExt::from_mesh`] to [`TriMesh`].
pub trait TriMeshExt {
    /// Converts a [`Mesh`] into a [`TriMesh`].
    fn from_mesh(mesh: &Mesh) -> Option<TriMesh>;
}

#[cfg(feature = "from_mesh")]
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
