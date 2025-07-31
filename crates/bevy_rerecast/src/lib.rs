#![doc = include_str!("../../../readme.md")]

use bevy_app::{PluginGroupBuilder, prelude::*};
pub use bevy_rerecast_core::*;
pub use bevy_rerecast_editor_integration as editor_integration;

/// Everything you need to get started with the Navmesh plugins.
pub mod prelude {
    pub use crate::{Navmesh, NavmeshPlugins, generator::NavmeshGenerator};
}

/// The plugin group of the crate. Contains the following plugins:
/// - [`RerecastPlugin`]: The main plugin. Adds functionality for creating and managing navmeshes.
/// - [`RerecastEditorIntegrationPlugin`](editor_integration::RerecastEditorIntegrationPlugin): Allows communication with the editor.
///   Requires the `editor_integration` feature.
///
/// Note that rerecast does not do anything until you also add a navmesh affector backend.
/// A navmesh affector is something that represents non-walkable geometry in form of a [`TriMesh`](rerecast::TriMesh).
///
/// A backend's job is to provide the [`TriMesh`](rerecast::TriMesh)es that will be used to create the navmesh.
/// For example, if you enable the `bevy_mesh` feature, you can add the [`Mesh3dNavmeshPlugin`] to your app to
/// set a backend that generates navmeshes from entities with a `Mesh3d` component.
///
/// To set your own backend, use [`NavmeshApp::set_navmesh_affector_backend`].
/// Only one backend can be set at a time. Setting a new backend will replace the previous one.
/// By default, no backend is set.
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
