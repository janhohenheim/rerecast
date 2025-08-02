//! Types for visualizing navmeshes for debugging purposes.

use bevy_app::prelude::*;
use bevy_asset::{RenderAssetUsages, prelude::*};
use bevy_color::{Alpha as _, palettes::tailwind};
use bevy_ecs::{component::HookContext, prelude::*, world::DeferredWorld};
use bevy_gizmos::prelude::*;
use bevy_mesh::{Indices, Mesh, PrimitiveTopology};
use bevy_pbr::prelude::*;
use bevy_reflect::prelude::*;
use bevy_render::{prelude::*, view::RenderLayers};
use glam::vec3;
use rerecast::PolygonNavmesh;

use crate::Navmesh;

/// Plugin for visualizing navmeshes for debugging purposes.
/// After adding the plugin, spawn a [`DetailNavmeshGizmo`] or [`PolygonNavmeshGizmo`] to visualize a navmesh.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NavmeshDebugPlugin;

impl Plugin for NavmeshDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavmeshGizmoConfig>()
            .init_resource::<GizmoHandles>();
        app.register_type::<NavmeshGizmoConfig>()
            .register_type::<DetailNavmeshGizmo>()
            .register_type::<PolygonNavmeshGizmo>();
        app.add_systems(
            PreUpdate,
            (
                mark_gizmos_dirty_on_config_change,
                mark_gizmos_dirty_on_asset_change,
                update_dirty_polygon_gizmos,
                update_dirty_detail_gizmos,
            )
                .chain(),
        );
    }
}

fn mark_gizmos_dirty_on_config_change(
    mut commands: Commands,
    config: Res<NavmeshGizmoConfig>,
    mut last_config: Local<Option<NavmeshGizmoConfig>>,
    polygon_gizmos: Query<Entity, With<PolygonNavmeshGizmo>>,
    detail_gizmos: Query<Entity, With<DetailNavmeshGizmo>>,
) {
    if !config.is_changed() {
        return;
    }
    let Some(last_config) = last_config.as_mut() else {
        *last_config = Some(config.clone());
        // The first change will be skipped because that's triggered when the config is first initialized.
        // Since all gizmos are spawned as dirty, we don't need to mark them as dirty again.
        return;
    };

    if !cfg_eq(&last_config.polygon_navmesh, &config.polygon_navmesh) {
        for entity in polygon_gizmos.iter() {
            commands.entity(entity).insert(DirtyNavmeshGizmo);
        }
    }
    if !cfg_eq(&last_config.detail_navmesh, &config.detail_navmesh) {
        for entity in detail_gizmos.iter() {
            commands.entity(entity).insert(DirtyNavmeshGizmo);
        }
    }
    *last_config = config.clone();
}

fn mark_gizmos_dirty_on_asset_change(
    mut commands: Commands,
    mut asset_events: EventReader<AssetEvent<Navmesh>>,
    polygon_gizmos: Query<(Entity, &PolygonNavmeshGizmo)>,
    detail_gizmos: Query<(Entity, &DetailNavmeshGizmo)>,
) {
    for event in asset_events.read() {
        match event {
            AssetEvent::Added { id }
            | AssetEvent::LoadedWithDependencies { id }
            | AssetEvent::Modified { id } => {
                for (entity, current_id) in polygon_gizmos
                    .iter()
                    .map(|(entity, handle)| (entity, handle.0))
                    .chain(
                        detail_gizmos
                            .iter()
                            .map(|(entity, handle)| (entity, handle.0)),
                    )
                {
                    if current_id == *id {
                        commands.entity(entity).insert(DirtyNavmeshGizmo);
                    }
                }
            }
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                for (entity, current_id) in polygon_gizmos
                    .iter()
                    .map(|(entity, handle)| (entity, handle.0))
                    .chain(
                        detail_gizmos
                            .iter()
                            .map(|(entity, handle)| (entity, handle.0)),
                    )
                {
                    if current_id == *id {
                        commands.entity(entity).try_despawn();
                    }
                }
            }
        }
    }
}

fn cfg_eq(a: &GizmoConfig, b: &GizmoConfig) -> bool {
    a.enabled == b.enabled
        && a.line.width == b.line.width
        && a.line.perspective == b.line.perspective
        && a.line.style == b.line.style
        && a.line.joints == b.line.joints
        && a.depth_bias == b.depth_bias
        && a.render_layers == b.render_layers
}

/// Marker for gizmos that need to be updated.
#[derive(Component, Default)]
struct DirtyNavmeshGizmo;

/// Component that draws a [`DetailNavmesh`](rerecast::DetailNavmesh).
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[require(DirtyNavmeshGizmo, Visibility)]
#[component(on_add = init_detail_navmesh_gizmo)]
pub struct DetailNavmeshGizmo(pub AssetId<Navmesh>);

fn init_detail_navmesh_gizmo(mut world: DeferredWorld, ctx: HookContext) {
    let gizmo_handle = world
        .resource_mut::<Assets<GizmoAsset>>()
        .add(GizmoAsset::new());
    let material_handle = world.resource::<GizmoHandles>().detail_material.clone();
    let config = world
        .resource::<NavmeshGizmoConfig>()
        .detail_navmesh
        .clone();
    world.commands().entity(ctx.entity).insert((
        Gizmo {
            handle: gizmo_handle,
            line_config: config.line,
            depth_bias: config.depth_bias,
        },
        config.render_layers,
        MeshMaterial3d(material_handle),
    ));
}

fn update_dirty_polygon_gizmos(
    mut commands: Commands,
    mut gizmos: Query<
        (
            Entity,
            &mut Gizmo,
            &mut RenderLayers,
            &PolygonNavmeshGizmo,
            &mut Visibility,
        ),
        With<DirtyNavmeshGizmo>,
    >,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    navmeshes: Res<Assets<Navmesh>>,
    config: Res<NavmeshGizmoConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, mut gizmo_handle, mut layers, navmesh_handle, mut visibility) in gizmos.iter_mut()
    {
        let Some(gizmo) = gizmo_assets.get_mut(&gizmo_handle.handle) else {
            continue;
        };
        let config = config.polygon_navmesh.clone();
        if !config.enabled {
            gizmo.clear();
            commands.entity(entity).remove::<DirtyNavmeshGizmo>();
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some(navmesh) = navmeshes.get(navmesh_handle.0) else {
            continue;
        };

        let mesh = &navmesh.polygon;
        let nvp = mesh.max_vertices_per_polygon as usize;
        let origin = mesh.aabb.min;
        let to_local = vec3(mesh.cell_size, mesh.cell_height, mesh.cell_size);
        for i in 0..mesh.polygon_count() {
            let poly = &mesh.polygons[i * nvp..];
            let mut verts = poly[..nvp]
                .iter()
                .filter(|i| **i != PolygonNavmesh::NO_INDEX)
                .map(|i| {
                    let vert_local = mesh.vertices[*i as usize];

                    origin + vert_local.as_vec3() * to_local
                })
                .collect::<Vec<_>>();
            // Connect back to first vertex to finish the polygon
            verts.push(verts[0]);

            gizmo.linestrip(verts, tailwind::SKY_700);
        }

        let mut visual_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        let mut visual_verts = Vec::new();
        let mut visual_indices = Vec::new();

        for i in 0..mesh.polygon_count() {
            let poly = &mesh.polygons[i * nvp..];
            let a = origin + mesh.vertices[poly[0] as usize].as_vec3() * to_local;
            let a_idx = visual_verts.len() as u32;
            visual_verts.push(a);

            // Fan triangulation
            for val in poly[1..nvp].windows(2) {
                let b = val[0];
                let c = val[1];
                if b == PolygonNavmesh::NO_INDEX || c == PolygonNavmesh::NO_INDEX {
                    continue;
                }
                let b = origin + mesh.vertices[b as usize].as_vec3() * to_local;
                let c = origin + mesh.vertices[c as usize].as_vec3() * to_local;

                let b_vi = visual_verts.len() as u32;
                visual_verts.push(b);
                let c_vi = visual_verts.len() as u32;
                visual_verts.push(c);

                visual_indices.push(a_idx);
                visual_indices.push(b_vi);
                visual_indices.push(c_vi);
            }
        }
        visual_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, visual_verts);
        visual_mesh.insert_indices(Indices::U32(visual_indices));

        commands
            .entity(entity)
            .insert((Mesh3d(meshes.add(visual_mesh)),));

        gizmo_handle.line_config = config.line;
        gizmo_handle.depth_bias = config.depth_bias;
        *layers = config.render_layers;
        *visibility = Visibility::Inherited;
        commands.entity(entity).remove::<DirtyNavmeshGizmo>();
    }
}

fn update_dirty_detail_gizmos(
    mut commands: Commands,
    mut gizmos: Query<
        (
            Entity,
            &mut Gizmo,
            &mut RenderLayers,
            &DetailNavmeshGizmo,
            &mut Visibility,
        ),
        With<DirtyNavmeshGizmo>,
    >,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    navmeshes: Res<Assets<Navmesh>>,
    config: Res<NavmeshGizmoConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, mut gizmo_handle, mut layers, navmesh_handle, mut visibility) in gizmos.iter_mut()
    {
        let Some(gizmo) = gizmo_assets.get_mut(&gizmo_handle.handle) else {
            continue;
        };

        let config = config.detail_navmesh.clone();
        if !config.enabled {
            gizmo.clear();
            commands.entity(entity).remove::<DirtyNavmeshGizmo>();
            *visibility = Visibility::Hidden;
            continue;
        }
        let Some(navmesh) = navmeshes.get(navmesh_handle.0) else {
            continue;
        };

        let mesh = &navmesh.detail;

        for submesh in &mesh.meshes {
            let submesh_verts = &mesh.vertices[submesh.base_vertex_index as usize..]
                [..submesh.vertex_count as usize];
            let submesh_tris = &mesh.triangles[submesh.base_triangle_index as usize..]
                [..submesh.triangle_count as usize];
            for tri in submesh_tris {
                let mut verts = tri
                    .iter()
                    .map(|i| submesh_verts[*i as usize])
                    .collect::<Vec<_>>();
                // Connect back to first vertex to finish the polygon
                verts.push(verts[0]);

                gizmo.linestrip(verts, tailwind::GREEN_700);
            }
        }

        let mut visual_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
        let mut visual_verts = Vec::new();
        let mut visual_indices = Vec::new();

        for submesh in &mesh.meshes {
            let submesh_verts = &mesh.vertices[submesh.base_vertex_index as usize..]
                [..submesh.vertex_count as usize];

            let submesh_tris = &mesh.triangles[submesh.base_triangle_index as usize..]
                [..submesh.triangle_count as usize];
            for tri in submesh_tris.iter() {
                for &i in tri {
                    visual_indices.push(i as u32 + visual_verts.len() as u32);
                }
            }
            visual_verts.extend(submesh_verts.iter().copied());
        }
        visual_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, visual_verts);
        visual_mesh.insert_indices(Indices::U32(visual_indices));

        commands
            .entity(entity)
            .insert((Mesh3d(meshes.add(visual_mesh)),));

        gizmo_handle.line_config = config.line;
        gizmo_handle.depth_bias = config.depth_bias;
        *layers = config.render_layers;
        *visibility = Visibility::Inherited;
        commands.entity(entity).remove::<DirtyNavmeshGizmo>();
    }
}

/// Component that draws a [`PolygonNavmesh`].
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[require(DirtyNavmeshGizmo, Visibility)]
#[component(on_add = init_polygon_navmesh_gizmo)]
pub struct PolygonNavmeshGizmo(pub AssetId<Navmesh>);

fn init_polygon_navmesh_gizmo(mut world: DeferredWorld, ctx: HookContext) {
    let gizmo_handle = world
        .resource_mut::<Assets<GizmoAsset>>()
        .add(GizmoAsset::new());
    let material_handle = world.resource::<GizmoHandles>().polygon_material.clone();
    let config = world
        .resource::<NavmeshGizmoConfig>()
        .polygon_navmesh
        .clone();
    world.commands().entity(ctx.entity).insert((
        Gizmo {
            handle: gizmo_handle,
            line_config: config.line,
            depth_bias: config.depth_bias,
        },
        config.render_layers,
        MeshMaterial3d(material_handle),
    ));
}

#[derive(Resource)]
struct GizmoHandles {
    polygon_material: Handle<StandardMaterial>,
    detail_material: Handle<StandardMaterial>,
}

impl FromWorld for GizmoHandles {
    fn from_world(world: &mut World) -> Self {
        Self {
            polygon_material: world.resource_mut::<Assets<StandardMaterial>>().add(
                StandardMaterial {
                    base_color: tailwind::BLUE_600.with_alpha(0.7).into(),
                    unlit: true,
                    double_sided: true,
                    alpha_mode: AlphaMode::AlphaToCoverage,
                    ..Default::default()
                },
            ),
            detail_material: world.resource_mut::<Assets<StandardMaterial>>().add(
                StandardMaterial {
                    base_color: tailwind::EMERALD_200.with_alpha(0.7).into(),
                    unlit: true,
                    double_sided: true,
                    alpha_mode: AlphaMode::AlphaToCoverage,
                    ..Default::default()
                },
            ),
        }
    }
}

/// Global configuration for all navmesh gizmos.
#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct NavmeshGizmoConfig {
    /// Configuration for all [`PolygonNavmeshGizmo`]s.
    pub polygon_navmesh: GizmoConfig,
    /// Configuration for all [`DetailNavmeshGizmo`]s.
    pub detail_navmesh: GizmoConfig,
}

impl Default for NavmeshGizmoConfig {
    fn default() -> Self {
        Self {
            polygon_navmesh: GizmoConfig {
                enabled: false,
                line: GizmoLineConfig {
                    perspective: true,
                    width: 20.0,
                    ..Default::default()
                },
                depth_bias: -0.001,
                ..Default::default()
            },
            detail_navmesh: GizmoConfig {
                enabled: true,
                line: GizmoLineConfig {
                    perspective: true,
                    width: 20.0,
                    ..Default::default()
                },
                depth_bias: -0.001,
                ..Default::default()
            },
        }
    }
}
