#![doc = include_str!("../../../readme.md")]

use bevy_app::{PluginGroupBuilder, prelude::*};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemId};

use bevy_transform::components::GlobalTransform;

#[cfg(feature = "editor_integration")]
pub mod editor_integration;
#[cfg(feature = "bevy_mesh")]
pub mod mesh;

pub use rerecast;
use rerecast::TriMesh;

/// Everything you need to get started with the Navmesh plugins.
pub mod prelude {
    pub use crate::NavmeshPlugins;
}

/// The plugin group of the crate.
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

/// The plugin of the crate.
#[non_exhaustive]
#[derive(Default)]
pub struct RerecastPlugin;

#[derive(Resource, Default, Clone, Deref, DerefMut)]
struct NavmeshAffectorBackend(Option<SystemId<(), Vec<(GlobalTransform, TriMesh)>>>);

/// Extension used to implement [`RerecastAppExt::add_rasterizer`] on [`App`]
pub trait RerecastAppExt {
    /// Add a system for rasterizing navmesh data. This will be called when the editor is fetching navmesh data.
    fn set_navmesh_affector_backend<M>(
        &mut self,
        system: impl IntoSystem<(), Vec<(GlobalTransform, TriMesh)>, M> + 'static,
    ) -> &mut App;
}

impl RerecastAppExt for App {
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
                "Failed to add rasterizer: internal resource not initialized. Did you forget to add the `RerecastPlugin`?"
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
