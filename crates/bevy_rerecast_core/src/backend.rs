use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemId};
use bevy_platform::collections::HashSet;
use bevy_reflect::prelude::*;
use bevy_transform::prelude::*;
use rerecast::{NavmeshConfigBuilder, TriMesh};
use serde::{Deserialize, Serialize};

/// The current backend registered through [`NavmeshApp::set_navmesh_affector_backend`]
#[derive(Resource, Debug, Clone, Deref, DerefMut)]
pub struct NavmeshAffectorBackend(
    pub SystemId<In<NavmeshAffectorBackendInput>, Vec<(GlobalTransform, TriMesh)>>,
);

/// Extension used to implement [`NavmeshApp::set_navmesh_affector_backend`] on [`App`]
pub trait NavmeshApp {
    /// Set the backend for generating navmesh affectors. Only one backend can be set at a time.
    /// Setting a backend will replace any existing backend. By default, no backend is set.
    fn set_navmesh_affector_backend<M>(
        &mut self,
        system: impl IntoSystem<In<NavmeshAffectorBackendInput>, Vec<(GlobalTransform, TriMesh)>, M>
        + 'static,
    ) -> &mut App;
}

impl NavmeshApp for App {
    fn set_navmesh_affector_backend<M>(
        &mut self,
        system: impl IntoSystem<In<NavmeshAffectorBackendInput>, Vec<(GlobalTransform, TriMesh)>, M>
        + 'static,
    ) -> &mut App {
        let id = self.register_system(system);
        self.world_mut().insert_resource(NavmeshAffectorBackend(id));
        self
    }
}

/// The input passed to the navmesh affector backend system.
#[derive(Debug, Clone, Default, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct NavmeshAffectorBackendInput {
    /// The user-provided configuration for the navmesh.
    pub config: NavmeshConfigBuilder,
    /// An optional list of entities to generate navmesh affectors for.
    /// If `Some`, the backend is expected to only consider the specified entities when generating affectors.
    /// If `None`, the backend is expected to generate affectors for as many entities as is reasonable.
    pub filter: Option<HashSet<Entity>>,
}
