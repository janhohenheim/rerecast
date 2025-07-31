use anyhow::Context as _;
use bevy::{platform::collections::HashMap, prelude::*, remote::BrpRequest};
use bevy_rerecast::{
    NavmeshAffector,
    editor_integration::{BRP_GET_NAVMESH_INPUT_METHOD, NavmeshInputResponse},
};
use bevy_rerecast_transmission::deserialize;

use crate::visualization::VisualMesh;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(fetch_navmesh_input);
}

#[derive(Event)]
pub(crate) struct GetNavmeshInput;

fn fetch_navmesh_input(
    _: Trigger<GetNavmeshInput>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mesh_handles: Query<
        Entity,
        (
            With<Mesh3d>,
            Or<(With<VisualMesh>, With<NavmeshAffector<Mesh3d>>)>,
        ),
    >,
    gizmo_handles: Query<&Gizmo>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
) -> Result {
    // Create the URL. We're going to need it to issue the HTTP request.
    let host_part = format!("{}:{}", "127.0.0.1", 15702);
    let url = format!("http://{host_part}/");

    let req = BrpRequest {
        jsonrpc: String::from("2.0"),
        method: String::from(BRP_GET_NAVMESH_INPUT_METHOD),
        id: Some(serde_json::to_value(1)?),
        params: None,
    };

    let response = ureq::post(&url)
        .send_json(req)?
        .body_mut()
        .read_json::<serde_json::Value>()?;
    let result = response
        .get("result")
        .context("Failed to get `result` from response")?;
    let response: NavmeshInputResponse = deserialize(result)?;

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
        let mesh = affector.mesh.into_mesh();

        commands.spawn((
            affector.transform.compute_transform(),
            Mesh3d(meshes.add(mesh)),
            NavmeshAffector::<Mesh3d>::default(),
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

    Ok(())
}
