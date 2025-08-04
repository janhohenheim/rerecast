#![doc = include_str!("../../../readme.md")]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
#[cfg(feature = "debug_plugin")]
use bevy_rerecast_core::debug::{DetailNavmeshGizmo, PolygonNavmeshGizmo};
use serde::{Deserialize, Serialize};

pub mod brp;
pub mod transmission;

/// The optional editor integration for authoring the navmesh.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NavmeshEditorIntegrationPlugin;

impl Plugin for NavmeshEditorIntegrationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(brp::plugin);
        #[cfg(feature = "debug_plugin")]
        {
            app.add_observer(exclude_polygon_gizmo)
                .add_observer(exclude_detail_gizmo);
        }
        app.register_type::<EditorExluded>();
    }
}

#[cfg(feature = "debug_plugin")]
fn exclude_polygon_gizmo(trigger: Trigger<OnAdd, PolygonNavmeshGizmo>, mut commands: Commands) {
    commands.entity(trigger.target()).insert(EditorExluded);
}

#[cfg(feature = "debug_plugin")]
fn exclude_detail_gizmo(trigger: Trigger<OnAdd, DetailNavmeshGizmo>, mut commands: Commands) {
    commands.entity(trigger.target()).insert(EditorExluded);
}

/// Component used to mark [`Mesh3d`]es so that they're not sent to the editor for previewing the level.
#[derive(Debug, Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct EditorExluded;
