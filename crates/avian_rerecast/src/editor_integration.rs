//! The optional editor integration for authoring the navmesh.

use avian3d::prelude::*;
use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use bevy_rerecast_transmission::{SerializedMesh, serialize};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    rigid_bodies: Query<(&RigidBody, &RigidBodyColliders)>,
    q_colliders: Query<(&GlobalTransform, &Collider)>,
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
            let proxy_mesh = SerializedMesh::from_mesh(mesh);
            Some((transform, proxy_mesh))
        })
        .collect::<Vec<_>>();

    let response = NavmeshInputResponse {
        meshes,
        rigid_bodies,
    };

    serialize(&response).map_err(|e| BrpError {
        code: bevy::remote::error_codes::INTERNAL_ERROR,
        message: format!("Failed to serialize navmesh input: {e}"),
        data: None,
    })
}
