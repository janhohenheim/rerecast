#![doc = include_str!("../../../readme.md")]

use bevy::prelude::*;

mod compact_cell;
mod compact_heightfield;
mod compact_span;
#[cfg(feature = "editor_integration")]
pub mod editor_integration;
mod erosion;
mod heightfield;
mod main_api;
pub(crate) mod math;
mod pre_filter;
mod rasterize;
mod region;
mod span;
mod trimesh;

pub use compact_cell::CompactCell;
pub use compact_heightfield::CompactHeightfield;
pub use compact_span::CompactSpan;
pub use heightfield::{Heightfield, HeightfieldBuilder, HeightfieldBuilderError};
pub use region::Region;
pub use span::{AreaType, Span, SpanKey};
pub use trimesh::TrimeshedCollider;

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
