#![doc = include_str!("../../../readme.md")]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_render::mesh::Mesh3d;
use serde::{Deserialize, Serialize};

pub mod brp;
pub mod transmission;

/// The optional editor integration for authoring the navmesh.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct RerecastEditorIntegrationPlugin {
    /// The settings for when [`EditorVisible`] is inserted automatically.
    pub visibility_settings: EditorVisibilitySettings,
}

impl Plugin for RerecastEditorIntegrationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(brp::plugin);
        app.register_type::<EditorVisible>();
        match self.visibility_settings {
            EditorVisibilitySettings::AllMeshes => {
                app.add_observer(insert_editor_visible_to_meshes);
            }
            EditorVisibilitySettings::Manual => {}
        }
    }
}

fn insert_editor_visible_to_meshes(trigger: Trigger<OnAdd, Mesh3d>, mut commands: Commands) {
    commands.entity(trigger.target()).insert(EditorVisible);
}

/// The settings for when [`EditorVisible`] is inserted automatically.
#[derive(Debug, Default)]
pub enum EditorVisibilitySettings {
    /// All entities with [`Mesh3d`] will have [EditorVisible`] inserted automatically.
    #[default]
    AllMeshes,
    /// [`EditorVisible`] will not be inserted automatically. The user must manually insert it.
    Manual,
}

/// Component used to mark [`Mesh3d`]es so that they're sent to the editor for previewing the level.
#[derive(Debug, Component, Reflect, Serialize, Deserialize)]
#[reflect(Component, Serialize, Deserialize)]
pub struct EditorVisible;
