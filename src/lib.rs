#![warn(missing_docs)]
#![doc = include_str!("../readme.md")]

use bevy::prelude::*;

/// Everything you need to get started with the NavMesh plugin.
pub mod prelude {
    pub use crate::NavMeshPlugin;
}

#[non_exhaustive]
#[derive(Default)]
pub struct NavMeshPlugin;

impl Plugin for NavMeshPlugin {
    fn build(&self, app: &mut App) {}
}
