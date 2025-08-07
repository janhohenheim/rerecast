//! Core code for [`bevy_rerecast`](https://docs.rs/bevy_rerecast),
//! which excludes the [`bevy_rerecast_editor_integration`](https://docs.rs/bevy_rerecast_editor_integration)

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
#[cfg(feature = "bevy_mesh")]
mod mesh;
use bevy_reflect::prelude::*;
#[cfg(feature = "bevy_mesh")]
pub use mesh::{Mesh3dBackendPlugin, TriMeshFromBevyMesh};
mod backend;
#[cfg(feature = "debug_plugin")]
pub mod debug;
pub mod generator;
pub use backend::*;
pub mod asset_loader;
#[macro_use]
extern crate alloc;

pub use rerecast;
use rerecast::{DetailNavmesh, PolygonNavmesh};
use serde::{Deserialize, Serialize};

/// Everything you need to use the crate.
pub mod prelude {
    pub use crate::{
        Navmesh, NavmeshApp as _, NavmeshSettings,
        generator::{NavmeshGenerator, NavmeshReady},
    };
}

/// The main plugin of the crate. Adds functionality for creating and managing navmeshes.
#[non_exhaustive]
#[derive(Default)]
pub struct RerecastPlugin;

impl Plugin for RerecastPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((generator::plugin, asset_loader::plugin));
    }
}

/// Resource containing the navmesh data.
/// Load this using either a file or by using [`NavmeshGenerator`](generator::NavmeshGenerator)
#[derive(Debug, Clone, PartialEq, Asset, Reflect, Serialize, Deserialize)]
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
