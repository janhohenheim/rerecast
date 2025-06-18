//! The optional editor integration for authoring the navmesh.

use bevy::{
    prelude::*,
    reflect::{TypeRegistry, serde::ReflectSerializer},
    remote::{
        BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods, RemotePlugin,
        http::RemoteHttpPlugin,
    },
};
use serde_json::Value;

pub(super) fn plugin(app: &mut App) {
    if let Some(error_message) = match (
        contains_plugin::<RemotePlugin>(app),
        contains_plugin::<RemoteHttpPlugin>(app),
    ) {
        (true, true) => None,
        (true, false) => {
            Some("`RemoteHttpPlugin` was not found. Please add it before `NavmeshPlugin`")
        }
        (false, true) => Some("`RemotePlugin` was not found. Please add it before `NavmeshPlugin`"),
        (false, false) => Some(
            "`RemotePlugin` and `RemoteHttpPlugin` were not found. Please add them before `NavmeshPlugin`",
        ),
    } {
        warn!("Failed to set up navmesh editor integration: {error_message}");
        return;
    }
    app.add_systems(Startup, setup_methods);
}

fn contains_plugin<T: Plugin>(app: &App) -> bool {
    !app.get_added_plugins::<T>().is_empty()
}

fn setup_methods(mut methods: ResMut<RemoteMethods>, mut commands: Commands) {
    methods.insert(
        BRP_GET_NAVMESH_INPUT_METHOD,
        RemoteMethodSystemId::Instant(commands.register_system(get_navmesh_input)),
    );
}

fn get_navmesh_input(
    In(params): In<Option<Value>>,
    meshes: Res<Assets<Mesh>>,
    mesh_handles: Query<&Mesh3d>,
    type_registry: Res<AppTypeRegistry>,
) -> BrpResult {
    if let Some(params) = params {
        return Err(BrpError {
            code: bevy::remote::error_codes::INVALID_PARAMS,
            message: format!(
                "BRP method `{BRP_GET_NAVMESH_INPUT_METHOD}` requires no parameters, but received {params}"
            ),
            data: None,
        });
    }
    let first_mesh_handle = mesh_handles.iter().next().unwrap();
    let mesh = meshes.get(first_mesh_handle).unwrap();
    info!("{mesh:?}");
    let type_registry = type_registry.read();

    let serializer = ReflectSerializer::new(mesh, &type_registry);
    let serialized = serde_json::ser::to_string_pretty(&serializer).unwrap();
    Ok(Value::String(serialized))
}

/// The BRP method that the navmesh editor uses to get the navmesh input.
pub const BRP_GET_NAVMESH_INPUT_METHOD: &str = "avian_navmesh/get_navmesh_input";
