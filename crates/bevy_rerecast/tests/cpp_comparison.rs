#![allow(missing_docs)]

use std::time::Instant;

use bevy::{
    asset::AssetPlugin,
    gltf::GltfPlugin,
    log::LogPlugin,
    prelude::*,
    render::{RenderPlugin, mesh::MeshPlugin},
    scene::{SceneInstanceReady, ScenePlugin},
};
use bevy_app::ScheduleRunnerPlugin;
use bevy_rerecast::{Mesh3dNavmeshPlugin, prelude::*};

#[test]
fn validate_bevy_navmesh_against_cpp_implementation() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        LogPlugin::default(),
        AssetPlugin {
            file_path: "../../assets".to_string(),
            ..default()
        },
        ScenePlugin,
        MeshPlugin,
        GltfPlugin::default(),
    ))
    .add_plugins((NavmeshPlugins::default(), Mesh3dNavmeshPlugin::default()));

    app.add_systems(Startup, setup);
    let now = Instant::now();
    while app.world().get_resource::<GltfLoaded>().is_none() {
        app.update();
        if now.elapsed().as_secs() > 10 {
            panic!("Timeout waiting for glTF to load");
        }
    }
}

#[derive(Resource)]
struct GltfLoaded;

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands
        .spawn(SceneRoot(assets.load("models/dungeon.glb#Scene0")))
        .observe(|_: Trigger<SceneInstanceReady>, mut commands: Commands| {
            commands.insert_resource(GltfLoaded);
        });
}
