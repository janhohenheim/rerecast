//! Internal test utilities.
#![allow(missing_docs)]

use approxim::relative_eq;
use glam::{U8Vec3, UVec3, Vec3, Vec3A};
use rerecast::{
    Aabb3d, AreaType, BuildContoursFlags, CompactHeightfield, Config, ContourSet, DetailNavmesh,
    Heightfield, PolygonNavmesh, RegionId, TriMesh,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use std::{env, path::PathBuf};

pub trait NavmeshConfigTest {
    fn load_from_test_data(project: &str) -> Config;
}

impl NavmeshConfigTest for Config {
    fn load_from_test_data(project: &str) -> Config {
        let config = load_json::<CppConfig>(project, "config");
        Config {
            width: config.width,
            height: config.height,
            tile_size: config.tile_size,
            border_size: config.border_size,
            cell_size: config.cs,
            cell_height: config.ch,
            aabb: Aabb3d {
                min: Vec3::from_array(config.bmin),
                max: Vec3::from_array(config.bmax),
            },
            walkable_slope_angle: config.walkable_slope_angle.to_radians(),
            walkable_height: config.walkable_height,
            walkable_climb: config.walkable_climb,
            walkable_radius: config.walkable_radius,
            max_edge_len: config.max_edge_len,
            max_simplification_error: config.max_simplification_error,
            min_region_area: config.min_region_area,
            merge_region_area: config.merge_region_area,
            max_vertices_per_polygon: config.max_verts_per_poly,
            detail_sample_dist: config.detail_sample_dist,
            detail_sample_max_error: config.detail_sample_max_error,
            area_volumes: Vec::new(),
            contour_flags: BuildContoursFlags::default(),
        }
    }
}

pub trait AssertEqTest {
    fn assert_eq(&self, project: &str, reference_name: &str);
}

impl AssertEqTest for Heightfield {
    fn assert_eq(&self, project: &str, reference_name: &str) {
        let heightfield = load_json::<CppHeightfield>(project, reference_name);

        assert_eq!(
            self.width, heightfield.width,
            "{project}/{reference_name}: heightfield width"
        );
        assert_eq!(
            self.height, heightfield.height,
            "{project}/{reference_name}: heightfield height"
        );
        assert_eq!(
            self.aabb.min,
            Vec3::from(heightfield.bmin),
            "{project}/{reference_name}: heightfield aabb min"
        );
        assert_eq!(
            self.aabb.max,
            Vec3::from(heightfield.bmax),
            "{project}/{reference_name}: heightfield aabb max"
        );
        assert_eq!(
            self.cell_size, heightfield.cs,
            "{project}/{reference_name}: heightfield cell size"
        );
        assert_eq!(
            self.cell_height, heightfield.ch,
            "{project}/{reference_name}: heightfield cell height"
        );
        assert_eq!(
            self.spans.len(),
            heightfield.spans.len(),
            "{project}/{reference_name}: heightfield spans length"
        );

        assert_eq!(self.spans.len(), self.width as usize * self.height as usize);
        assert_eq!(
            heightfield.spans.len(),
            heightfield.width as usize * heightfield.height as usize
        );

        for x in 0..self.width {
            for z in 0..self.height {
                let column_index = x as usize + z as usize * self.width as usize;
                let cpp_span = heightfield.spans[column_index].clone();
                let span_key = self.span_key_at(x, z);
                if let EmptyOption::Some(mut cpp_span) = cpp_span {
                    let mut layer = 0;
                    let mut span_key = span_key.unwrap_or_else(|| {
                    panic!("{project}/{reference_name}: C++ has a base span at [{x}, {z}] but Rust does not")
                });
                    loop {
                        let span = self.allocated_spans[span_key].clone();
                        assert_eq!(
                            span.min, cpp_span.min,
                            "{project}/{reference_name}: [{x}, {z}, {layer}] span min"
                        );
                        assert_eq!(
                            span.max, cpp_span.max,
                            "{project}/{reference_name}: [{x}, {z}, {layer}] span max"
                        );
                        let cpp_area = if cpp_span.area == 63 {
                            // We use u8::MAX currently, though this may change in the future.
                            AreaType::DEFAULT_WALKABLE
                        } else {
                            AreaType::from(cpp_span.area)
                        };
                        assert_eq!(
                            span.area, cpp_area,
                            "{project}/{reference_name}: [{x}, {z}, {layer}] span area"
                        );
                        if let EmptyOption::Some(next) = cpp_span.next {
                            span_key = span.next.unwrap();
                            cpp_span = *next;
                        } else {
                            assert!(span.next.is_none());
                            break;
                        }
                        layer += 1;
                    }
                } else {
                    assert!(
                        span_key.is_none(),
                        "{project}/{reference_name}: C++ has no base span at [{x}, {z}] but Rust does"
                    );
                }
            }
        }
    }
}

impl AssertEqTest for CompactHeightfield {
    fn assert_eq(&self, project: &str, reference_name: &str) {
        let heightfield = load_json::<CppCompactHeightfield>(project, reference_name);

        assert_eq!(
            self.width, heightfield.width,
            "{project}/{reference_name}: compact_heightfield width"
        );
        assert_eq!(
            self.height, heightfield.height,
            "{project}/{reference_name}: compact_heightfield height"
        );
        assert_eq!(
            self.walkable_height, heightfield.walkable_height,
            "{project}/{reference_name}: compact_heightfield walkable height"
        );
        assert_eq!(
            self.walkable_climb, heightfield.walkable_climb,
            "{project}/{reference_name}: compact_heightfield walkable climb"
        );
        assert_eq!(
            self.border_size, heightfield.border_size,
            "{project}/{reference_name}: compact_heightfield border size"
        );
        assert_eq!(
            self.max_region.bits(),
            heightfield.max_regions,
            "{project}/{reference_name}: compact_heightfield max region"
        );
        assert_eq!(
            self.max_distance, heightfield.max_distance,
            "{project}/{reference_name}: compact_heightfield max distance"
        );
        assert_eq!(
            self.aabb.min,
            Vec3::from(heightfield.bmin),
            "{project}/{reference_name}: compact_heightfield aabb min"
        );
        assert_eq!(
            self.aabb.max,
            Vec3::from(heightfield.bmax),
            "{project}/{reference_name}: compact_heightfield aabb max"
        );
        assert_eq!(
            self.cell_size, heightfield.cs,
            "{project}/{reference_name}: compact_heightfield cell size"
        );
        assert_eq!(
            self.cell_height, heightfield.ch,
            "{project}/{reference_name}: compact_heightfield cell height"
        );
        assert_eq!(
            self.cells.len(),
            heightfield.cells.len(),
            "{project}/{reference_name}: compact_heightfield cells length"
        );
        assert_eq!(
            self.spans.len(),
            heightfield.spans.len(),
            "{project}/{reference_name}: compact_heightfield spans length"
        );
        assert_eq!(
            self.dist.len(),
            heightfield.dist.len(),
            "{project}/{reference_name}: compact_heightfield dist length"
        );
        assert_eq!(
            self.areas.len(),
            heightfield.areas.len(),
            "{project}/{reference_name}: compact_heightfield areas length"
        );

        assert_eq!(self.cells.len(), self.width as usize * self.height as usize);
        assert_eq!(
            heightfield.cells.len(),
            heightfield.width as usize * heightfield.height as usize
        );

        for (i, (cell, cpp_cell)) in self.cells.iter().zip(heightfield.cells.iter()).enumerate() {
            assert_eq!(
                cell.index(),
                cpp_cell.index,
                "{project}/{reference_name}: compact_heightfield cell index {i}"
            );
            assert_eq!(
                cell.count(),
                cpp_cell.count,
                "{project}/{reference_name}: compact_heightfield cell count {i}"
            );
        }

        for (i, (span, cpp_span)) in self.spans.iter().zip(heightfield.spans.iter()).enumerate() {
            assert_eq!(
                span.y, cpp_span.y,
                "{project}/{reference_name}: compact_heightfield span y {i}"
            );
            assert_eq!(
                span.region,
                RegionId::from(cpp_span.reg),
                "{project}/{reference_name}: compact_heightfield span reg {i}"
            );
            let first_24_bits = span.data & 0x00FF_FFFF;
            assert_eq!(
                first_24_bits, cpp_span.con,
                "{project}/{reference_name}: compact_heightfield span con {i}"
            );
            assert_eq!(
                span.height(),
                cpp_span.h,
                "{project}/{reference_name}: compact_heightfield span height {i}"
            );
        }

        for (i, (dist, cpp_dist)) in self.dist.iter().zip(heightfield.dist.iter()).enumerate() {
            assert_eq!(
                *dist, *cpp_dist,
                "{project}/{reference_name}: compact_heightfield dist {i}"
            );
        }

        for (i, (area, cpp_area)) in self.areas.iter().zip(heightfield.areas.iter()).enumerate() {
            let cpp_area = if *cpp_area == 63 {
                AreaType::DEFAULT_WALKABLE
            } else {
                AreaType::from(*cpp_area)
            };
            assert_eq!(
                *area, cpp_area,
                "{project}/{reference_name}: compact_heightfield area {i}"
            );
        }
    }
}

impl AssertEqTest for ContourSet {
    fn assert_eq(&self, project: &str, reference_name: &str) {
        let contours = load_json::<CppContourSet>(project, reference_name);
        assert_almost_eq!(
            contours.bmin[..],
            self.aabb.min.to_array(),
            "{project}/{reference_name}: contour aabb min"
        );
        assert_almost_eq!(
            contours.bmax[..],
            self.aabb.max.to_array(),
            "{project}/{reference_name}: contour aabb max"
        );
        assert_almost_eq!(
            contours.cs,
            self.cell_size,
            "{project}/{reference_name}: contour cell size"
        );
        assert_almost_eq!(
            contours.ch,
            self.cell_height,
            "{project}/{reference_name}: contour cell height"
        );
        assert_eq!(
            contours.width, self.width,
            "{project}/{reference_name}: contour width"
        );
        assert_eq!(
            contours.height, self.height,
            "{project}/{reference_name}: contour height"
        );
        assert_eq!(
            contours.border_size, self.border_size,
            "{project}/{reference_name}: contour border size"
        );
        assert_almost_eq!(
            contours.max_error,
            self.max_error,
            "{project}/{reference_name}: contour max error"
        );
        assert_eq!(
            contours.contours.len(),
            self.contours.len(),
            "{project}/{reference_name}: contour count"
        );
        for (i, (cpp_contour, contour)) in contours
            .contours
            .iter()
            .zip(self.contours.iter())
            .enumerate()
        {
            assert_eq!(
                cpp_contour.reg,
                contour.region.bits(),
                "{project}/{reference_name}: contour {i} region id"
            );
            let cpp_area = if cpp_contour.area == 63 {
                AreaType::DEFAULT_WALKABLE
            } else {
                AreaType::from(cpp_contour.area)
            };
            assert_eq!(
                cpp_area, contour.area,
                "{project}/{reference_name}: contour {i} region area"
            );
            assert_eq!(
                cpp_contour.verts.len(),
                contour.vertices.len(),
                "{project}/{reference_name}: contour {i} vertex count"
            );
            assert_eq!(
                cpp_contour.rverts.len(),
                contour.raw_vertices.len(),
                "{project}/{reference_name}: contour {i} raw vertex count"
            );
            for (cpp_vert, (coord, data)) in cpp_contour.verts.iter().zip(contour.vertices.iter()) {
                let cpp_coords = &cpp_vert[..3];
                assert_eq!(
                    cpp_coords,
                    coord.as_uvec3().to_array(),
                    "{project}/{reference_name}: contour {i} vertex coordinates"
                );
                assert_eq!(
                    cpp_vert[3], *data,
                    "{project}/{reference_name}: contour {i} vertex data"
                );
            }
            for (cpp_vert, (coord, data)) in
                cpp_contour.rverts.iter().zip(contour.raw_vertices.iter())
            {
                let cpp_coords = &cpp_vert[..3];
                assert_eq!(
                    cpp_coords,
                    coord.as_uvec3().to_array(),
                    "{project}/{reference_name}: contour {i} raw vertex coordinates"
                );
                assert_eq!(
                    cpp_vert[3],
                    data.bits(),
                    "{project}/{reference_name}: contour {i} raw vertex data"
                );
            }
        }
    }
}

impl AssertEqTest for PolygonNavmesh {
    fn assert_eq(&self, project: &str, reference_name: &str) {
        let poly_mesh = load_json::<CppPolyMesh>(project, reference_name);
        assert_almost_eq!(
            poly_mesh.bmin[..],
            self.aabb.min.to_array(),
            "{project}/{reference_name}: poly mesh aabb min"
        );
        assert_almost_eq!(
            poly_mesh.bmax[..],
            self.aabb.max.to_array(),
            "{project}/{reference_name}: poly mesh aabb max"
        );

        assert_almost_eq!(
            poly_mesh.cs,
            self.cell_size,
            "{project}/{reference_name}: poly mesh cell size"
        );

        assert_almost_eq!(
            poly_mesh.ch,
            self.cell_height,
            "{project}/{reference_name}: poly mesh cell height"
        );

        assert_eq!(
            poly_mesh.nvp, self.max_vertices_per_polygon,
            "{project}/{reference_name}: poly mesh vertices per polygon"
        );

        assert_eq!(
            poly_mesh.border_size, self.border_size,
            "{project}/{reference_name}: poly mesh border_size"
        );
        assert_almost_eq!(
            poly_mesh.max_edge_error,
            self.max_edge_error,
            "{project}/{reference_name}: poly mesh max_edge_error"
        );
        assert_eq!(
            poly_mesh.verts.len(),
            self.vertices.len(),
            "{project}/{reference_name}: poly mesh verts len"
        );
        for (i, (cpp_vert, vert)) in poly_mesh.verts.iter().zip(self.vertices.iter()).enumerate() {
            assert_eq!(
                cpp_vert,
                &vert.to_array(),
                "{project}/{reference_name}: {i} poly mesh vertices"
            );
        }
        assert_eq!(
            poly_mesh.polys.len() / 2,
            self.polygons.len(),
            "{project}/{reference_name}: poly mesh polygons len"
        );
        assert_eq!(
            poly_mesh.polys.len() / 2,
            self.polygon_neighbors.len(),
            "{project}/{reference_name}: poly mesh polygons len"
        );
        let mut cpp_polys = Vec::new();
        let mut cpp_neighbors = Vec::new();
        for verts in poly_mesh.polys.chunks_exact(poly_mesh.nvp as usize * 2) {
            let (verts, neighbors) = verts.split_at(poly_mesh.nvp as usize);
            cpp_polys.extend_from_slice(verts);
            cpp_neighbors.extend_from_slice(neighbors);
        }
        for (i, (cpp_poly, poly)) in cpp_polys.iter().zip(self.polygons.iter()).enumerate() {
            assert_eq!(
                cpp_poly, poly,
                "{project}/{reference_name}: {i} poly mesh polygon"
            );
        }

        for (i, (cpp_neighbor, neighbor)) in cpp_neighbors
            .iter()
            .zip(self.polygon_neighbors.iter())
            .enumerate()
        {
            assert_eq!(
                cpp_neighbor, neighbor,
                "{project}/{reference_name}: {i} poly mesh polygon neighbor"
            );
        }
        assert_eq!(
            poly_mesh.flags.len(),
            self.flags.len(),
            "{project}/{reference_name}: poly mesh flags len"
        );
        for (i, (cpp_area, area)) in poly_mesh.areas.iter().zip(self.areas.iter()).enumerate() {
            let cpp_area = if *cpp_area == 63 {
                // We use u8::MAX currently, though this may change in the future.
                AreaType::DEFAULT_WALKABLE
            } else {
                AreaType::from(*cpp_area)
            };
            assert_eq!(
                cpp_area, *area,
                "{project}/{reference_name}: {i} poly mesh area"
            );
        }
        assert_eq!(
            poly_mesh.areas.len(),
            self.areas.len(),
            "{project}/{reference_name}: poly mesh areas len"
        );
        for (i, (cpp_flag, flag)) in poly_mesh.flags.iter().zip(self.flags.iter()).enumerate() {
            assert_eq!(
                cpp_flag, flag,
                "{project}/{reference_name}: {i} poly mesh flag"
            );
        }
    }
}

impl AssertEqTest for DetailNavmesh {
    fn assert_eq(&self, project: &str, reference_name: &str) {
        let detail_mesh = load_json::<CppDetailPolyMesh>(project, reference_name);

        assert_eq!(
            detail_mesh.meshes.len(),
            self.meshes.len(),
            "{project}/{reference_name}: detail mesh meshes len"
        );
        for (i, (cpp_mesh, mesh)) in detail_mesh
            .meshes
            .iter()
            .zip(self.meshes.iter())
            .enumerate()
        {
            assert_eq!(
                cpp_mesh[0] as u32, mesh.base_vertex_index,
                "{project}/{reference_name}: {i} detail mesh first vertex index"
            );
            assert_eq!(
                cpp_mesh[1] as u32, mesh.vertex_count,
                "{project}/{reference_name}: {i} detail mesh vertex_count"
            );
            assert_eq!(
                cpp_mesh[2] as u32, mesh.base_triangle_index,
                "{project}/{reference_name}: {i} detail mesh first triangle index"
            );
            assert_eq!(
                cpp_mesh[3] as u32, mesh.triangle_count,
                "{project}/{reference_name}: {i} detail mesh triangle_count"
            );
        }

        assert_eq!(
            detail_mesh.tris.len(),
            self.triangles.len(),
            "{project}/{reference_name}: detail mesh triangles len"
        );
        for (i, ((cpp_tri, tri), flags)) in detail_mesh
            .tris
            .iter()
            .zip(self.triangles.iter())
            .zip(self.triangle_flags.iter())
            .enumerate()
        {
            let cpp_tri_without_data = U8Vec3::from_slice(&cpp_tri[..3]);
            assert_eq!(
                cpp_tri_without_data,
                U8Vec3::from_array(*tri),
                "{project}/{reference_name}: {i} detail mesh triangle"
            );
            assert_eq!(
                cpp_tri[3], *flags,
                "{project}/{reference_name}: {i} detail mesh triangle data"
            );
        }

        assert_eq!(
            detail_mesh.verts.len(),
            self.vertices.len(),
            "{project}/{reference_name}: detail mesh vertices len"
        );
        for (i, (cpp_vert, vert)) in detail_mesh
            .verts
            .iter()
            .zip(self.vertices.iter())
            .enumerate()
        {
            // the jitter functions are sliiiiiightly different in Rust and C++
            assert!(
                vert.distance(Vec3::from_array(*cpp_vert)) < 1.0e-5,
                "{project}/{reference_name}: {cpp_vert:?} != {vert} failed: {i} detail mesh vertex"
            );
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppHeightfield {
    pub width: u16,
    pub height: u16,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
    pub cs: f32,
    pub ch: f32,
    pub spans: Vec<EmptyOption<CppSpan>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppSpan {
    pub min: u16,
    pub max: u16,
    pub area: u8,
    pub next: EmptyOption<Box<CppSpan>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppCompactHeightfield {
    pub width: u16,
    pub height: u16,
    #[serde(rename = "walkableHeight")]
    pub walkable_height: u16,
    #[serde(rename = "walkableClimb")]
    pub walkable_climb: u16,
    #[serde(rename = "borderSize")]
    pub border_size: u16,
    #[serde(rename = "maxDistance")]
    pub max_distance: u16,
    #[serde(rename = "maxRegions")]
    pub max_regions: u16,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
    pub cs: f32,
    pub ch: f32,
    pub cells: Vec<CppCompactCell>,
    pub spans: Vec<CppCompactSpan>,
    pub dist: Vec<u16>,
    pub areas: Vec<u8>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppCompactCell {
    pub index: u32,
    pub count: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppCompactSpan {
    pub y: u16,
    pub reg: u16,
    pub con: u32,
    pub h: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppVolumes {
    pub volumes: Vec<CppVolumeArea>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppVolumeArea {
    pub verts: Vec<[f32; 3]>,
    pub hmin: f32,
    pub hmax: f32,
    pub area: u8,
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

#[derive(Debug, Deserialize, Clone)]
pub struct CppContourSet {
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
    pub cs: f32,
    pub ch: f32,
    pub width: u16,
    pub height: u16,
    #[serde(rename = "borderSize")]
    pub border_size: u16,
    #[serde(rename = "maxError")]
    pub max_error: f32,
    pub contours: Vec<CppContour>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppContour {
    pub reg: u16,
    pub area: u8,
    pub verts: Vec<[u32; 4]>,
    pub rverts: Vec<[u32; 4]>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppPolyMesh {
    pub verts: Vec<[u16; 3]>,
    pub polys: Vec<u16>,
    pub flags: Vec<u16>,
    pub areas: Vec<u8>,
    pub nvp: u16,
    pub cs: f32,
    pub ch: f32,
    #[serde(rename = "borderSize")]
    pub border_size: u16,
    #[serde(rename = "maxEdgeError")]
    pub max_edge_error: f32,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppDetailPolyMesh {
    pub meshes: Vec<[u16; 4]>,
    pub tris: Vec<[u8; 4]>,
    pub verts: Vec<[f32; 3]>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppGeometry {
    pub verts: Vec<[f32; 3]>,
    pub tris: Vec<[u32; 3]>,
}

impl CppGeometry {
    pub fn to_trimesh(&self) -> TriMesh {
        TriMesh {
            vertices: self.verts.iter().map(|v| Vec3A::from(*v)).collect(),
            indices: self.tris.iter().map(|i| UVec3::from(*i)).collect(),
            area_types: vec![AreaType::NOT_WALKABLE; self.tris.len()],
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CppConfig {
    pub width: u16,
    pub height: u16,
    #[serde(rename = "tileSize")]
    pub tile_size: u16,
    #[serde(rename = "borderSize")]
    pub border_size: u16,
    pub cs: f32,
    pub ch: f32,
    pub bmin: [f32; 3],
    pub bmax: [f32; 3],
    #[serde(rename = "walkableSlopeAngle")]
    pub walkable_slope_angle: f32,
    #[serde(rename = "walkableHeight")]
    pub walkable_height: u16,
    #[serde(rename = "walkableClimb")]
    pub walkable_climb: u16,
    #[serde(rename = "walkableRadius")]
    pub walkable_radius: u16,
    #[serde(rename = "maxEdgeLen")]
    pub max_edge_len: u16,
    #[serde(rename = "maxSimplificationError")]
    pub max_simplification_error: f32,
    #[serde(rename = "minRegionArea")]
    pub min_region_area: u16,
    #[serde(rename = "mergeRegionArea")]
    pub merge_region_area: u16,
    #[serde(rename = "maxVertsPerPoly")]
    pub max_verts_per_poly: u16,
    #[serde(rename = "detailSampleDist")]
    pub detail_sample_dist: f32,
    #[serde(rename = "detailSampleMaxError")]
    pub detail_sample_max_error: f32,
}

pub fn test_data_dir() -> PathBuf {
    env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("assets")
        .join("test")
}

#[track_caller]
pub fn load_json<T: DeserializeOwned>(project: &str, name: &str) -> T {
    let test_path = test_data_dir().join(project).join(format!("{name}.json"));

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

macro_rules! assert_almost_eq {
    ($left:expr, $right:expr, $($arg:tt)+) => {
        if !(relative_eq!($left, $right, epsilon = 1e-6)) {
            assert_eq!($left, $right, $($arg)+);
        }
    };
}
use assert_almost_eq;
