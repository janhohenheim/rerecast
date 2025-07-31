#![doc = include_str!("../../../readme.md")]

use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    ecs::entity_disabling::Disabled,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};

mod collider_to_trimesh;
use bevy_rerecast::{NavmeshAffector, editor_integration::RerecastAppExt as _};
use bevy_rerecast_transmission::SerializedMesh;

pub use rerecast;

use crate::collider_to_trimesh::ToTriMesh;

/// Everything you need to get started with the Navmesh plugin.
pub mod prelude {
    pub use crate::AvianRerecastPlugin;
}

/// The plugin of the crate. Will make all entities with both [`Collider`] and [`NavmeshAffector<Collider>`] available for navmesh generation.
#[non_exhaustive]
#[derive(Debug, Default)]
pub struct AvianRerecastPlugin {
    /// Settings for when [`NavmeshAffector<Collider>`] is inserted automatically.
    affector_settings: AvianNavmeshAffectorSettings,
}

/// The settings for when [`NavmeshAffector<Collider>`] is inserted automatically.
#[derive(Debug, Default)]
pub enum AvianNavmeshAffectorSettings {
    /// All entities with [`Collider`] belonging to a static [`RigidBody`] will have [`NavmeshAffector<Collider>`] inserted automatically.
    #[default]
    Static,
    /// [`NavmeshAffector<Collider>`] will not be inserted automatically. The user must manually insert it.
    Manual,
}

impl Plugin for AvianRerecastPlugin {
    fn build(&self, app: &mut App) {
        app.add_rasterizer(rasterize_colliders);
        match self.affector_settings {
            AvianNavmeshAffectorSettings::Static => {
                app.add_observer(insert_navmesh_affector_to_static_bodies);
            }
            AvianNavmeshAffectorSettings::Manual => {}
        }
    }
}

fn insert_navmesh_affector_to_static_bodies(
    trigger: Trigger<OnAdd, ColliderOf>,
    mut commands: Commands,
    collider_of: Query<&ColliderOf, Or<(With<Disabled>, Without<Disabled>)>>,
    bodies: Query<&RigidBody>,
) {
    let entity = trigger.target();
    let Ok(collider_of) = collider_of.get(entity) else {
        return;
    };

    let Ok(body) = bodies.get(collider_of.body) else {
        return;
    };

    if !body.is_static() {
        return;
    }

    commands
        .entity(entity)
        .insert(NavmeshAffector::<Collider>::default());
}

fn rasterize_colliders(
    colliders: Query<(&GlobalTransform, &Collider), With<NavmeshAffector<Collider>>>,
) -> Vec<(GlobalTransform, SerializedMesh)> {
    colliders
        .iter()
        .filter_map(|(transform, collider)| {
            let subdivisions = 10;
            let mesh = rasterize_collider(collider, subdivisions)?;
            Some((*transform, mesh))
        })
        .collect::<Vec<_>>()
}

fn rasterize_collider(collider: &Collider, subdivisions: u32) -> Option<SerializedMesh> {
    let trimesh = collider.to_trimesh(subdivisions)?;
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(
                trimesh.vertices.into_iter().map(|v| v.to_array()).collect(),
            ),
        )
        .with_inserted_indices(Indices::U32(
            trimesh
                .indices
                .into_iter()
                .flat_map(|i| i.to_array())
                .collect(),
        ));
    Some(SerializedMesh::from_mesh(&mesh))
}
