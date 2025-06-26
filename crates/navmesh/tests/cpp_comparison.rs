use std::{
    env,
    ffi::OsString,
    fs::read_dir,
    io,
    path::{Path, PathBuf},
};

use bevy::{
    gltf::GltfPlugin,
    log::LogPlugin,
    pbr::PbrPlugin,
    prelude::*,
    render::{mesh::MeshPlugin, view::PreviousVisibleEntities},
    scene::SceneInstanceReady,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

#[test]
fn initial_heightfield() {
    App::new()
        .add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Shader>()
        .init_asset::<Scene>()
        .init_resource::<PreviousVisibleEntities>()
        .add_plugins((
            GltfPlugin::default(),
            MeshPlugin,
            PbrPlugin::default(),
            LogPlugin::default(),
        ))
        .add_systems(Startup, load_heightfield)
        .add_systems(Update, handle_asset_event)
        .add_observer(compare_heightfields)
        .run();
}

fn load_heightfield(asset_server: Res<AssetServer>, mut commands: Commands) {
    let virtual_path = "models/dungeon.gltf#Mesh0/Primitive0";
    let global_path = virtual_path_to_global_path(virtual_path);
    if !global_path.exists() {
        panic!("Asset not found: {global_path:?}");
    }

    let mesh = asset_server.load(virtual_path);
    println!("{:?}", asset_server.get_load_state(mesh.id()));
    commands.spawn(Mesh3d(mesh));
}

#[derive(Event)]
struct MeshLoaded(AssetId<Mesh>);

fn handle_asset_event(mut events: EventReader<AssetEvent<Mesh>>, mut commands: Commands) {
    for event in events.read() {
        match event {
            AssetEvent::LoadedWithDependencies { id } => {
                commands.trigger(MeshLoaded(*id));
            }
            AssetEvent::Removed { id } => {
                panic!("Failed to load asset {id:?}");
            }
            AssetEvent::Unused { id } => {
                panic!("Failed to load asset {id:?}");
            }
            AssetEvent::Added { id } => {
                info!("Added: {id:?}");
            }
            AssetEvent::Modified { id } => {
                info!("Modified: {id:?}");
            }
        }
    }
}

fn compare_heightfields(trigger: Trigger<MeshLoaded>, meshes: Res<Assets<Mesh>>) {
    let mesh = meshes.get(trigger.event().0).unwrap();
}

#[derive(Debug, Deserialize)]
struct CppHeightfield {
    width: u16,
    height: u16,
    bmin: [f32; 3],
    bmax: [f32; 3],
    cs: f32,
    ch: f32,
    spans: Vec<EmptyOption<CppSpan>>,
}

#[derive(Debug, Deserialize)]
struct CppSpan {
    min: u16,
    max: u16,
    area: u8,
    next: EmptyOption<Box<CppSpan>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum EmptyOption<T> {
    Some(T),
    None {},
}

#[track_caller]
fn load_json<T: DeserializeOwned>(name: &str) -> T {
    let test_path = env::current_dir()
        .unwrap()
        .join("reference_data")
        .join(format!("{name}.json"));

    let file = std::fs::read_to_string(test_path.clone()).unwrap_or_else(|e| {
        panic!("Failed to read file: {}: {}", test_path.display(), e);
    });
    let value: Value = serde_json::from_str(&file).unwrap_or_else(|e| {
        panic!("Failed to parse JSON: {}: {}", test_path.display(), e);
    });
    serde_json::from_value(value).unwrap_or_else(|e| {
        panic!("Failed to deserialize JSON: {}: {}", test_path.display(), e);
    })
}

fn virtual_path_to_global_path(virtual_path: &str) -> PathBuf {
    // remove everything after the first #
    let stripped = virtual_path.split('#').next().unwrap();
    let mut path = env::current_dir().unwrap();
    path.push("assets");
    path.push(stripped);
    path
}
