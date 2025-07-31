#![doc = include_str!("../../../readme.md")]

use bevy_app::{PluginGroupBuilder, prelude::*};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemId};

use bevy_transform::components::GlobalTransform;

#[cfg(feature = "editor_integration")]
pub mod editor_integration;
#[cfg(feature = "bevy_mesh")]
mod mesh;
#[cfg(feature = "bevy_mesh")]
pub use mesh::Mesh3dNavmeshPlugin;

pub use rerecast;
use rerecast::TriMesh;

/// Everything you need to get started with the Navmesh plugins.
pub mod prelude {
    pub use crate::NavmeshPlugins;
}

/// The plugin group of the crate. Contains the following plugins:
/// - [`RerecastPlugin`]: The main plugin. Adds functionality for creating and managing navmeshes.
/// - [`RerecastEditorIntegrationPlugin`](editor_integration::RerecastEditorIntegrationPlugin): Allows communication with the editor.
///   Requires the `editor_integration` feature.
///
/// Note that rerecast does not do anything until you also add a navmesh affector backend.
/// A navmesh affector is something that represents non-walkable geometry in form of a [`TriMesh`].
///
/// A backend's job is to provide the [`TriMesh`]es that will be used to create the navmesh.
/// For example, if you enable the `bevy_mesh` feature, you can add the [`Mesh3dNavmeshPlugin`] to your app to
/// set a backend that generates navmeshes from entities with a [`Mesh3d`](bevy_render::mesh::Mesh3d) component.
///
/// To set your own backend, use [`NavmeshAffectorBackendAppExt::set_navmesh_affector_backend`].
/// Only one backend can be set at a time. Setting a new backend will replace the previous one.
/// By default, no backend is set.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NavmeshPlugins;

impl PluginGroup for NavmeshPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>().add(RerecastPlugin::default());
        #[cfg(feature = "editor_integration")]
        let builder = builder.add(editor_integration::RerecastEditorIntegrationPlugin::default());
        builder
    }
}

/// The main plugin of the crate. Adds functionality for creating and managing navmeshes.
#[non_exhaustive]
#[derive(Default)]
pub struct RerecastPlugin;

#[derive(Resource, Default, Clone, Deref, DerefMut)]
struct NavmeshAffectorBackend(Option<SystemId<(), Vec<(GlobalTransform, TriMesh)>>>);

/// Extension used to implement [`NavmeshAffectorBackendAppExt::set_navmesh_affector_backend`] on [`App`]
pub trait NavmeshAffectorBackendAppExt {
    /// Set the backend for generating navmesh affectors. Only one backend can be set at a time.
    /// Setting a backend will replace any existing backend. By default, no backend is set.
    fn set_navmesh_affector_backend<M>(
        &mut self,
        system: impl IntoSystem<(), Vec<(GlobalTransform, TriMesh)>, M> + 'static,
    ) -> &mut App;
}

impl NavmeshAffectorBackendAppExt for App {
    fn set_navmesh_affector_backend<M>(
        &mut self,
        system: impl IntoSystem<(), Vec<(GlobalTransform, TriMesh)>, M> + 'static,
    ) -> &mut App {
        let id = self.register_system(system);
        let systems = self
            .world_mut()
            .get_resource_mut::<NavmeshAffectorBackend>();
        let Some(mut systems) = systems else {
            tracing::error!(
                "Failed to set backend: internal resource not initialized. Did you forget to add the `NavmeshPlugins`?"
            );
            return self;
        };
        systems.replace(id);
        self
    }
}

impl Plugin for RerecastPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavmeshAffectorBackend>();
    }
}
