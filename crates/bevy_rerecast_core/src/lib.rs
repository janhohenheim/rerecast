#![doc = include_str!("../../../readme.md")]

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
#[cfg(feature = "bevy_mesh")]
mod mesh;
use bevy_reflect::Reflect;
#[cfg(feature = "bevy_mesh")]
pub use mesh::{Mesh3dNavmeshPlugin, TriMeshFromBevyMesh};
mod backend;
pub mod generator;
pub use backend::*;

pub use rerecast;
use rerecast::{DetailNavmesh, PolygonNavmesh};

/// The main plugin of the crate. Adds functionality for creating and managing navmeshes.
#[non_exhaustive]
#[derive(Default)]
pub struct RerecastPlugin;

impl Plugin for RerecastPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Navmesh>();
        app.add_plugins(generator::plugin);
    }
}

/// Resource containing the navmesh data.
/// Load this using either a file or by using [`NavmeshGenerator`](generator::NavmeshGenerator)
#[derive(Debug, Default, Clone, PartialEq, Asset, Reflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Navmesh {
    polygon: PolygonNavmesh,
    detail: DetailNavmesh,
}
