//! Compare the output of the C++ implementation with the Rust implementation.

use std::env;

use avian_navmesh::{heightfield::HeightfieldBuilder, span::AreaType, trimesh::TrimeshedCollider};
use bevy::prelude::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

#[test]
fn initial_heightfield() {
    let geometry = load_json::<CppGeometry>("geometry");
    let mut trimesh = geometry.to_trimesh();
    let walkable_slope = 45.0_f32.to_radians();
    trimesh.mark_walkable_triangles(walkable_slope);

    let aabb = trimesh.compute_aabb().unwrap();

    let mut heightfield = HeightfieldBuilder {
        aabb,
        cell_size: 0.3,
        cell_height: 0.2,
    }
    .build()
    .unwrap();

    let walkable_climb = 4;
    for (i, triangle) in trimesh.indices.iter().enumerate() {
        let triangle = [
            trimesh.vertices[triangle[0] as usize],
            trimesh.vertices[triangle[1] as usize],
            trimesh.vertices[triangle[2] as usize],
        ];
        let area_type = trimesh.area_types[i];
        heightfield
            .rasterize_triangle(triangle, area_type, walkable_climb)
            .unwrap();
    }

    let cpp_heightfield = load_json::<CppHeightfield>("heightfield_initial");

    println!("heightfield:");
    println!("\twidth: {}", heightfield.width);
    println!("\theight: {}", heightfield.height);
    println!("\taabb: {:?}", heightfield.aabb);
    println!("\tcell size: {}", heightfield.cell_size);
    println!("\tcell height: {}", heightfield.cell_height);

    println!("C++ heightfield:");
    println!("\twidth: {}", cpp_heightfield.width);
    println!("\theight: {}", cpp_heightfield.height);
    println!("\tbmin: {:?}", cpp_heightfield.bmin);
    println!("\tbmax: {:?}", cpp_heightfield.bmax);
    println!("\tcs: {}", cpp_heightfield.cs);
    println!("\tch: {}", cpp_heightfield.ch);

    assert_eq!(
        heightfield.width, cpp_heightfield.width,
        "heightfield width"
    );
    assert_eq!(
        heightfield.height, cpp_heightfield.height,
        "heightfield height"
    );
    assert_eq!(
        heightfield.aabb.min,
        Vec3A::from(cpp_heightfield.bmin),
        "heightfield aabb min"
    );
    assert_eq!(
        heightfield.aabb.max,
        Vec3A::from(cpp_heightfield.bmax),
        "heightfield aabb max"
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

    assert_eq!(
        heightfield.spans.len(),
        heightfield.width as usize * heightfield.height as usize
    );
    assert_eq!(
        cpp_heightfield.spans.len(),
        cpp_heightfield.width as usize * cpp_heightfield.height as usize
    );

    for x in 0..heightfield.width {
        for z in 0..heightfield.height {
            let column_index = x + z * heightfield.width;
            let cpp_span = cpp_heightfield.spans[column_index as usize].clone();
            let span_key = heightfield.span_key_at(x, z);
            if let EmptyOption::Some(mut cpp_span) = cpp_span {
                let mut layer = 0;
                let mut span_key = span_key.unwrap_or_else(|| {
                    panic!("C++ has a base span at [{x}, {z}] but Rust does not")
                });
                loop {
                    println!("layer {layer}");
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
                    layer += 1;
                }
            } else {
                assert!(
                    span_key.is_none(),
                    "C++ has no base span at [{x}, {z}] but Rust does"
                );
            }
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
enum EmptyOption<T> {
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

#[derive(Debug, Deserialize, Clone)]
struct CppGeometry {
    verts: Vec<[f32; 3]>,
    tris: Vec<[u32; 3]>,
}

impl CppGeometry {
    fn to_trimesh(&self) -> TrimeshedCollider {
        TrimeshedCollider {
            vertices: self.verts.iter().map(|v| Vec3A::from(*v)).collect(),
            indices: self.tris.iter().map(|i| UVec3::from(*i)).collect(),
            area_types: vec![AreaType::NOT_WALKABLE; self.tris.len()],
        }
    }
}

#[track_caller]
fn load_json<T: DeserializeOwned>(name: &str) -> T {
    let test_path = env::current_dir()
        .unwrap()
        .join("tests")
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
