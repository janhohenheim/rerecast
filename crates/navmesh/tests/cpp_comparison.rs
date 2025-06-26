use std::{env, path::PathBuf};

use avian_navmesh::{heightfield::HeightfieldBuilder, trimesh::TrimeshedCollider};
use bevy::{
    gltf::GltfPlugin,
    log::LogPlugin,
    pbr::PbrPlugin,
    prelude::*,
    render::{mesh::MeshPlugin, view::PreviousVisibleEntities},
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
            _ => {}
        }
    }
}

fn compare_heightfields(trigger: Trigger<MeshLoaded>, meshes: Res<Assets<Mesh>>) {
    let mesh = meshes.get(trigger.event().0).unwrap();
    let trimesh = TrimeshedCollider::from_mesh(mesh).unwrap();

    let aabb = trimesh.compute_aabb().unwrap();

    let mut heightfield = HeightfieldBuilder {
        aabb,
        cell_size: 0.3,
        cell_height: 0.2,
    }
    .build()
    .unwrap();
    heightfield.populate_from_trimesh(trimesh, 10, 4).unwrap();

    let cpp_heightfield = load_json::<CppHeightfield>("heightfield_initial");

    info!("heightfield:");
    info!("\twidth: {}", heightfield.width);
    info!("\theight: {}", heightfield.height);
    info!("\taabb: {:?}", heightfield.aabb);
    info!("\tcell size: {}", heightfield.cell_size);
    info!("\tcell height: {}", heightfield.cell_height);

    info!("C++ heightfield:");
    info!("\twidth: {}", cpp_heightfield.width);
    info!("\theight: {}", cpp_heightfield.height);
    info!("\tbmin: {:?}", cpp_heightfield.bmin);
    info!("\tbmax: {:?}", cpp_heightfield.bmax);
    info!("\tcs: {}", cpp_heightfield.cs);
    info!("\tch: {}", cpp_heightfield.ch);

    assert_eq!(
        heightfield.width, cpp_heightfield.width,
        "heightfield width"
    );
    assert_eq!(
        heightfield.height, cpp_heightfield.height,
        "heightfield height"
    );
    assert_eq!(
        heightfield.aabb.min.to_array(),
        cpp_heightfield.bmin,
        "heightfield bmin"
    );
    assert_eq!(
        heightfield.aabb.max.to_array(),
        cpp_heightfield.bmax,
        "heightfield bmax"
    );
    assert_eq!(
        heightfield.cell_size, cpp_heightfield.cs,
        "heightfield cell size"
    );
    assert_eq!(
        heightfield.cell_height, cpp_heightfield.ch,
        "heightfield cell height"
    );
    assert_eq!(
        heightfield.spans.len(),
        cpp_heightfield.spans.len(),
        "heightfield spans length"
    );
    for (i, span) in heightfield.spans.iter().enumerate() {
        let cpp_span = cpp_heightfield.spans[i].clone();
        if let EmptyOption::Some(mut cpp_span) = cpp_span {
            let mut span_key = span.unwrap();

            loop {
                let span = heightfield.allocated_spans[span_key].clone();
                assert_eq!(span.min(), cpp_span.min, "span min");
                assert_eq!(span.max(), cpp_span.max, "span max");
                assert_eq!(span.area().0, cpp_span.area, "span area");
                if let EmptyOption::Some(next) = cpp_span.next {
                    span_key = span.next().unwrap();
                    cpp_span = *next;
                } else {
                    assert!(span.next().is_none());
                }
            }
        } else {
            assert!(span.is_none());
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct CppHeightfield {
    width: u16,
    height: u16,
    bmin: [f32; 3],
    bmax: [f32; 3],
    cs: f32,
    ch: f32,
    spans: Vec<EmptyOption<CppSpan>>,
}

#[derive(Debug, Deserialize, Clone)]
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

impl<T: Clone> Clone for EmptyOption<T> {
    fn clone(&self) -> Self {
        match self {
            EmptyOption::Some(value) => EmptyOption::Some(value.clone()),
            EmptyOption::None {} => EmptyOption::None {},
        }
    }
}

#[track_caller]
fn load_json<T: DeserializeOwned>(name: &str) -> T {
    let test_path = env::current_dir()
        .unwrap()
        .join("assets")
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
