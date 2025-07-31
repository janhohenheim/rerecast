//! The editor for the Navmesh plugin.

use bevy::{
    ecs::error::{GLOBAL_ERROR_HANDLER, warn},
    prelude::*,
};
use bevy_rerecast::prelude::*;
use bevy_ui_text_input::TextInputPlugin;

mod build;
mod camera;
mod get_navmesh_input;
mod theme;
mod ui;
mod visualization;

fn main() -> AppExit {
    GLOBAL_ERROR_HANDLER
        .set(warn)
        .expect("The error handler can only be set once, globally.");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((NavmeshPlugins::default(), TextInputPlugin))
        .add_plugins((
            camera::plugin,
            get_navmesh_input::plugin,
            ui::plugin,
            theme::plugin,
            build::plugin,
            visualization::plugin,
        ))
        .run()
}
