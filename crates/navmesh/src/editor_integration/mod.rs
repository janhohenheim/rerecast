//! The optional editor integration for authoring the navmesh.

use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde_json::Value;

use crate::editor_integration::serialization::CloneProxy as _;

pub mod serialization;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Startup,
        setup_methods.run_if(resource_exists::<RemoteMethods>),
    );
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
    let proxy_mesh = mesh.clone_proxy();
    let serialized = serde_json::ser::to_string(&proxy_mesh).unwrap();
    //info!(?serialized);
    Ok(Value::String(serialized))
}

/// The BRP method that the navmesh editor uses to get the navmesh input.
pub const BRP_GET_NAVMESH_INPUT_METHOD: &str = "avian_navmesh/get_navmesh_input";
