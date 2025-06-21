use avian_navmesh::prelude::*;
use avian3d::prelude::*;
use bevy::{
    ecs::error::{GLOBAL_ERROR_HANDLER, warn},
    prelude::*,
};

mod camera;
mod get_navmesh_input;
mod theme;
mod ui;

fn main() -> AppExit {
    GLOBAL_ERROR_HANDLER
        .set(warn)
        .expect("The error handler can only be set once, globally.");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()))
        .add_plugins(NavMeshPlugin::default())
        .add_plugins((
            camera::plugin,
            get_navmesh_input::plugin,
            ui::plugin,
            theme::plugin,
        ))
        .run()
}
