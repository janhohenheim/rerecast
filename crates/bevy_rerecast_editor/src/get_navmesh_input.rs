use anyhow::{Result, anyhow};
use bevy::{
    asset::RenderAssetUsages,
    ecs::world::WorldId,
    mesh::{Indices, PrimitiveTopology},
    platform::collections::HashMap,
    prelude::*,
    remote::BrpRequest,
    tasks::{IoTaskPool, Task},
};
use bevy_rerecast::editor_integration::{
    brp::{
        BRP_GENERATE_EDITOR_INPUT, BRP_POLL_EDITOR_INPUT, GenerateEditorInputParams,
        GenerateEditorInputResponse, PollEditorInputParams, PollEditorInputResponse,
    },
    transmission::deserialize,
};
use bevy_ui_text_input::TextInputContents;

use crate::{
    backend::{GlobalNavmeshSettings, NavmeshHandle, NavmeshObstacles},
    ui::ConnectionInput,
    visualization::{ObstacleGizmo, VisualMesh},
};
use bevy_malek_async::{WorldIdRes, async_access};

pub(super) fn plugin(app: &mut App) {
    app.add_observer(on_get_navmesh_input);
}

#[derive(Event)]
pub(crate) struct GetNavmeshInput;

fn on_get_navmesh_input(
    _: On<GetNavmeshInput>,
    mut task: Local<Option<Task<()>>>,
    world_id: Res<WorldIdRes>,
) {
    let world_id = world_id.0.clone();
    if task.as_ref().is_some_and(|task| task.is_finished()) {
        task.take();
    }
    match task.as_ref() {
        None => {
            task.replace(IoTaskPool::get().spawn(async move {
                if let Err(e) = navmesh_pipeline(world_id).await {
                    error!("navmesh pipeline failed: {e:?}");
                }
            }));
        }
        Some(_) => {
            error!("a navmesh task is already running");
        }
    }
}

async fn navmesh_pipeline(world_id: WorldId) -> Result<()> {
    let (settings, url): (serde_json::Value, String) =
        async_access::<
            (
                Res<GlobalNavmeshSettings>,
                Single<&TextInputContents, With<ConnectionInput>>,
            ),
            _,
            _,
        >(world_id, |(settings, connection_input)| {
            Ok::<_, anyhow::Error>((
                serde_json::to_value(GenerateEditorInputParams {
                    backend_input: settings.0.clone(),
                })?,
                connection_input.get().to_string(),
            ))
        })
        .await?;

    let generate_id = {
        let req = BrpRequest {
            jsonrpc: "2.0".into(),
            method: BRP_GENERATE_EDITOR_INPUT.into(),
            id: None,
            params: Some(settings),
        };
        let resp = ehttp::fetch_async(ehttp::Request::json(url, &req)?)
            .await
            .map_err(|s| anyhow!("{s}"))?;

        let mut v: serde_json::Value = resp.json()?;
        let val = v.get_mut("result").map(|r| r.take()).ok_or_else(|| {
            anyhow!(
                "BRP error: {}",
                v.get("error").unwrap_or(&serde_json::Value::Null)
            )
        })?;
        let GenerateEditorInputResponse { id, .. } = serde_json::from_value(val)?;
        id
    };

    let response: PollEditorInputResponse = {
        let params = serde_json::to_value(PollEditorInputParams { id: generate_id })?;
        let req = BrpRequest {
            jsonrpc: "2.0".into(),
            method: BRP_POLL_EDITOR_INPUT.into(),
            id: None,
            params: Some(params),
        };
        let resp = ehttp::fetch_async(ehttp::Request::json("http://127.0.0.1:15702/", &req)?)
            .await
            .map_err(|s| anyhow!("{s}"))?;

        let mut v: serde_json::Value = resp.json()?;
        let val = v.get_mut("result").map(|r| r.take()).ok_or_else(|| {
            anyhow!(
                "BRP error: {}",
                v.get("error").unwrap_or(&serde_json::Value::Null)
            )
        })?;
        deserialize(&val)?
    };

    async_access::<
        (
            Commands,
            ResMut<Assets<Mesh>>,
            ResMut<Assets<StandardMaterial>>,
            ResMut<Assets<Image>>,
            Query<Entity, (With<Mesh3d>, With<VisualMesh>)>,
            Query<&Gizmo>,
            ResMut<Assets<GizmoAsset>>,
            ResMut<NavmeshHandle>,
        ),
        _,
        _,
    >(
        world_id,
        move |(
            mut commands,
            mut meshes,
            mut materials,
            mut images,
            mesh_handles,
            gizmo_handles,
            mut gizmos,
            mut navmesh_handle,
        )| {
            // Clear existing scene bits.
            for e in mesh_handles.iter() {
                commands.entity(e).despawn();
            }
            for gizmo in gizmo_handles.iter() {
                if let Some(g) = gizmos.get_mut(&gizmo.handle) {
                    g.clear();
                }
            }

            // Obstacles preview mesh.
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    response.obstacles.vertices.clone(),
                )
                .with_inserted_indices(Indices::U32(
                    response
                        .obstacles
                        .indices
                        .iter()
                        .flat_map(|tri| tri.to_array())
                        .collect(),
                ))
                .with_computed_normals();

            commands.spawn((
                Transform::default(),
                Mesh3d(meshes.add(mesh)),
                Visibility::Hidden,
                ObstacleGizmo,
                Gizmo {
                    handle: gizmos.add(GizmoAsset::new()),
                    line_config: GizmoLineConfig {
                        perspective: true,
                        width: 15.0,
                        joints: GizmoLineJoint::Bevel,
                        ..default()
                    },
                    depth_bias: -0.005,
                },
            ));
            commands.insert_resource(NavmeshObstacles(response.obstacles.clone()));

            // Visual meshes + materials (with per-index caches).
            let mut image_indices: HashMap<u32, Handle<Image>> = HashMap::default();
            let mut material_indices: HashMap<u32, Handle<StandardMaterial>> = HashMap::default();
            let mut mesh_indices: HashMap<u32, Handle<Mesh>> = HashMap::default();
            let fallback_material = materials.add(Color::WHITE);

            for visual in response.visual_meshes {
                let mesh_handle = mesh_indices
                    .entry(visual.mesh)
                    .or_insert_with(|| {
                        let mut m = response.meshes[visual.mesh as usize].clone().into_mesh();
                        // Avoid skinned attributes without SkinnedMesh
                        m.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
                        m.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
                        meshes.add(m)
                    })
                    .clone();

                let material_handle = if let Some(idx) = visual.material {
                    material_indices
                        .entry(idx)
                        .or_insert_with(|| {
                            let mat = response.materials[idx as usize]
                                .clone()
                                .into_standard_material(
                                    &mut image_indices,
                                    &mut images,
                                    &response.images,
                                );
                            materials.add(mat)
                        })
                        .clone()
                } else {
                    fallback_material.clone()
                };

                commands.spawn((
                    visual.transform.compute_transform(),
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    VisualMesh,
                ));
            }

            // Clear previous navmesh handle
            navmesh_handle.0 = Default::default();

            Ok::<_, anyhow::Error>(())
        },
    )
    .await?;

    Ok(())
}
