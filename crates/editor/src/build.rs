use avian_navmesh::{
    heightfield::{HeightfieldBuilder, HeightfieldBuilderError},
    trimesh::{ToTrimesh as _, TrimeshedCollider},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use thiserror::Error;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(build_navmesh);
}

#[derive(Event)]
pub(crate) struct BuildNavmesh;

#[derive(Resource)]
pub(crate) struct BuildNavmeshConfig {
    pub(crate) subdivisions: u32,
    pub(crate) cell_size: f32,
    pub(crate) cell_height: f32,
}

impl Default for BuildNavmeshConfig {
    fn default() -> Self {
        Self {
            subdivisions: 12,
            cell_size: 1.0,
            cell_height: 1.0,
        }
    }
}

fn build_navmesh(
    _trigger: Trigger<BuildNavmesh>,
    colliders: Query<&Collider>,
    config: Res<BuildNavmeshConfig>,
) -> Result {
    let mut trimesh = TrimeshedCollider::default();
    for collider in colliders.iter() {
        let Some(collider) = collider.to_trimesh(config.subdivisions) else {
            warn!("Failed to convert collider to trimesh. Skipping.");
            continue;
        };
        trimesh.extend(collider);
    }

    let Some(aabb) = trimesh.compute_aabb() else {
        return Err(BuildNavmeshBuildError::TrimeshEmpty.into());
    };

    let mut heightfield = HeightfieldBuilder {
        aabb,
        cell_size: config.cell_size,
        cell_height: config.cell_height,
    }
    .build()?;
    trimesh.rasterize(&mut heightfield);
    Ok(())
}

#[derive(Error, Debug)]
pub(crate) enum BuildNavmeshBuildError {
    #[error("Failed to build heightfield: {0}")]
    HeightfieldBuilderError(#[from] HeightfieldBuilderError),
    #[error("Trimesh is empty")]
    TrimeshEmpty,
}
