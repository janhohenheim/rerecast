use anyhow::Context as _;
use avian_navmesh::editor_integration::{
    BRP_GET_NAVMESH_INPUT_METHOD, NavmeshInputResponse, serialization::deserialize,
};
use avian3d::prelude::*;
use bevy::{input::common_conditions::input_just_pressed, prelude::*, remote::BrpRequest};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        fetch_navmesh_input.run_if(input_just_pressed(KeyCode::Space)),
    );
}

fn fetch_navmesh_input(
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mesh_handles: Query<Entity, With<Mesh3d>>,
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
    for (transform, mesh) in response.meshes {
        let mesh: Mesh = mesh.into();
        let mesh = meshes.add(mesh);

        commands.spawn((
            transform.compute_transform(),
            Mesh3d(mesh),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                ..default()
            })),
        ));
    }
    for rigid_bodies in response.rigid_bodies {
        let mut entity_commands = commands.spawn((RigidBody::Static, Transform::default()));
        for (transform, collider) in rigid_bodies {
            entity_commands.with_child((transform.compute_transform(), collider));
        }
    }

    Ok(())
}
