#![doc = include_str!("../../../readme.md")]

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

pub use rerecast;
use rerecast::{DetailNavmesh, PolygonNavmesh};
use serde::{Deserialize, Serialize};

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
