#![allow(missing_docs)]

use std::{path::Path, time::Instant};

use bevy::{
    asset::{
        AssetPlugin, LoadState,
        io::{
            AssetSource, AssetSourceId,
            memory::{Dir, MemoryAssetReader},
        },
    },
    gltf::GltfPlugin,
    log::LogPlugin,
    prelude::*,
    render::{
        RenderPlugin, camera::CameraPlugin, mesh::MeshPlugin, primitives::Aabb,
        view::VisibilityPlugin,
    },
    scene::{SceneInstanceReady, ScenePlugin},
};
use bevy_app::ScheduleRunnerPlugin;
use bevy_rerecast::{Mesh3dNavmeshPlugin, prelude::*};

#[test]
fn validate_bevy_navmesh_against_cpp_implementation() {
    let mut app = App::new();
    app.add_plugins(headless_plugins);

    app.add_plugins((NavmeshPlugins::default(), Mesh3dNavmeshPlugin::default()));

    app.add_systems(Startup, setup);

    app.finish();
    app.cleanup();

    let now = Instant::now();
    while app.world().get_resource::<GltfLoaded>().is_none() {
        app.update();
        if now.elapsed().as_secs() > 5 {
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

fn headless_plugins(app: &mut App) {
    app.add_plugins((
        MinimalPlugins,
        LogPlugin::default(),
        AssetPlugin {
            file_path: "../../assets".to_string(),
            ..default()
        },
        ScenePlugin,
        MeshPlugin,
        TransformPlugin,
        VisibilityPlugin,
        GltfPlugin::default(),
    ))
    .init_asset::<StandardMaterial>()
    .register_type::<Visibility>()
    .register_type::<InheritedVisibility>()
    .register_type::<ViewVisibility>()
    .register_type::<Aabb>()
    .register_type::<MeshMaterial3d<StandardMaterial>>();
}
