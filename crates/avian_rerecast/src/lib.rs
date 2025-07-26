#![doc = include_str!("../../../readme.md")]

use avian3d::prelude::*;
use bevy::prelude::*;

mod collider_to_trimesh;
use bevy_rerecast::{NavmeshAffector, editor_integration::RerecastAppExt as _};
use bevy_rerecast_transmission::{
    SerializedIndices, SerializedMesh, SerializedMeshVertexAttributeId,
    SerializedPrimitiveTopology, SerializedVertexAttributeValues,
};

pub use rerecast;

use crate::collider_to_trimesh::ToTriMesh;

/// Everything you need to get started with the NavMesh plugin.
pub mod prelude {
    pub use crate::AvianRerecastPlugin;
}

/// The plugin of the crate.
#[non_exhaustive]
#[derive(Default)]
pub struct AvianRerecastPlugin;

impl Plugin for AvianRerecastPlugin {
    fn build(&self, app: &mut App) {
        app.add_rasterizer(rasterize_colliders);
    }
}

fn rasterize_colliders(
    colliders: Query<(&GlobalTransform, &Collider), With<NavmeshAffector<Collider>>>,
) -> Vec<(GlobalTransform, SerializedMesh)> {
    colliders
        .iter()
        .filter_map(|(transform, collider)| {
            let trimesh = collider.to_trimesh(10)?;
            let attr_id =
                SerializedMeshVertexAttributeId::try_from(Mesh::ATTRIBUTE_POSITION.id).unwrap();
            let attr_values = SerializedVertexAttributeValues::Float32x3(
                trimesh.vertices.into_iter().map(|v| v.to_array()).collect(),
            );
            let indices = SerializedIndices::U32(
                trimesh
                    .indices
                    .into_iter()
                    .flat_map(|i| i.to_array())
                    .collect(),
            );
            let serialized_mesh = SerializedMesh {
                primitive_topology: SerializedPrimitiveTopology::TriangleList,
                attributes: vec![(attr_id, attr_values)],
                indices: Some(indices),
            };
            Some((*transform, serialized_mesh))
        })
        .collect::<Vec<_>>()
}
