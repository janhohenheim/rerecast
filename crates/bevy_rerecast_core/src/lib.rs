//! Core code for [`bevy_rerecast`](https://docs.rs/bevy_rerecast),
//! which excludes the [`bevy_rerecast_editor_integration`](https://docs.rs/bevy_rerecast_editor_integration)

#![no_std]
use bevy_app::prelude::*;
#[cfg(feature = "bevy_asset")]
use bevy_asset::prelude::*;
#[cfg(feature = "bevy_mesh")]
mod mesh;
use bevy_reflect::prelude::*;
#[cfg(feature = "bevy_mesh")]
pub use mesh::{Mesh3dBackendPlugin, TriMeshFromBevyMesh};
mod backend;
#[cfg(feature = "debug_plugin")]
pub mod debug;
#[cfg(feature = "bevy_asset")]
pub mod generator;
pub use backend::*;
#[cfg(feature = "bevy_asset")]
pub mod asset_loader;
extern crate alloc;

pub use rerecast;
use rerecast::{DetailNavmesh, PolygonNavmesh};
use serde::{Deserialize, Serialize};

/// Everything you need to use the crate.
pub mod prelude {
    #[cfg(feature = "bevy_asset")]
    pub use crate::generator::{NavmeshGenerator, NavmeshReady};
    pub use crate::{Navmesh, NavmeshApp as _, NavmeshSettings};
}

/// The main plugin of the crate. Adds functionality for creating and managing navmeshes.
#[non_exhaustive]
#[derive(Default)]
pub struct RerecastPlugin;

impl Plugin for RerecastPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_asset")]
        app.add_plugins(generator::plugin);
        #[cfg(feature = "bevy_asset")]
        app.add_plugins(asset_loader::plugin);
        let _ = app;
    }
}

/// Resource containing the navmesh data.
/// Load this using either a file or by using [`NavmeshGenerator`](generator::NavmeshGenerator)
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[cfg_attr(feature = "bevy_asset", derive(Asset))]
#[reflect(Serialize, Deserialize)]
pub struct Navmesh {
    /// The polygon navmesh data. This is a simplified representation of the navmesh that
    /// is efficient for pathfinding. To not clip an agent through floors or walls, users should
    /// use the [`Navmesh::detail`] to refine the path. This is especially important when walking up or down
    /// stairs, ramps, or slopes.
    ///
    /// If you can spare the performance cost, you can also always use [`Navmesh::detail`] to pathfind instead.
    pub polygon: PolygonNavmesh,

    /// The detail navmesh data. This is a more detailed representation of the navmesh that
    /// accurately follows geometry. It contains more data than the [`Navmesh::polygon`], so
    /// the latter is more efficient for pathfinding. Use this navmesh to refine the path.
    ///
    /// If you can spare the performance cost, you can also always use this navmesh to pathfind instead.
    pub detail: DetailNavmesh,

    /// The configuration that was used to generate this navmesh.
    pub settings: NavmeshSettings,
}
