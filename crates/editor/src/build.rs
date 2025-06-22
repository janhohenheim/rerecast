use avian_navmesh::{
    heightfield::HeightfieldBuilder,
    trimesh::{ToTrimesh as _, TrimeshedCollider},
};
use avian3d::prelude::*;
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(build_navmesh);
}

#[derive(Event)]
pub(crate) struct BuildNavmesh;

#[derive(Resource)]
pub(crate) struct BuildNavmeshConfig {
    pub(crate) subdivisions: u32,
}

impl Default for BuildNavmeshConfig {
    fn default() -> Self {
        Self { subdivisions: 12 }
    }
}

fn build_navmesh(
    _trigger: Trigger<BuildNavmesh>,
    mut commands: Commands,
    colliders: Query<&Collider>,
    config: Res<BuildNavmeshConfig>,
) {
    let mut trimesh = TrimeshedCollider::default();
    for collider in colliders.iter() {
        let Some(collider) = collider.to_trimesh(config.subdivisions) else {
            warn!("Failed to trimesh collider");
            continue;
        };
        trimesh.extend(collider);
    }

    let mut heightfield = HeightfieldBuilder {
        width: todo!(),
        height: todo!(),
        aabb: todo!(),
        cell_size: todo!(),
        cell_height: todo!(),
    }
    .build();
}
