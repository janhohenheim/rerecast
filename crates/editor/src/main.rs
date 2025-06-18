use anyhow::{Context as _, anyhow};
use avian_navmesh::{
    editor_integration::{BRP_GET_NAVMESH_INPUT_METHOD, serialization::ProxyMesh},
    prelude::*,
};
use bevy::{
    ecs::error::{GLOBAL_ERROR_HANDLER, warn},
    input::common_conditions::input_just_pressed,
    math::VectorSpace,
    prelude::*,
    reflect::serde::ReflectDeserializer,
    remote::BrpRequest,
};
use serde::de::{Deserialize as _, DeserializeSeed as _};

fn main() -> AppExit {
    GLOBAL_ERROR_HANDLER
        .set(warn)
        .expect("The error handler can only be set once, globally.");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NavMeshPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            fetch_navmesh_input.run_if(input_just_pressed(KeyCode::Space)),
        )
        .run()
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-20.0, 50.0, -20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn fetch_navmesh_input(
    type_registry: Res<AppTypeRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    let mesh_string = result
        .as_str()
        .context("Response `result` is not a string")?;

    let mesh: ProxyMesh = serde_json::from_str(mesh_string)?;

    info!("{mesh:?}");

    /*
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..default()
        })),
    )); */

    Ok(())
}
