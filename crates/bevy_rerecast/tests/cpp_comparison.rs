#![allow(missing_docs)]

use std::time::Instant;

use bevy::{
    asset::AssetPlugin,
    gltf::GltfPlugin,
    log::LogPlugin,
    prelude::*,
    render::{mesh::MeshPlugin, primitives::Aabb, view::VisibilityPlugin},
    scene::{SceneInstanceReady, ScenePlugin},
};
use bevy_rerecast::{Mesh3dBackendPlugin, prelude::*};
use bevy_rerecast_editor_integration::NavmeshEditorIntegrationPlugin;

#[test]
fn validate_bevy_navmesh_against_cpp_implementation() {
    let mut app = App::new();
    app.add_plugins(headless_plugins);

    app.add_plugins((
        NavmeshPlugins::default()
            .build()
            .disable::<NavmeshDebugPlugin>()
            .disable::<NavmeshEditorIntegrationPlugin>(),
        Mesh3dBackendPlugin::default(),
    ));
    app.add_observer(on_navmesh_ready);

    app.finish();
    app.cleanup();

    app.world_mut().run_system_cached(setup).unwrap();

    let now = Instant::now();
    while app.world().get_resource::<GltfLoaded>().is_none() {
        app.update();
        if now.elapsed().as_secs() > 5 {
            panic!("Timeout waiting for glTF to load");
        }
    }
    app.world_mut().run_system_cached(generate_navmesh).unwrap();
    let now = Instant::now();
    while app.world().get_resource::<IsNavmeshReady>().is_none() {
        app.update();
        if now.elapsed().as_secs() > 5 {
            panic!("Timeout waiting for navmesh generation");
        }
    }
    let navmesh_handle = app.world().resource::<NavmeshHandle>().0.clone();
    let navmesh = app
        .world()
        .resource::<Assets<Navmesh>>()
        .get(&navmesh_handle)
        .unwrap()
        .clone();

    let expected_navmesh: Handle<Navmesh> = app
        .world()
        .resource::<AssetServer>()
        .load("test/navmesh.nav");
    let now = Instant::now();
    let expected_navmesh = loop {
        app.update();
        if let Some(navmesh) = app
            .world()
            .resource::<Assets<Navmesh>>()
            .get(&expected_navmesh)
        {
            break navmesh.clone();
        }
        if now.elapsed().as_secs() > 5 {
            panic!("Timeout waiting for reading reference navmesh");
        }
    };

    assert_eq!(
        expected_navmesh, navmesh,
        "Generated navmesh does not match reference"
    );
}

#[derive(Resource)]
struct GltfLoaded;

#[derive(Resource)]
struct NavmeshHandle(Handle<Navmesh>);

#[derive(Resource)]
struct IsNavmeshReady;

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands
        .spawn(SceneRoot(assets.load("models/dungeon.glb#Scene0")))
        .observe(|_: Trigger<SceneInstanceReady>, mut commands: Commands| {
            commands.insert_resource(GltfLoaded);
        });
}

fn generate_navmesh(mut commands: Commands, mut generator: NavmeshGenerator) {
    let handle = generator.generate(Default::default());
    commands.insert_resource(NavmeshHandle(handle));
}

fn on_navmesh_ready(_trigger: Trigger<NavmeshReady>, mut commands: Commands) {
    commands.insert_resource(IsNavmeshReady);
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
