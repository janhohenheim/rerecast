use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemId};
use bevy_transform::prelude::*;
use rerecast::TriMesh;

/// The current backend registered through [`NavmeshApp::set_navmesh_affector_backend`]
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct NavmeshAffectorBackend(pub SystemId<(), Vec<(GlobalTransform, TriMesh)>>);

/// Extension used to implement [`NavmeshApp::set_navmesh_affector_backend`] on [`App`]
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
        self.world_mut().insert_resource(NavmeshAffectorBackend(id));
        self
    }
}
