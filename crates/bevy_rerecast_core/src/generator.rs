//! Utilities for generating navmeshes at runtime.

use std::marker::PhantomData;

use anyhow::Context as _;
use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use bevy_transform::{TransformSystem, components::GlobalTransform};
use glam::Vec3;
use rerecast::{Aabb3d, DetailNavmesh, HeightfieldBuilder, NavmeshConfigBuilder, TriMesh};

use crate::{Navmesh, NavmeshAffectorBackend};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<NavmeshQueue>();
    app.init_resource::<NavmeshTaskQueue>();
    app.add_systems(
        PostUpdate,
        (drain_queue_into_tasks, poll_tasks)
            .chain()
            .after(TransformSystem::TransformPropagate),
    );
}

/// System parameter for generating navmeshes.
#[derive(SystemParam)]
pub struct NavmeshGenerator<'w> {
    #[system_param(
        validation_message = "Failed to find `Assets<Navmesh>`. Did you forget to add `NavmeshPlugins` to your app?"
    )]
    navmeshes: Res<'w, Assets<Navmesh>>,
    queue: ResMut<'w, NavmeshQueue>,
}

impl<'w> NavmeshGenerator<'w> {
    /// Queue a navmesh generation task.
    /// When you call this method, a new navmesh will be generated asynchronously.
    /// Calling it multiple times will queue multiple navmeshes to be generated.
    /// Affectors existing this frame at [`PostUpdate`] will be used to generate the navmesh.
    ///
    /// If [`NavmeshConfigBuilder::aabb`] is left empty, the navmesh will be generated for the entire world.
    /// Otherwise, the navmesh will be generated for the specified area.
    pub fn generate(&mut self, config: NavmeshConfigBuilder) -> Handle<Navmesh> {
        let handle = self.navmeshes.reserve_handle();
        self.queue.push((handle.clone(), config));
        handle
    }
}

#[derive(Debug, Resource, Default, Deref, DerefMut)]
struct NavmeshQueue(Vec<(Handle<Navmesh>, NavmeshConfigBuilder)>);

#[derive(Resource, Default, Deref, DerefMut)]
struct NavmeshTaskQueue(Vec<(Handle<Navmesh>, Task<Result<Navmesh>>)>);

fn drain_queue_into_tasks(world: &mut World) {
    let queue = {
        let Some(mut queue) = world.get_resource_mut::<NavmeshQueue>() else {
            tracing::error!(
                "Cannot generate navmesh: No queue available. Please submit a bug report"
            );
            return;
        };
        std::mem::take(&mut queue.0)
    };
    if queue.is_empty() {
        return;
    }
    let Some(backend) = world.get_resource::<NavmeshAffectorBackend>() else {
        tracing::error!("Cannot generate navmesh: No backend available");
        return;
    };
    let affectors = match world.run_system(backend.0) {
        Ok(affectors) => affectors,
        Err(err) => {
            tracing::error!("Cannot generate navmesh: Backend error: {err}");
            return;
        }
    };
    let Some(mut tasks_queue) = world.get_resource_mut::<NavmeshTaskQueue>() else {
        tracing::error!(
            "Cannot generate navmesh: No task queue available. Please submit a bug report"
        );
        return;
    };
    let thread_pool = AsyncComputeTaskPool::get();
    for (handle, config) in queue {
        let task = thread_pool.spawn(generate_navmesh(affectors.clone(), config));
        tasks_queue.push((handle, task));
    }
}

fn poll_tasks(mut tasks: ResMut<NavmeshTaskQueue>, mut navmeshes: ResMut<Assets<Navmesh>>) {
    let mut removed_indices = Vec::new();
    for (index, (handle, task)) in tasks.iter_mut().enumerate() {
        let Some(navmesh) = future::block_on(future::poll_once(task)) else {
            continue;
        };
        removed_indices.push(index);
        let navmesh = match navmesh {
            Ok(navmesh) => navmesh,
            Err(err) => {
                tracing::error!("Failed to generate navmesh: {err}");
                continue;
            }
        };
        // Process the generated navmesh
        navmeshes.insert(handle, navmesh);
    }
    for index in removed_indices {
        let _completed_task = tasks.swap_remove(index);
    }
}

async fn generate_navmesh(
    affectors: Vec<(GlobalTransform, TriMesh)>,
    config_builder: NavmeshConfigBuilder,
) -> Result<Navmesh> {
    let mut trimesh = TriMesh::default();
    for (transform, mut current_trimesh) in affectors {
        let transform = transform.compute_transform();
        for vertex in &mut current_trimesh.vertices {
            *vertex = transform.transform_point(Vec3::from(*vertex)).into();
        }
        trimesh.extend(current_trimesh);
    }
    let config = {
        let mut config_builder = config_builder.clone();

        if config_builder.aabb == Aabb3d::default() {
            config_builder.aabb = trimesh
                .compute_aabb()
                .context("Failed to compute AABB: trimesh is empty")?;
        }
        config_builder.build()
    };

    trimesh.mark_walkable_triangles(config.walkable_slope_angle);

    let mut heightfield = HeightfieldBuilder {
        aabb: config.aabb,
        cell_size: config.cell_size,
        cell_height: config.cell_height,
    }
    .build()?;

    heightfield.rasterize_triangles(&trimesh, config.walkable_climb)?;

    // Once all geometry is rasterized, we do initial pass of filtering to
    // remove unwanted overhangs caused by the conservative rasterization
    // as well as filter spans where the character cannot possibly stand.
    heightfield.filter_low_hanging_walkable_obstacles(config.walkable_climb);
    heightfield.filter_ledge_spans(config.walkable_height, config.walkable_climb);
    heightfield.filter_walkable_low_height_spans(config.walkable_height);

    let mut compact_heightfield =
        heightfield.into_compact(config.walkable_height, config.walkable_climb)?;

    compact_heightfield.erode_walkable_area(config.walkable_radius);

    for volume in &config.area_volumes {
        compact_heightfield.mark_convex_poly_area(volume);
    }

    compact_heightfield.build_distance_field();

    compact_heightfield.build_regions(
        config.border_size,
        config.min_region_area,
        config.merge_region_area,
    )?;

    let contours = compact_heightfield.build_contours(
        config.max_simplification_error,
        config.max_edge_len,
        config.contour_flags,
    );

    let poly_mesh = contours.into_polygon_mesh(config.max_vertices_per_polygon)?;

    let detail_mesh = DetailNavmesh::new(
        &poly_mesh,
        &compact_heightfield,
        config.detail_sample_dist,
        config.detail_sample_max_error,
    )?;
    Ok(Navmesh {
        polygon: poly_mesh,
        detail: detail_mesh,
        config: config_builder,
    })
}
