use anyhow::Context as _;
use avian_navmesh::{
    editor_integration::{
        BRP_GET_NAVMESH_INPUT_METHOD, NavmeshInputResponse, input_data::ProxyMesh,
        serialization::deserialize,
    },
    prelude::*,
};
use bevy::{
    ecs::error::{GLOBAL_ERROR_HANDLER, warn},
    input::common_conditions::input_just_pressed,
    prelude::*,
    remote::BrpRequest,
};

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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(19.769, 50.702, 20.619).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/voortrekker_interior_1k_diffuse.ktx2"),
            specular_map: asset_server
                .load("environment_maps/voortrekker_interior_1k_specular.ktx2"),
            intensity: 2000.0,
            ..default()
        },
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Vec3::new(0.5, -1.0, 0.3), Vec3::Y),
    ));
}

fn fetch_navmesh_input(
    type_registry: Res<AppTypeRegistry>,
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

    Ok(())
}
