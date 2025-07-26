#![doc = include_str!("../../../readme.md")]

use bevy::prelude::*;

#[cfg(feature = "editor_integration")]
pub mod editor_integration;

pub use rerecast;

/// Everything you need to get started with the NavMesh plugin.
pub mod prelude {
    pub use crate::{NavmeshAffector, RerecastPlugin};
}

/// The plugin of the crate.
#[non_exhaustive]
#[derive(Default)]
pub struct RerecastPlugin;

impl Plugin for RerecastPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "editor_integration")]
        app.add_plugins(editor_integration::plugin);
        app.register_type::<NavmeshAffector>();
    }
}

/// Component used to mark [`Mesh`]es as navmesh affectors.
/// Only meshes with this component will be considered when building the navmesh.
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct NavmeshAffector;
