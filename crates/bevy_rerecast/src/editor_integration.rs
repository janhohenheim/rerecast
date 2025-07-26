//! The optional editor integration for authoring the navmesh.

use bevy::{
    ecs::system::SystemId,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods},
};
use bevy_rerecast_transmission::{SerializedMesh, serialize};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::NavmeshAffector;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Startup,
        setup_methods.run_if(resource_exists::<RemoteMethods>),
    );
    app.init_resource::<RasterizerSystems>();
    app.register_type::<EditorVisible>();
    app.add_rasterizer(rasterize_meshes);
}

/// Extension used to implement [`RerecastAppExt::add_rasterizer`] on [`App`]
pub trait RerecastAppExt {
    /// Add a system for rasterizing navmesh data. This will be called when the editor is fetching navmesh data.
    fn add_rasterizer<M>(
        &mut self,
        system: impl IntoSystem<(), Vec<(GlobalTransform, SerializedMesh)>, M> + 'static,
    ) -> &mut App;
}

impl RerecastAppExt for App {
    fn add_rasterizer<M>(
        &mut self,
        system: impl IntoSystem<(), Vec<(GlobalTransform, SerializedMesh)>, M> + 'static,
    ) -> &mut App {
        let id = self.register_system(system);
        let systems = self.world_mut().get_resource_mut::<RasterizerSystems>();
        let Some(mut systems) = systems else {
            error!(
                "Failed to add rasterizer: internal resource not initialized. Did you forget to add the `RerecastPlugin`?"
            );
            return self;
        };
        systems.push(id);
        self
    }
}

#[derive(Resource, Default, Clone, Deref, DerefMut)]
struct RasterizerSystems(Vec<SystemId<(), Vec<(GlobalTransform, SerializedMesh)>>>);

fn rasterize_meshes(
    meshes: Res<Assets<Mesh>>,
    affectors: Query<(&GlobalTransform, &Mesh3d), With<NavmeshAffector<Mesh3d>>>,
) -> Vec<(GlobalTransform, SerializedMesh)> {
    affectors
        .iter()
        .filter_map(|(transform, mesh)| {
            let transform = *transform;
            let mesh = meshes.get(mesh)?;
            let proxy_mesh = SerializedMesh::from_mesh(mesh);
            Some((transform, proxy_mesh))
        })
        .collect::<Vec<_>>()
}

/// Component used to mark [`Mesh3d`]es so that they're not sent to the editor for previewing the level.
#[derive(Debug, Component, Reflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
#[reflect(Component)]
pub struct EditorVisible;

fn setup_methods(mut methods: ResMut<RemoteMethods>, mut commands: Commands) {
    methods.insert(
        BRP_GET_NAVMESH_INPUT_METHOD,
        RemoteMethodSystemId::Instant(commands.register_system(get_navmesh_input)),
    );
}

fn get_navmesh_input(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    if let Some(params) = params {
        return Err(BrpError {
            code: bevy::remote::error_codes::INVALID_PARAMS,
            message: format!(
                "BRP method `{BRP_GET_NAVMESH_INPUT_METHOD}` requires no parameters, but received {params}"
            ),
            data: None,
        });
    }

    let Some(system_ids) = world.get_resource::<RasterizerSystems>().cloned() else {
        return Err(BrpError {
            code: bevy::remote::error_codes::INTERNAL_ERROR,
            message: "Failed to get rasterizer systems".to_string(),
            data: None,
        });
    };
    let mut affectors = Vec::new();
    for id in system_ids.iter() {
        let rasterizer_result = world.run_system(*id);
        if let Ok(rasterizer_response) = rasterizer_result {
            affectors.extend(rasterizer_response);
        }
    }

    let mut visuals = world
        .query_filtered::<(&GlobalTransform, &Mesh3d, &InheritedVisibility), With<EditorVisible>>();
    let Some(meshes) = world.get_resource::<Assets<Mesh>>() else {
        return Err(BrpError {
            code: bevy::remote::error_codes::INTERNAL_ERROR,
            message: "Failed to get meshes".to_string(),
            data: None,
        });
    };
    let visuals = visuals
        .iter(world)
        .filter_map(|(transform, mesh, visibility)| {
            if !matches!(*visibility, InheritedVisibility::VISIBLE) {
                return None;
            }
            let transform = *transform;
            let mesh = meshes.get(mesh)?;
            let proxy_mesh = SerializedMesh::from_mesh(mesh);
            Some((transform, proxy_mesh))
        })
        .collect::<Vec<_>>();
    let response = NavmeshInputResponse {
        affector_meshes: affectors,
        visual_meshes: visuals,
    };

    serialize(&response).map_err(|e| BrpError {
        code: bevy::remote::error_codes::INTERNAL_ERROR,
        message: format!("Failed to serialize navmesh input: {e}"),
        data: None,
    })
}

/// The BRP method that the navmesh editor uses to get the navmesh input.
pub const BRP_GET_NAVMESH_INPUT_METHOD: &str = "bevy_rerecast/get_navmesh_input";

/// The response to [`BRP_GET_NAVMESH_INPUT_METHOD`] requests.
#[derive(Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct NavmeshInputResponse {
    /// The meshes that affect the navmesh.
    pub affector_meshes: Vec<(GlobalTransform, SerializedMesh)>,
    /// Additional meshes that don't affect the navmesh, but are sent to the editor for visualization.
    pub visual_meshes: Vec<(GlobalTransform, SerializedMesh)>,
}
