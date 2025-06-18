//! The optional editor integration for authoring the navmesh.

use std::iter;

use anyhow::Context;
use base64::{Engine as _, prelude::BASE64_STANDARD};
use bevy::{
    platform::collections::HashSet,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use serde_json::Value;

use crate::editor_integration::input_data::CloneProxy as _;

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
    with_mesh: Query<Entity, With<Mesh3d>>,
    mesh_handles: Query<&Mesh3d>,
    parents: Query<&ChildOf>,
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
    let mut already_visited = HashSet::new();
    for entity in with_mesh.iter() {
        for entity in iter::once(entity).chain(parents.iter_ancestors(entity)) {
            if already_visited.contains(&entity) {
                continue;
            }
            already_visited.insert(entity);
            if let Some(mesh) = meshes.get(mesh_handles.get(entity).unwrap()) {
                serialized.push(mesh);
            }
        }
    }
    let first_mesh_handle = with_mesh.iter().next().unwrap();
    let mesh = meshes.get(first_mesh_handle).unwrap();
    let serialized = serialization::serialize_mesh(mesh).unwrap();
    Ok(serialized)
}

/// The BRP method that the navmesh editor uses to get the navmesh input.
pub const BRP_GET_NAVMESH_INPUT_METHOD: &str = "avian_navmesh/get_navmesh_input";
