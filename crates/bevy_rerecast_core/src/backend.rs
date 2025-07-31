use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemId};
use bevy_transform::prelude::*;
use rerecast::TriMesh;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<NavmeshAffectorBackend>();
}

#[derive(Resource, Default, Clone, Deref, DerefMut)]
pub(crate) struct NavmeshAffectorBackend(Option<SystemId<(), Vec<(GlobalTransform, TriMesh)>>>);

/// Extension used to implement [`NavmeshAffectorBackendAppExt::set_navmesh_affector_backend`] on [`App`]
pub trait NavmeshApp {
    /// Set the backend for generating navmesh affectors. Only one backend can be set at a time.
    /// Setting a backend will replace any existing backend. By default, no backend is set.
    fn set_navmesh_affector_backend<M>(
        &mut self,
        system: impl IntoSystem<(), Vec<(GlobalTransform, TriMesh)>, M> + 'static,
    ) -> &mut App;
}

impl NavmeshApp for App {
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
