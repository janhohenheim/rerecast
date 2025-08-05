//! The optional editor integration for authoring the navmesh.

use std::str::FromStr;

use bevy_app::prelude::*;
use bevy_asset::{prelude::*, uuid::Uuid};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_platform::collections::HashMap;
use bevy_remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods};
use bevy_render::prelude::*;
use bevy_rerecast_core::{NavmeshAffectorBackend, NavmeshSettings};
use bevy_tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use bevy_transform::prelude::*;
use rerecast::TriMesh;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    EditorExluded,
    transmission::{SerializedImage, SerializedMesh, SerializedStandardMaterial, serialize},
};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<NavmeshInputTasks>();
    app.add_systems(
        Startup,
        setup_methods.run_if(resource_exists::<RemoteMethods>),
    );
}

fn setup_methods(mut methods: ResMut<RemoteMethods>, mut commands: Commands) {
    methods.insert(
        BRP_GENERATE_EDITOR_INPUT,
        RemoteMethodSystemId::Instant(commands.register_system(get_navmesh_input)),
    );
    methods.insert(
        BRP_POLL_EDITOR_INPUT,
        RemoteMethodSystemId::Watching(commands.register_system(poll_navmesh_input)),
    );
}

/// The parameters for [`BRP_GENERATE_EDITOR_INPUT`].
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GenerateEditorInputParams {
    /// Input for the navmesh affector backend.
    pub backend_input: NavmeshSettings,
}

fn get_navmesh_input(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let Some(params) = params else {
        return Err(BrpError {
            code: bevy_remote::error_codes::INVALID_PARAMS,
            message: format!(
                "BRP method `{BRP_GENERATE_EDITOR_INPUT}` requires a parameter, but received none"
            ),
            data: None,
        });
    };
    let params: GenerateEditorInputParams = match serde_json::from_value(params.clone()) {
        Ok(id) => id,
        Err(e) => {
            return Err(BrpError {
                code: bevy_remote::error_codes::INVALID_PARAMS,
                message: format!(
                    "{e}. BRP method `{BRP_GENERATE_EDITOR_INPUT}` requires a parameter of type `GenerateEditorInputParams`, but received `{params:#?}`"
                ),
                data: None,
            });
        }
    };
    let Some(backend_id) = world.get_resource::<NavmeshAffectorBackend>().cloned() else {
        return Err(BrpError {
            code: bevy_remote::error_codes::RESOURCE_NOT_PRESENT,
            message: "No navmesh affector backend found. Did you forget to add one?".to_string(),
            data: None,
        });
    };
    let affectors = match world.run_system_with(*backend_id, params.backend_input) {
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
    ), Without<EditorExluded>>();
    let Some(meshes) = world.get_resource::<Assets<Mesh>>() else {
        return Err(BrpError {
            code: bevy_remote::error_codes::RESOURCE_NOT_PRESENT,
            message: "Failed to get meshes".to_string(),
            data: None,
        });
    };
    let Some(images) = world.get_resource::<Assets<Image>>() else {
        return Err(BrpError {
            code: bevy_remote::error_codes::RESOURCE_NOT_PRESENT,
            message: "Failed to get images".to_string(),
            data: None,
        });
    };
    let Some(materials) = world.get_resource::<Assets<StandardMaterial>>() else {
        return Err(BrpError {
            code: bevy_remote::error_codes::RESOURCE_NOT_PRESENT,
            message: "Failed to get materials".to_string(),
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
    let response = PollEditorInputResponse {
        affector_meshes: affectors,
        visual_meshes: visuals,
        materials: serialized_materials,
        meshes: serialized_meshes,
        images: serialized_images,
    };
    let future = async move {
        serialize(&response).map_err(|e| BrpError {
            code: bevy_remote::error_codes::INTERNAL_ERROR,
            message: format!("Failed to serialize navmesh input: {e}"),
            data: None,
        })
    };
    let id = Uuid::new_v4();
    let mut tasks = world.resource_mut::<NavmeshInputTasks>();
    let task = AsyncComputeTaskPool::get().spawn(future);
    tasks.0.insert(id, task);

    let response = GenerateEditorInputResponse {
        id: EditorInputTaskId(id.to_string()),
    };
    serde_json::to_value(&response).map_err(|e| BrpError {
        code: bevy_remote::error_codes::INTERNAL_ERROR,
        message: format!("Failed to serialize editor task ID: {e}"),
        data: None,
    })
}

fn poll_navmesh_input(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult<Option<Value>> {
    let Some(params) = params else {
        return Err(BrpError {
            code: bevy_remote::error_codes::INVALID_PARAMS,
            message: format!(
                "BRP method `{BRP_POLL_EDITOR_INPUT}` requires a parameter, but received none"
            ),
            data: None,
        });
    };
    let params: PollEditorInputParams = match serde_json::from_value(params.clone()) {
        Ok(id) => id,
        Err(e) => {
            return Err(BrpError {
                code: bevy_remote::error_codes::INVALID_PARAMS,
                message: format!(
                    "{e}. BRP method `{BRP_POLL_EDITOR_INPUT}` requires a parameter of type `PollEditorInputParams`, but received `{params:#?}`"
                ),
                data: None,
            });
        }
    };
    let id = match Uuid::from_str(&params.id.0) {
        Ok(id) => id,
        Err(e) => {
            return Err(BrpError {
                code: bevy_remote::error_codes::INVALID_PARAMS,
                message: format!("{e}: Task ID must be a valid UUID"),
                data: None,
            });
        }
    };

    let mut tasks = world.resource_mut::<NavmeshInputTasks>();
    let Some(task) = tasks.get_mut(&id) else {
        return Err(BrpError {
            code: bevy_remote::error_codes::INVALID_PARAMS,
            message: format!(
                "Got an invalid task ID: {id}. Make sure to only use task IDs returned by `{BRP_GENERATE_EDITOR_INPUT}` and to not poll again once a poll was successful"
            ),
            data: None,
        });
    };

    match future::block_on(future::poll_once(task)) {
        Some(result) => {
            tasks.remove(&id);
            result.map(Some)
        }
        None => Ok(None),
    }
}

#[derive(Resource, Default, DerefMut, Deref)]
struct NavmeshInputTasks(HashMap<Uuid, Task<Result<Value, BrpError>>>);

/// The BRP method that the navmesh editor uses to get its input from the running app.
/// Call without params. Returns [`GenerateEditorInputResponse`].
pub const BRP_GENERATE_EDITOR_INPUT: &str = "bevy_rerecast/generate_editor_input";
/// The BRP method that the navmesh editor uses to poll the status of an editor input task.
/// Call with [`PollEditorInputParams`]. Returns [`PollEditorInputResponse`].
pub const BRP_POLL_EDITOR_INPUT: &str = "bevy_rerecast/poll_editor_input";

/// The response to [`BRP_GENERATE_EDITOR_INPUT`] requests.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GenerateEditorInputResponse {
    /// The ID of the async task that is generating the navmesh input.
    /// Callers are supposed to poll this ID regularly by calling [`BRP_POLL_EDITOR_INPUT`] with [`PollEditorInputParams`].
    pub id: EditorInputTaskId,
}

/// The response to [`BRP_POLL_EDITOR_INPUT`] requests.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PollEditorInputParams {
    /// The ID of the async task to poll. Must correspond to the ID returned by [`BRP_GENERATE_EDITOR_INPUT`] through [`GenerateEditorInputResponse`].
    pub id: EditorInputTaskId,
}

/// The ID of an editor input task. Must be read from [`GenerateEditorInputResponse`]
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct EditorInputTaskId(pub String);

/// Data for the editor. Provided in [`PollEditorInputResponse`].
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PollEditorInputResponse {
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
    /// The index of the mesh in [`PollEditorInputResponse::meshes`].
    pub mesh: u32,
    /// The index of the material in [`PollEditorInputResponse::materials`].
    pub material: Option<u32>,
}
