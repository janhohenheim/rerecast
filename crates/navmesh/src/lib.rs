#![doc = include_str!("../../../readme.md")]

use bevy::prelude::*;

pub mod compact_cell;
pub mod compact_heightfield;
pub mod compact_span;
#[cfg(feature = "editor_integration")]
pub mod editor_integration;
pub mod heightfield;
mod main_api;
pub(crate) mod math;
mod pre_filter;
pub mod rasterize;
pub mod region;
pub mod span;
pub mod trimesh;

/// Everything you need to get started with the NavMesh plugin.
pub mod prelude {
    pub use crate::NavMeshPlugin;
}

/// The plugin of the crate.
#[non_exhaustive]
#[derive(Default)]
pub struct NavMeshPlugin;

impl Plugin for NavMeshPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "editor_integration")]
        app.add_plugins(editor_integration::plugin);
    }
}
