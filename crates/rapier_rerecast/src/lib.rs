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
    colliders: Query<(Entity, &Collider, &GlobalTransform), Without<ExcludeColliderFromNavmesh>>,
    bodies: Query<(
        AnyOf<(&RigidBody, &ChildOf)>,
        Has<ExcludeColliderFromNavmesh>,
    )>,
) -> TriMesh {
    colliders
        .iter()
        .filter_map(|(entity, collider, transform)| {
            if input
                .filter
                .as_ref()
                .is_some_and(|entities| !entities.contains(&entity))
            {
                return None;
            }

            let mut body_entity = entity;
            while let Ok(((body, child_of), is_excluded)) = bodies.get(body_entity) {
                if body.is_some_and(|body| *body != RigidBody::Fixed || is_excluded) {
                    return None;
                }
                if let Some(&ChildOf(parent)) = child_of {
                    body_entity = parent;
                    continue;
                }
                break;
            }

            let subdivisions = 16;
            let (_scale, rot, pos) = transform.to_scale_rotation_translation();
            collider.to_trimesh(pos, rot, subdivisions)
        })
        .fold(TriMesh::default(), |mut acc, t| {
            acc.extend(t);
            acc
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::Vec3A;
    use bevy_rerecast_core::{NavmeshBackend, NavmeshSettings};

    #[test]
    fn exclude_collider_from_navmesh() {
        use bevy_transform::TransformPlugin;

        let mut app = App::new();
        app.add_plugins((TransformPlugin, RapierBackendPlugin));

        let _included_body = app
            .world_mut()
            .spawn((
                Collider::cuboid(0.1, 0.1, 0.1),
                RigidBody::Fixed,
                Transform::from_xyz(0.0, 0.0, 10.0),
            ))
            .id();

        let _excluded_body = app
            .world_mut()
            .spawn((
                Collider::cuboid(0.1, 0.1, 0.1),
                RigidBody::Fixed,
                Transform::from_xyz(10.0, 0.0, 0.0),
                ExcludeColliderFromNavmesh,
            ))
            .id();

        let body_with_child = app
            .world_mut()
            .spawn((
                RigidBody::Fixed,
                Transform::from_xyz(0.0, 0.0, -10.0),
                ExcludeColliderFromNavmesh,
            ))
            .id();

        let _included_child = app
            .world_mut()
            .spawn((
                Collider::cuboid(0.1, 0.1, 0.1),
                Transform::from_xyz(-10.0, 0.0, 0.0),
                ChildOf(body_with_child),
            ))
            .id();

        app.update();
        app.update();

        let settings = NavmeshSettings::default();
        let backend_id = app.world().resource::<NavmeshBackend>().0;
        let trimesh: TriMesh = app
            .world_mut()
            .run_system_with(backend_id, settings.clone())
            .unwrap();

        let included_vertices = trimesh.vertices.len();
        assert!(included_vertices > 0, "The navmesh should have vertices");
        assert_eq!(
            included_vertices, 8,
            "Should have exactly 1 cubes worth of vertices (1 x 8)"
        );

        let has_included_body = trimesh
            .vertices
            .iter()
            .any(|v| v.distance(Vec3A::new(0.0, 0.0, 10.0)) <= 1.0);
        assert!(
            has_included_body,
            "Included body at (0, 0, 10) should be in navmesh"
        );

        let has_excluded_body = trimesh
            .vertices
            .iter()
            .any(|v| v.distance(Vec3A::new(10.0, 0.0, 0.0)) <= 1.0);
        assert!(
            !has_excluded_body,
            "Excluded body at (10, 0, 0) should NOT be in navmesh"
        );

        let has_excluded_child = trimesh
            .vertices
            .iter()
            .any(|v| v.distance(Vec3A::new(-10.0, 0.0, -10.0)) <= 1.0);
        assert!(
            !has_excluded_child,
            "Excluded child at (-10, 0, -10) should NOT be in navmesh"
        );
    }
}
