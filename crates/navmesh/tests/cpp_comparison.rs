//! Compare the output of the C++ implementation with the Rust implementation.

use std::env;

use avian_navmesh::{
    AreaType, CompactHeightfield, Heightfield, HeightfieldBuilder, Region, TrimeshedCollider,
};
use bevy::prelude::*;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

#[test]
fn validate_navmesh_against_cpp_implementation() {
    let geometry = load_json::<CppGeometry>("geometry");
    let mut trimesh = geometry.to_trimesh();
    let walkable_slope = 45.0_f32.to_radians();
    let walkable_height = 10;
    let walkable_climb = 4;
    let walkable_radius = 2;
    trimesh.mark_walkable_triangles(walkable_slope);

    let aabb = trimesh.compute_aabb().unwrap();

    let mut heightfield = HeightfieldBuilder {
        aabb,
        cell_size: 0.3,
        cell_height: 0.2,
    }
    .build()
    .unwrap();

    // Find triangles which are walkable based on their slope and rasterize them.
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
    assert_eq_heightfield(&heightfield, "heightfield_initial");

    // Once all geometry is rasterized, we do initial pass of filtering to
    // remove unwanted overhangs caused by the conservative rasterization
    // as well as filter spans where the character cannot possibly stand.
    heightfield.filter_low_hanging_walkable_obstacles(walkable_climb);
    heightfield.filter_ledge_spans(walkable_height, walkable_climb);
    heightfield.filter_walkable_low_height_spans(walkable_height);

    assert_eq_heightfield(&heightfield, "heightfield_filtered");

    let mut compact_heightfield =
        CompactHeightfield::from_heightfield(heightfield, walkable_height, walkable_climb).unwrap();

    assert_eq_compact_heightfield(&compact_heightfield, "compact_heightfield_initial");

    compact_heightfield.erode_walkable_area(walkable_radius);
    assert_eq_compact_heightfield(&compact_heightfield, "compact_heightfield_eroded");
}

#[track_caller]
fn assert_eq_heightfield(heightfield: &Heightfield, reference_name: &str) {
    let cpp_heightfield = load_json::<CppHeightfield>(reference_name);

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
            let column_index = x as usize + z as usize * heightfield.width as usize;
            let cpp_span = cpp_heightfield.spans[column_index].clone();
            let span_key = heightfield.span_key_at(x, z);
            if let EmptyOption::Some(mut cpp_span) = cpp_span {
                let mut layer = 0;
                let mut span_key = span_key.unwrap_or_else(|| {
                    panic!("C++ has a base span at [{x}, {z}] but Rust does not")
                });
                loop {
                    println!("layer {layer}");
                    let span = heightfield.allocated_spans[span_key].clone();
                    assert_eq!(span.min(), cpp_span.min, "[{x}, {z}] span min");
                    assert_eq!(span.max(), cpp_span.max, "[{x}, {z}] span max");
                    let cpp_area = if cpp_span.area == 63 {
                        // We use u8::MAX currently, though this may change in the future.
                        AreaType::DEFAULT_WALKABLE
                    } else {
                        AreaType::from(cpp_span.area)
                    };
                    assert_eq!(span.area(), cpp_area, "[{x}, {z}] span area");
                    if let EmptyOption::Some(next) = cpp_span.next {
                        span_key = span.next().unwrap();
                        cpp_span = *next;
                    } else {
                        assert!(span.next().is_none());
                        break;
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

#[track_caller]
fn assert_eq_compact_heightfield(compact_heightfield: &CompactHeightfield, reference_name: &str) {
    let cpp_heightfield = load_json::<CppCompactHeightfield>(reference_name);

    assert_eq!(
        compact_heightfield.width, cpp_heightfield.width,
        "compact_heightfield width"
    );
    assert_eq!(
        compact_heightfield.height, cpp_heightfield.height,
        "compact_heightfield height"
    );
    assert_eq!(
        compact_heightfield.walkable_height, cpp_heightfield.walkable_height,
        "compact_heightfield walkable height"
    );
    assert_eq!(
        compact_heightfield.walkable_climb, cpp_heightfield.walkable_climb,
        "compact_heightfield walkable climb"
    );
    assert_eq!(
        compact_heightfield.border_size, cpp_heightfield.border_size,
        "compact_heightfield border size"
    );
    assert_eq!(
        compact_heightfield.max_region,
        Region(cpp_heightfield.max_regions),
        "compact_heightfield max region"
    );
    assert_eq!(
        compact_heightfield.max_distance, cpp_heightfield.max_distance,
        "compact_heightfield max distance"
    );
    assert_eq!(
        compact_heightfield.aabb.min,
        Vec3A::from(cpp_heightfield.bmin),
        "compact_heightfield aabb min"
    );
    assert_eq!(
        compact_heightfield.aabb.max,
        Vec3A::from(cpp_heightfield.bmax),
        "compact_heightfield aabb max"
    );
    assert_eq!(
        compact_heightfield.cell_size, cpp_heightfield.cs,
        "compact_heightfield cell size"
    );
    assert_eq!(
        compact_heightfield.cell_height, cpp_heightfield.ch,
        "compact_heightfield cell height"
    );
    assert_eq!(
        compact_heightfield.cells.len(),
        cpp_heightfield.cells.len(),
        "compact_heightfield cells length"
    );
    assert_eq!(
        compact_heightfield.spans.len(),
        cpp_heightfield.spans.len(),
        "compact_heightfield spans length"
    );
    if !cpp_heightfield.dist.is_empty() {
        // we preallocate the dist array to the maximum possible size, so an empty array will be full of 0s for us
        assert_eq!(
            compact_heightfield.dist.len(),
            cpp_heightfield.dist.len(),
            "compact_heightfield dist length"
        );
    }
    assert_eq!(
        compact_heightfield.areas.len(),
        cpp_heightfield.areas.len(),
        "compact_heightfield areas length"
    );

    assert_eq!(
        compact_heightfield.cells.len(),
        compact_heightfield.width as usize * compact_heightfield.height as usize
    );
    assert_eq!(
        cpp_heightfield.cells.len(),
        cpp_heightfield.width as usize * cpp_heightfield.height as usize
    );

    for (i, (cell, cpp_cell)) in compact_heightfield
        .cells
        .iter()
        .zip(cpp_heightfield.cells.iter())
        .enumerate()
    {
        assert_eq!(
            cell.index(),
            cpp_cell.index,
            "compact_heightfield cell index {i}"
        );
        assert_eq!(
            cell.count(),
            cpp_cell.count,
            "compact_heightfield cell count {i}"
        );
    }

    for (i, (span, cpp_span)) in compact_heightfield
        .spans
        .iter()
        .zip(cpp_heightfield.spans.iter())
        .enumerate()
    {
        assert_eq!(span.y, cpp_span.y, "compact_heightfield span y {i}");
        assert_eq!(
            span.region,
            Region(cpp_span.reg),
            "compact_heightfield span reg {i}"
        );
        let first_24_bits = span.data & 0x00FF_FFFF;
        assert_eq!(
            first_24_bits, cpp_span.con,
            "compact_heightfield span con {i}"
        );
        assert_eq!(
            span.height(),
            cpp_span.h,
            "compact_heightfield span height {i}"
        );
    }

    for (i, (dist, cpp_dist)) in compact_heightfield
        .dist
        .iter()
        .zip(cpp_heightfield.dist.iter())
        .enumerate()
    {
        assert_eq!(*dist, *cpp_dist, "compact_heightfield dist {i}");
    }

    for (i, (area, cpp_area)) in compact_heightfield
        .areas
        .iter()
        .zip(cpp_heightfield.areas.iter())
        .enumerate()
    {
        let cpp_area = if *cpp_area == 63 {
            AreaType::DEFAULT_WALKABLE
        } else {
            AreaType::from(*cpp_area)
        };
        assert_eq!(*area, cpp_area, "compact_heightfield area {i}");
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

#[derive(Debug, Deserialize, Clone)]
struct CppCompactHeightfield {
    width: u16,
    height: u16,
    #[serde(rename = "walkableHeight")]
    walkable_height: u16,
    #[serde(rename = "walkableClimb")]
    walkable_climb: u16,
    #[serde(rename = "borderSize")]
    border_size: u16,
    #[serde(rename = "maxDistance")]
    max_distance: u16,
    #[serde(rename = "maxRegions")]
    max_regions: u16,
    bmin: [f32; 3],
    bmax: [f32; 3],
    cs: f32,
    ch: f32,
    cells: Vec<CppCompactCell>,
    spans: Vec<CppCompactSpan>,
    dist: Vec<u16>,
    areas: Vec<u8>,
}

#[derive(Debug, Deserialize, Clone)]
struct CppCompactCell {
    index: u32,
    count: u8,
}

#[derive(Debug, Deserialize, Clone)]
struct CppCompactSpan {
    y: u16,
    reg: u16,
    con: u32,
    h: u8,
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
