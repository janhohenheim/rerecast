//! The optional editor integration for authoring the navmesh.

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_platform::collections::HashMap;
use bevy_remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods};
use bevy_render::prelude::*;
use bevy_rerecast_core::NavmeshAffectorBackend;
use bevy_transform::prelude::*;
use rerecast::TriMesh;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    EditorVisible,
    transmission::{SerializedImage, SerializedMesh, SerializedStandardMaterial, serialize},
};

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

fn get_navmesh_input(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    if let Some(params) = params {
        return Err(BrpError {
            code: bevy_remote::error_codes::INVALID_PARAMS,
            message: format!(
                "BRP method `{BRP_GET_NAVMESH_INPUT_METHOD}` requires no parameters, but received {params}"
            ),
            data: None,
        });
    }

    let Some(backend_id) = world.get_resource::<NavmeshAffectorBackend>().cloned() else {
        return Err(BrpError {
            code: bevy_remote::error_codes::INTERNAL_ERROR,
            message: "No navmesh affector backend found. Did you forget to add one?".to_string(),
            data: None,
        });
    };
    let affectors = match world.run_system(*backend_id) {
        Ok(result) => result,
        Err(err) => {
            return Err(BrpError {
                code: bevy_remote::error_codes::INTERNAL_ERROR,
                message: format!("Navmesh affector backend failed: {err}"),
                data: None,
            });
        }
    };
    let affectors = affectors
        .into_iter()
        .map(|(transform, mesh)| AffectorMesh { transform, mesh })
        .collect();

    let mut visuals = world.query_filtered::<(
        &GlobalTransform,
        &Mesh3d,
        &InheritedVisibility,
        Option<&MeshMaterial3d<StandardMaterial>>,
    ), With<EditorVisible>>();
    let Some(meshes) = world.get_resource::<Assets<Mesh>>() else {
        return Err(BrpError {
            code: bevy_remote::error_codes::INTERNAL_ERROR,
            message: "Failed to get meshes".to_string(),
            data: None,
        });
    };
    let Some(images) = world.get_resource::<Assets<Image>>() else {
        return Err(BrpError {
            code: bevy_remote::error_codes::INTERNAL_ERROR,
            message: "Failed to get images".to_string(),
            data: None,
        });
    };
    let Some(materials) = world.get_resource::<Assets<StandardMaterial>>() else {
        return Err(BrpError {
            code: bevy_remote::error_codes::INTERNAL_ERROR,
            message: "Failed to get images".to_string(),
            data: None,
        });
    };

    let mut image_indices: HashMap<Handle<Image>, u32> = HashMap::new();
    let mut material_indices: HashMap<Handle<StandardMaterial>, u32> = HashMap::new();
    let mut mesh_indices: HashMap<Handle<Mesh>, u32> = HashMap::new();
    let mut serialized_images: Vec<SerializedImage> = Vec::new();
    let mut serialized_materials: Vec<SerializedStandardMaterial> = Vec::new();
    let mut serialized_meshes: Vec<SerializedMesh> = Vec::new();

    let visuals = visuals
        .iter(world)
        .filter_map(|(transform, mesh_handle, visibility, material_handle)| {
            if !matches!(*visibility, InheritedVisibility::VISIBLE) {
                return None;
            }
            let transform = *transform;
            let mesh_index = if let Some(&index) = mesh_indices.get(&mesh_handle.0) {
                index
            } else {
                let mesh = meshes.get(mesh_handle)?;
                let index = serialized_meshes.len() as u32;
                serialized_meshes.push(SerializedMesh::from_mesh(mesh));
                mesh_indices.insert(mesh_handle.0.clone(), index);
                index
            };
            let material_index = if let Some(material_handle) = material_handle {
                if let Some(&index) = material_indices.get(&material_handle.0) {
                    Some(index)
                } else {
                    match materials.get(material_handle) {
                        Some(material) => {
                            let index = serialized_materials.len() as u32;
                            match SerializedStandardMaterial::try_from_standard_material(
                                material.clone(),
                                &mut image_indices,
                                images,
                                &mut serialized_images,
                            ) {
                                Ok(serialized_material) => {
                                    serialized_materials.push(serialized_material);
                                    material_indices.insert(material_handle.0.clone(), index);
                                    Some(index)
                                }
                                Err(_e) => None,
                            }
                        }
                        None => None,
                    }
                }
            } else {
                None
            };

            Some(VisualMesh {
                transform,
                mesh: mesh_index,
                material: material_index,
            })
        })
        .collect::<Vec<_>>();
    let response = NavmeshInputResponse {
        affector_meshes: affectors,
        visual_meshes: visuals,
        materials: serialized_materials,
        meshes: serialized_meshes,
        images: serialized_images,
    };

    serialize(&response).map_err(|e| BrpError {
        code: bevy_remote::error_codes::INTERNAL_ERROR,
        message: format!("Failed to serialize navmesh input: {e}"),
        data: None,
    })
}

/// The BRP method that the navmesh editor uses to get the navmesh input.
pub const BRP_GET_NAVMESH_INPUT_METHOD: &str = "bevy_rerecast/get_navmesh_input";

/// The response to [`BRP_GET_NAVMESH_INPUT_METHOD`] requests.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NavmeshInputResponse {
    /// The meshes that affect the navmesh.
    pub affector_meshes: Vec<AffectorMesh>,
    /// Additional meshes that don't affect the navmesh, but are sent to the editor for visualization.
    pub visual_meshes: Vec<VisualMesh>,
    /// Materials indexed by [`Self::visual_meshes`].
    pub materials: Vec<SerializedStandardMaterial>,
    /// Meshes indexed by [`Self::visual_meshes`].
    pub meshes: Vec<SerializedMesh>,
    /// Images indexed by [`Self::materials`].
    pub images: Vec<SerializedImage>,
}

/// A mesh that affects the navmesh.
#[derive(Debug, Serialize, Deserialize)]
pub struct AffectorMesh {
    /// The transform of the mesh.
    pub transform: GlobalTransform,
    /// The mesh data.
    pub mesh: TriMesh,
}

/// A mesh that doesn't affect the navmesh, but is sent to the editor for visualization.
#[derive(Debug, Serialize, Deserialize)]
pub struct VisualMesh {
    /// The transform of the mesh.
    pub transform: GlobalTransform,
    /// The index of the mesh in [`NavmeshInputResponse::meshes`].
    pub mesh: u32,
    /// The index of the material in [`NavmeshInputResponse::materials`].
    pub material: Option<u32>,
}
