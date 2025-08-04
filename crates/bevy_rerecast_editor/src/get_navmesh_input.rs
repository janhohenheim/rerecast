use anyhow::{Context as _, anyhow};
use bevy::{
    asset::RenderAssetUsages,
    platform::collections::HashMap,
    prelude::*,
    remote::BrpRequest,
    render::mesh::{Indices, PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};
use bevy_rerecast::editor_integration::{
    brp::{BRP_GET_NAVMESH_INPUT_METHOD, NavmeshInputResponse},
    transmission::deserialize,
};

use crate::{
    backend::{NavmeshAffector, NavmeshHandle},
    visualization::VisualMesh,
};

pub(super) fn plugin(app: &mut App) {
    app.add_observer(fetch_navmesh_input);
    app.add_systems(
        Update,
        poll_navmesh_input.run_if(resource_exists::<GetNavmeshInputRequestTask>),
    );
}

#[derive(Event)]
pub(crate) struct GetNavmeshInput;

#[derive(Resource)]
pub(crate) struct GetNavmeshInputRequestTask(Task<Result<NavmeshInputResponse, anyhow::Error>>);

fn fetch_navmesh_input(
    _: Trigger<GetNavmeshInput>,
    mut commands: Commands,
    maybe_task: Option<Res<GetNavmeshInputRequestTask>>,
) {
    if maybe_task.is_some() {
        // There's already an ongoing task, so we'll wait for it to complete.
        return;
    }
    let future = async {
        // Create the URL. We're going to need it to issue the HTTP request.
        let host_part = format!("{}:{}", "127.0.0.1", 15702);
        let url = format!("http://{host_part}/");
        let req = BrpRequest {
            jsonrpc: String::from("2.0"),
            method: String::from(BRP_GET_NAVMESH_INPUT_METHOD),
            id: Some(serde_json::to_value(1)?),
            params: None,
        };
        let request = ehttp::Request::json(url, &req)?;
        let resp = ehttp::fetch_async(request)
            .await
            .map_err(|s| anyhow!("{s}"))?;

        // Parse just the outer JSON
        let v: serde_json::Value = resp.json()?;

        // Grab the base64 blob from result
        let base64_blob = &v["result"];

        // Decode manually
        let response: NavmeshInputResponse = deserialize(base64_blob)?;
        Ok(response)
    };

    let task = AsyncComputeTaskPool::get().spawn(future);
    commands.insert_resource(GetNavmeshInputRequestTask(task));
}

fn poll_navmesh_input(
    mut task: ResMut<GetNavmeshInputRequestTask>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mesh_handles: Query<Entity, (With<Mesh3d>, Or<(With<VisualMesh>, With<NavmeshAffector>)>)>,
    gizmo_handles: Query<&Gizmo>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    mut navmesh_handle: ResMut<NavmeshHandle>,
) -> Result {
    let Some(result) = future::block_on(future::poll_once(&mut task.0)) else {
        return Ok(());
    };
    commands.remove_resource::<GetNavmeshInputRequestTask>();
    let response = result?;

    for entity in mesh_handles.iter() {
        commands.entity(entity).despawn();
    }
    for gizmo in gizmo_handles.iter() {
        let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
            continue;
        };
        gizmo.clear();
    }

    for affector in response.affector_meshes {
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, affector.mesh.vertices.clone())
            .with_inserted_indices(Indices::U32(
                affector
                    .mesh
                    .indices
                    .iter()
                    .flat_map(|indices| indices.to_array())
                    .collect(),
            ));

        commands.spawn((
            affector.transform.compute_transform(),
            Mesh3d(meshes.add(mesh)),
            NavmeshAffector(affector.mesh),
            Visibility::Hidden,
            Gizmo {
                handle: gizmos.add(GizmoAsset::new()),
                line_config: GizmoLineConfig {
                    perspective: true,
                    width: 20.0,
                    joints: GizmoLineJoint::Bevel,
                    ..default()
                },
                depth_bias: -0.001,
            },
        ));
    }

    let mut image_indices: HashMap<u32, Handle<Image>> = HashMap::new();
    let mut material_indices: HashMap<u32, Handle<StandardMaterial>> = HashMap::new();
    let mut mesh_indices: HashMap<u32, Handle<Mesh>> = HashMap::new();
    let fallback_material = materials.add(Color::WHITE);

    for visual in response.visual_meshes {
        let mesh = if let Some(mesh_handle) = mesh_indices.get(&visual.mesh) {
            mesh_handle.clone()
        } else {
            let serialized_mesh = response.meshes[visual.mesh as usize].clone();
            let mesh = serialized_mesh.into_mesh();
            let handle = meshes.add(mesh);
            mesh_indices.insert(visual.mesh, handle.clone());
            handle
        };

        let material = if let Some(index) = visual.material {
            if let Some(material_handle) = material_indices.get(&index) {
                material_handle.clone()
            } else {
                let serialized_material = response.materials[index as usize].clone();
                let material = serialized_material.into_standard_material(
                    &mut image_indices,
                    &mut images,
                    &response.images,
                );
                let handle = materials.add(material.clone());
                material_indices.insert(index, handle.clone());
                handle
            }
        } else {
            fallback_material.clone()
        };

        commands.spawn((
            visual.transform.compute_transform(),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            VisualMesh,
        ));
    }
    // Clear previous navmesh
    navmesh_handle.0 = Default::default();

    Ok(())
}
