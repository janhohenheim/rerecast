//! The optional editor integration for authoring the navmesh.

use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::editor_integration::{
    input_data::{CloneProxy as _, ProxyMesh},
    serialization::serialize,
};

pub mod input_data;
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
    mesh_handles: Query<(&GlobalTransform, &Mesh3d)>,
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
    let meshes = mesh_handles
        .iter()
        .filter_map(|(transform, mesh)| {
            let transform = *transform;
            let mesh = meshes.get(mesh)?;
            let proxy_mesh = mesh.clone_proxy();
            Some((transform, proxy_mesh))
        })
        .collect::<Vec<_>>();
    let response = NavmeshInputResponse { meshes };

    serialize(&response).map_err(|e| BrpError {
        code: bevy::remote::error_codes::INTERNAL_ERROR,
        message: format!("Failed to serialize navmesh input: {e}"),
        data: None,
    })
}

/// The BRP method that the navmesh editor uses to get the navmesh input.
pub const BRP_GET_NAVMESH_INPUT_METHOD: &str = "avian_navmesh/get_navmesh_input";

/// The response to [`BRP_GET_NAVMESH_INPUT_METHOD`] requests.
#[derive(Serialize, Deserialize)]
pub struct NavmeshInputResponse {
    /// All meshes of the current scene.
    pub meshes: Vec<(GlobalTransform, ProxyMesh)>,
}
