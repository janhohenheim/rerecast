#![doc = include_str!("../../../readme.md")]

use avian3d::prelude::*;
use bevy::prelude::*;

mod collider_to_trimesh;
use bevy_rerecast::NavmeshApp as _;

pub use rerecast;
use rerecast::TriMesh;

use crate::collider_to_trimesh::ToTriMesh;

/// Everything you need to get started with the Navmesh plugin.
pub mod prelude {
    pub use crate::AvianRerecastPlugin;
}

/// The plugin of the crate. Will make all entities with [`Collider`] a collider belonging to a static [`RigidBody`] available for navmesh generation.
#[non_exhaustive]
#[derive(Debug, Default)]
pub struct AvianRerecastPlugin;

impl Plugin for AvianRerecastPlugin {
    fn build(&self, app: &mut App) {
        app.set_navmesh_affector_backend(collider_backend);
    }
}

fn collider_backend(
    colliders: Query<(&GlobalTransform, &Collider, &ColliderOf)>,
    bodies: Query<&RigidBody>,
) -> Vec<(GlobalTransform, TriMesh)> {
    colliders
        .iter()
        .filter_map(|(transform, collider, collider_of)| {
            let body = bodies.get(collider_of.body).ok()?;
            if !body.is_static() {
                return None;
            }
            let subdivisions = 10;
            let mesh = collider.to_trimesh(subdivisions)?;
            Some((*transform, mesh))
        })
        .collect::<Vec<_>>()
}
