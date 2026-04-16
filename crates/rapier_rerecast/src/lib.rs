//! Backend for using [`bevy_rapier3d`](https://docs.rs/bevy_rapier3d) with [`bevy_rerecast`](https://docs.rs/bevy_rerecast).

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_reflect::prelude::*;
use bevy_rerecast_core::{NavmeshApp as _, NavmeshSettings, rerecast::TriMesh};
use bevy_transform::prelude::*;

mod collider_to_trimesh;
pub use crate::collider_to_trimesh::ColliderToTriMesh;

/// Everything you need to get started with the Navmesh plugin.
pub mod prelude {
    pub use crate::{ExcludeColliderFromNavmesh, RapierBackendPlugin};
}

/// The plugin of the crate. Will make all entities with [`Collider`] a collider belonging to a static [`RigidBody`] available for navmesh generation.
#[non_exhaustive]
#[derive(Debug, Default)]
pub struct RapierBackendPlugin;

impl Plugin for RapierBackendPlugin {
    fn build(&self, app: &mut App) {
        app.set_navmesh_backend(collider_backend);
    }
}

/// Component to opt-out a [`Collider`] or [`RigidBody`] from navmesh generation when using [`RapierBackendPlugin`].
/// If that backend is not used, this component has no effect.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct ExcludeColliderFromNavmesh;

fn collider_backend(
    input: In<NavmeshSettings>,
    colliders: Query<
        (Entity, &Collider, &GlobalTransform, &RigidBody),
        Without<ExcludeColliderFromNavmesh>,
    >,
) -> TriMesh {
    colliders
        .iter()
        .filter_map(|(entity, collider, transform, body)| {
            if input
                .filter
                .as_ref()
                .is_some_and(|entities| !entities.contains(&entity))
            {
                return None;
            }
            if *body != RigidBody::Fixed {
                return None;
            }
            let subdivisions = 10;
            let pos = transform.translation();
            let rot = transform.rotation();
            collider.to_trimesh(pos, rot, subdivisions)
        })
        .fold(TriMesh::default(), |mut acc, t| {
            acc.extend(t);
            acc
        })
}
