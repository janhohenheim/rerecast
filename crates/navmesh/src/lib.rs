#![warn(missing_docs)]
#![doc = include_str!("../../../readme.md")]

use bevy::prelude::*;

use crate::heightfield::Heightfield;
#[cfg(feature = "editor_integration")]
pub mod editor_integration;
mod heightfield;
pub mod rasterize_collider;
mod span;

/// Everything you need to get started with the NavMesh plugin.
pub mod prelude {
    pub use crate::NavMeshPlugin;
}

#[non_exhaustive]
#[derive(Default)]
pub struct NavMeshPlugin;

impl Plugin for NavMeshPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "editor_integration")]
        app.add_plugins(editor_integration::plugin);
    }
}
