//! Utilities for generating navmeshes at runtime.

use anyhow::{Context as _, anyhow};
use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_platform::collections::HashMap;
use bevy_tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use bevy_transform::{TransformSystem, components::GlobalTransform};
use glam::{U16Vec3, Vec3, Vec3A};
use rerecast::{Aabb3d, DetailNavmesh, HeightfieldBuilder, TriMesh};

use crate::{Navmesh, NavmeshAffectorBackend, NavmeshSettings};

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
    task_queue: ResMut<'w, NavmeshTaskQueue>,
}

impl<'w> NavmeshGenerator<'w> {
    /// Queue a navmesh generation task.
    /// When you call this method, a new navmesh will be generated asynchronously.
    /// Calling it multiple times will queue multiple navmeshes to be generated.
    /// Affectors existing this frame at [`PostUpdate`] will be used to generate the navmesh.
    pub fn generate(&mut self, settings: NavmeshSettings) -> Handle<Navmesh> {
        let handle = self.navmeshes.reserve_handle();
        self.queue.insert(handle.id(), settings);
        handle
    }

    /// Queue a navmesh regeneration task.
    /// When you call this method, an existing navmesh will be regenerated asynchronously.
    /// Calling it multiple times will have no effect until the regeneration is complete.
    /// Affectors existing this frame at [`PostUpdate`] will be used to generate the navmesh.
    ///
    /// Returns `true` if the regeneration was queued, `false` if it was already queued.
    pub fn regenerate(
        &mut self,
        id: impl Into<AssetId<Navmesh>>,
        settings: NavmeshSettings,
    ) -> bool {
        let id = id.into();
        if self
            .queue
            .keys()
            .chain(self.task_queue.keys())
            .any(|queued_id| queued_id == &id)
        {
            return false;
        }
        self.queue.insert(id, settings);
        true
    }
}

#[derive(Debug, Resource, Default, Deref, DerefMut)]
struct NavmeshQueue(HashMap<AssetId<Navmesh>, NavmeshSettings>);

#[derive(Resource, Default, Deref, DerefMut)]
struct NavmeshTaskQueue(HashMap<AssetId<Navmesh>, Task<Result<Navmesh>>>);

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
    for (handle, input) in queue {
        let Some(backend) = world.get_resource::<NavmeshAffectorBackend>() else {
            tracing::error!("Cannot generate navmesh: No backend available");
            return;
        };
        let affectors = match world.run_system_with(backend.0, input.clone()) {
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
        let task = thread_pool.spawn(generate_navmesh(affectors.clone(), input));
        tasks_queue.insert(handle, task);
    }
}

fn poll_tasks(
    mut commands: Commands,
    mut tasks: ResMut<NavmeshTaskQueue>,
    mut navmeshes: ResMut<Assets<Navmesh>>,
) {
    let mut removed_ids = Vec::new();
    for (id, task) in tasks.iter_mut() {
        let Some(navmesh) = future::block_on(future::poll_once(task)) else {
            continue;
        };
        removed_ids.push(id.clone());
        let navmesh = match navmesh {
            Ok(navmesh) => navmesh,
            Err(err) => {
                tracing::error!("Failed to generate navmesh: {err}");
                continue;
            }
        };
        // Process the generated navmesh
        navmeshes.insert(id.clone(), navmesh);
    }
    for id in removed_ids {
        if let Some(_task) = tasks.remove(&id) {
            commands.trigger(NavmeshReady(id));
        }
    }
}

/// Triggered when a navmesh created by the [`NavmeshGenerator`] is ready.
#[derive(Debug, Event, Deref, DerefMut)]
pub struct NavmeshReady(pub AssetId<Navmesh>);

async fn generate_navmesh(
    affectors: Vec<(GlobalTransform, TriMesh)>,
    settings: NavmeshSettings,
) -> Result<Navmesh> {
    let mut trimesh = TriMesh::default();
    for (transform, mut current_trimesh) in affectors {
        let transform = transform.compute_transform();
        for vertex in &mut current_trimesh.vertices {
            *vertex = transform.transform_point(Vec3::from(*vertex)).into();
        }
        trimesh.extend(current_trimesh);
    }
    let up = settings.up;
    match up {
        Vec3::Y => {
            // already Bevy's coordinate system
        }
        Vec3::Z => {
            for vertex in &mut trimesh.vertices {
                *vertex = Vec3A::new(vertex.y, vertex.z, vertex.x);
            }
        }
        Vec3::X => {
            for vertex in &mut trimesh.vertices {
                *vertex = Vec3A::new(vertex.z, vertex.x, vertex.y);
            }
        }
        _ => {
            return Err(BevyError::from(anyhow!(
                "Unsupported up direction. Expected one of Vec3::Y, Vec3::Z, or Vec3X, but got {up}"
            )));
        }
    }

    let mut config_builder = settings.clone().into_rerecast_config();
    let config = {
        if config_builder.aabb == Aabb3d::default() {
            config_builder.aabb = trimesh
                .compute_aabb()
                .context("Failed to compute AABB: trimesh is empty")?;
        }
        let min = &mut config_builder.aabb.min;
        let max = &mut config_builder.aabb.max;
        match up {
            Vec3::Y => {
                // already Bevy's coordinate system
            }
            Vec3::Z => {
                *min = Vec3::new(min.y, min.z, min.x);
                *max = Vec3::new(max.y, max.z, max.x);
            }
            Vec3::X => {
                *min = Vec3::new(min.z, min.x, min.y);
                *max = Vec3::new(max.z, max.x, max.y);
            }
            _ => {
                return Err(BevyError::from(anyhow!(
                    "Unsupported up direction. Expected one of Vec3::Y, Vec3::Z, or Vec3X, but got {up}"
                )));
            }
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

    let mut navmesh = Navmesh {
        polygon: poly_mesh,
        detail: detail_mesh,
        settings,
    };
    let min = &mut navmesh.polygon.aabb.min;
    let max = &mut navmesh.polygon.aabb.max;
    match up {
        Vec3::Y => {
            // already Bevy's coordinate system
        }
        Vec3::Z => {
            for vertex in &mut navmesh.polygon.vertices {
                *vertex = U16Vec3::new(vertex.z, vertex.x, vertex.y);
            }
            for vertex in &mut navmesh.detail.vertices {
                *vertex = Vec3::new(vertex.z, vertex.x, vertex.y);
            }
            *min = Vec3::new(min.z, min.x, min.y);
            *max = Vec3::new(max.z, max.x, max.y);
        }
        Vec3::X => {
            for vertex in &mut navmesh.polygon.vertices {
                *vertex = U16Vec3::new(vertex.y, vertex.z, vertex.x);
            }
            for vertex in &mut navmesh.detail.vertices {
                *vertex = Vec3::new(vertex.y, vertex.z, vertex.x);
            }
            *min = Vec3::new(min.y, min.z, min.x);
            *max = Vec3::new(max.y, max.z, max.x);
        }
        _ => {
            return Err(BevyError::from(anyhow!(
                "Unsupported up direction. Expected one of Vec3::Y, Vec3::Z, or Vec3X, but got {up}"
            )));
        }
    }

    Ok(navmesh)
}
