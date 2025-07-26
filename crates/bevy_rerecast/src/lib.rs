#![doc = include_str!("../../../readme.md")]

use std::marker::PhantomData;

use bevy::{app::PluginGroupBuilder, prelude::*};

#[cfg(feature = "editor_integration")]
pub mod editor_integration;

pub use rerecast;

/// Everything you need to get started with the NavMesh plugin.
pub mod prelude {
    pub use crate::{NavmeshAffector, NavmeshPlugins};
}

/// The plugin group of the crate.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NavmeshPlugins;

impl PluginGroup for NavmeshPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>().add(RerecastPlugin::default());
        #[cfg(feature = "editor_integration")]
        let builder = builder.add(editor_integration::RerecastEditorIntegrationPlugin::default());
        builder
    }
}

/// The plugin of the crate.
#[non_exhaustive]
#[derive(Default)]
pub struct RerecastPlugin;

impl Plugin for RerecastPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<NavmeshAffector<Mesh3d>>();
    }
}

/// Component used to mark [`Mesh`]es as navmesh affectors.
/// Only meshes with this component will be considered when building the navmesh.
#[derive(Debug, Component, Reflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
#[reflect(Component)]
pub struct NavmeshAffector<T> {
    #[reflect(ignore)]
    _pd: PhantomData<T>,
}

impl<T> Default for NavmeshAffector<T> {
    fn default() -> Self {
        Self { _pd: PhantomData }
    }
}
