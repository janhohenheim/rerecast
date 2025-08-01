//! Compare the output of the C++ implementation with the Rust implementation.

use std::{env, fs, path::PathBuf};

use glam::{U8Vec3, UVec3, Vec2, Vec3, Vec3A};
use rerecast::{
    Aabb3d, AreaType, BuildContoursFlags, CompactHeightfield, ContourSet, ConvexVolume,
    DetailNavmesh, Heightfield, HeightfieldBuilder, NavmeshConfig, PolygonNavmesh, RegionId,
    TriMesh,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

#[test]
fn validate_navmesh_against_cpp_implementation() {
    for entry in fs::read_dir(reference_data_dir()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }
        let project = path.file_name().unwrap().to_str().unwrap();
        if project != "chainboom" {
            continue;
        }
        println!("Testing {project}...");

        let geometry = load_json::<CppGeometry>(project, "geometry");
        let mut trimesh = geometry.to_trimesh();
        let config = load_config(project);
        assert_eq!(
            config.aabb,
            trimesh.compute_aabb().unwrap(),
            "{project}: AABB mismatch"
        );

        trimesh.mark_walkable_triangles(config.walkable_slope_angle);

        let aabb = trimesh.compute_aabb().unwrap();

        let mut heightfield = HeightfieldBuilder {
            aabb,
            cell_size: config.cell_size,
            cell_height: config.cell_height,
        }
        .build()
        .unwrap();

        heightfield
            .rasterize_triangles(&trimesh, config.walkable_climb)
            .unwrap();
        assert_eq_heightfield(&heightfield, project, "heightfield_initial");

        // Once all geometry is rasterized, we do initial pass of filtering to
        // remove unwanted overhangs caused by the conservative rasterization
        // as well as filter spans where the character cannot possibly stand.
        heightfield.filter_low_hanging_walkable_obstacles(config.walkable_climb);
        heightfield.filter_ledge_spans(config.walkable_height, config.walkable_climb);
        heightfield.filter_walkable_low_height_spans(config.walkable_height);

        assert_eq_heightfield(&heightfield, project, "heightfield_filtered");

        let mut compact_heightfield = heightfield
            .into_compact(config.walkable_height, config.walkable_climb)
            .unwrap();

        assert_eq_compact_heightfield(&compact_heightfield, project, "compact_heightfield_initial");

        compact_heightfield.erode_walkable_area(config.walkable_radius);
        assert_eq_compact_heightfield(&compact_heightfield, project, "compact_heightfield_eroded");

        let volumes = load_json::<CppVolumes>(project, "convex_volumes");
        for volume in volumes.volumes {
            let volume = ConvexVolume {
                vertices: volume
                    .verts
                    .iter()
                    .map(|[x, _y, z]| Vec2::new(*x, *z))
                    .collect(),
                min_y: volume.hmin,
                max_y: volume.hmax,
                area: AreaType::from(volume.area),
            };
            compact_heightfield.mark_convex_poly_area(volume);
        }

        compact_heightfield.build_distance_field();
        assert_eq_compact_heightfield(
            &compact_heightfield,
            project,
            "compact_heightfield_distance_field",
        );

        compact_heightfield
            .build_regions(
                config.border_size,
                config.min_region_area,
                config.merge_region_area,
            )
            .unwrap();
        assert_eq_compact_heightfield(&compact_heightfield, project, "compact_heightfield_regions");

        let contours = compact_heightfield.build_contours(
            config.max_simplification_error,
            config.max_edge_len,
            config.contour_flags,
        );
        assert_eq_compact_heightfield(
            &compact_heightfield,
            project,
            "compact_heightfield_contours",
        );
        assert_eq_contours(&contours, project, "contour_set");

        let poly_mesh = contours
            .into_polygon_mesh(config.max_vertices_per_polygon)
            .unwrap();
        assert_eq_poly_mesh(&poly_mesh, project, "poly_mesh");

        let detail_mesh = DetailNavmesh::new(
            &poly_mesh,
            &compact_heightfield,
            config.detail_sample_dist,
            config.detail_sample_max_error,
        )
        .unwrap();
        assert_eq_detail_mesh(&detail_mesh, project, "poly_mesh_detail");
        println!("passed!\n")
    }
}

fn load_config(project: &str) -> NavmeshConfig {
    let config = load_json::<CppConfig>(project, "config");
    NavmeshConfig {
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
        contour_flags: BuildContoursFlags::default(),
    }
}

fn assert_eq_heightfield(heightfield: &Heightfield, project: &str, reference_name: &str) {
    let cpp_heightfield = load_json::<CppHeightfield>(project, reference_name);

    assert_eq!(
        heightfield.width, cpp_heightfield.width,
        "{project}/{reference_name}: heightfield width"
    );
    assert_eq!(
        heightfield.height, cpp_heightfield.height,
        "{project}/{reference_name}: heightfield height"
    );
    assert_eq!(
        heightfield.aabb.min,
        Vec3::from(cpp_heightfield.bmin),
        "{project}/{reference_name}: heightfield aabb min"
    );
    assert_eq!(
        heightfield.aabb.max,
        Vec3::from(cpp_heightfield.bmax),
        "{project}/{reference_name}: heightfield aabb max"
    );
    assert_eq!(
        heightfield.cell_size, cpp_heightfield.cs,
        "{project}/{reference_name}: heightfield cell size"
    );
    assert_eq!(
        heightfield.cell_height, cpp_heightfield.ch,
        "{project}/{reference_name}: heightfield cell height"
    );
    assert_eq!(
        heightfield.spans.len(),
        cpp_heightfield.spans.len(),
        "{project}/{reference_name}: heightfield spans length"
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
                    panic!("{project}/{reference_name}: C++ has a base span at [{x}, {z}] but Rust does not")
                });
                loop {
                    let span = heightfield.allocated_spans[span_key].clone();
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

fn assert_eq_compact_heightfield(
    compact_heightfield: &CompactHeightfield,
    project: &str,
    reference_name: &str,
) {
    let cpp_heightfield = load_json::<CppCompactHeightfield>(project, reference_name);

    assert_eq!(
        compact_heightfield.width, cpp_heightfield.width,
        "{project}/{reference_name}: compact_heightfield width"
    );
    assert_eq!(
        compact_heightfield.height, cpp_heightfield.height,
        "{project}/{reference_name}: compact_heightfield height"
    );
    assert_eq!(
        compact_heightfield.walkable_height, cpp_heightfield.walkable_height,
        "{project}/{reference_name}: compact_heightfield walkable height"
    );
    assert_eq!(
        compact_heightfield.walkable_climb, cpp_heightfield.walkable_climb,
        "{project}/{reference_name}: compact_heightfield walkable climb"
    );
    assert_eq!(
        compact_heightfield.border_size, cpp_heightfield.border_size,
        "{project}/{reference_name}: compact_heightfield border size"
    );
    assert_eq!(
        compact_heightfield.max_region.bits(),
        cpp_heightfield.max_regions,
        "{project}/{reference_name}: compact_heightfield max region"
    );
    assert_eq!(
        compact_heightfield.max_distance, cpp_heightfield.max_distance,
        "{project}/{reference_name}: compact_heightfield max distance"
    );
    assert_eq!(
        compact_heightfield.aabb.min,
        Vec3::from(cpp_heightfield.bmin),
        "{project}/{reference_name}: compact_heightfield aabb min"
    );
    assert_eq!(
        compact_heightfield.aabb.max,
        Vec3::from(cpp_heightfield.bmax),
        "{project}/{reference_name}: compact_heightfield aabb max"
    );
    assert_eq!(
        compact_heightfield.cell_size, cpp_heightfield.cs,
        "{project}/{reference_name}: compact_heightfield cell size"
    );
    assert_eq!(
        compact_heightfield.cell_height, cpp_heightfield.ch,
        "{project}/{reference_name}: compact_heightfield cell height"
    );
    assert_eq!(
        compact_heightfield.cells.len(),
        cpp_heightfield.cells.len(),
        "{project}/{reference_name}: compact_heightfield cells length"
    );
    assert_eq!(
        compact_heightfield.spans.len(),
        cpp_heightfield.spans.len(),
        "{project}/{reference_name}: compact_heightfield spans length"
    );
    assert_eq!(
        compact_heightfield.dist.len(),
        cpp_heightfield.dist.len(),
        "{project}/{reference_name}: compact_heightfield dist length"
    );
    assert_eq!(
        compact_heightfield.areas.len(),
        cpp_heightfield.areas.len(),
        "{project}/{reference_name}: compact_heightfield areas length"
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
            "{project}/{reference_name}: compact_heightfield cell index {i}"
        );
        assert_eq!(
            cell.count(),
            cpp_cell.count,
            "{project}/{reference_name}: compact_heightfield cell count {i}"
        );
    }

    for (i, (span, cpp_span)) in compact_heightfield
        .spans
        .iter()
        .zip(cpp_heightfield.spans.iter())
        .enumerate()
    {
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

    for (i, (dist, cpp_dist)) in compact_heightfield
        .dist
        .iter()
        .zip(cpp_heightfield.dist.iter())
        .enumerate()
    {
        assert_eq!(
            *dist, *cpp_dist,
            "{project}/{reference_name}: compact_heightfield dist {i}"
        );
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
        assert_eq!(
            *area, cpp_area,
            "{project}/{reference_name}: compact_heightfield area {i}"
        );
    }
}

fn assert_eq_contours(contours: &ContourSet, project: &str, reference_name: &str) {
    let cpp_contours = load_json::<CppContourSet>(project, reference_name);
    assert_eq!(
        cpp_contours.bmin,
        contours.aabb.min.to_array(),
        "{project}/{reference_name}: contour aabb min"
    );
    assert_eq!(
        cpp_contours.bmax,
        contours.aabb.max.to_array(),
        "{project}/{reference_name}: contour aabb max"
    );
    assert_eq!(
        cpp_contours.cs, contours.cell_size,
        "{project}/{reference_name}: contour cell size"
    );
    assert_eq!(
        cpp_contours.ch, contours.cell_height,
        "{project}/{reference_name}: contour cell height"
    );
    assert_eq!(
        cpp_contours.width, contours.width,
        "{project}/{reference_name}: contour width"
    );
    assert_eq!(
        cpp_contours.height, contours.height,
        "{project}/{reference_name}: contour height"
    );
    assert_eq!(
        cpp_contours.border_size, contours.border_size,
        "{project}/{reference_name}: contour border size"
    );
    assert_eq!(
        cpp_contours.max_error, contours.max_error,
        "{project}/{reference_name}: contour max error"
    );
    assert_eq!(
        cpp_contours.contours.len(),
        contours.contours.len(),
        "{project}/{reference_name}: contour count"
    );
    for (i, (cpp_contour, contour)) in cpp_contours
        .contours
        .iter()
        .zip(contours.contours.iter())
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
        for (cpp_vert, (coord, data)) in cpp_contour.rverts.iter().zip(contour.raw_vertices.iter())
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

fn assert_eq_poly_mesh(poly_mesh: &PolygonNavmesh, project: &str, reference_name: &str) {
    let cpp_poly_mesh = load_json::<CppPolyMesh>(project, reference_name);
    assert_eq!(
        cpp_poly_mesh.bmin,
        poly_mesh.aabb.min.to_array(),
        "{project}/{reference_name}: poly mesh aabb min"
    );

    assert_eq!(
        cpp_poly_mesh.bmax,
        poly_mesh.aabb.max.to_array(),
        "{project}/{reference_name}: poly mesh aabb max"
    );

    assert_eq!(
        cpp_poly_mesh.cs, poly_mesh.cell_size,
        "{project}/{reference_name}: poly mesh cell size"
    );

    assert_eq!(
        cpp_poly_mesh.ch, poly_mesh.cell_height,
        "{project}/{reference_name}: poly mesh cell height"
    );

    assert_eq!(
        cpp_poly_mesh.nvp, poly_mesh.max_vertices_per_polygon,
        "{project}/{reference_name}: poly mesh vertices per polygon"
    );

    assert_eq!(
        cpp_poly_mesh.border_size, poly_mesh.border_size,
        "{project}/{reference_name}: poly mesh border_size"
    );
    assert_eq!(
        cpp_poly_mesh.max_edge_error, poly_mesh.max_edge_error,
        "{project}/{reference_name}: poly mesh max_edge_error"
    );
    assert_eq!(
        cpp_poly_mesh.verts.len(),
        poly_mesh.vertices.len(),
        "{project}/{reference_name}: poly mesh verts len"
    );
    for (i, (cpp_vert, vert)) in cpp_poly_mesh
        .verts
        .iter()
        .zip(poly_mesh.vertices.iter())
        .enumerate()
    {
        assert_eq!(
            cpp_vert,
            &vert.to_array(),
            "{project}/{reference_name}: {i} poly mesh vertices"
        );
    }
    assert_eq!(
        cpp_poly_mesh.polys.len() / 2,
        poly_mesh.polygons.len(),
        "{project}/{reference_name}: poly mesh polygons len"
    );
    assert_eq!(
        cpp_poly_mesh.polys.len() / 2,
        poly_mesh.polygon_neighbors.len(),
        "{project}/{reference_name}: poly mesh polygons len"
    );
    let mut cpp_polys = Vec::new();
    let mut cpp_neighbors = Vec::new();
    for verts in cpp_poly_mesh
        .polys
        .chunks_exact(cpp_poly_mesh.nvp as usize * 2)
    {
        let (verts, neighbors) = verts.split_at(cpp_poly_mesh.nvp as usize);
        cpp_polys.extend_from_slice(verts);
        cpp_neighbors.extend_from_slice(neighbors);
    }
    for (i, (cpp_poly, poly)) in cpp_polys.iter().zip(poly_mesh.polygons.iter()).enumerate() {
        assert_eq!(
            cpp_poly, poly,
            "{project}/{reference_name}: {i} poly mesh polygon"
        );
    }

    for (i, (cpp_neighbor, neighbor)) in cpp_neighbors
        .iter()
        .zip(poly_mesh.polygon_neighbors.iter())
        .enumerate()
    {
        assert_eq!(
            cpp_neighbor, neighbor,
            "{project}/{reference_name}: {i} poly mesh polygon neighbor"
        );
    }
    assert_eq!(
        cpp_poly_mesh.flags.len(),
        poly_mesh.flags.len(),
        "{project}/{reference_name}: poly mesh flags len"
    );
    for (i, (cpp_area, area)) in cpp_poly_mesh
        .areas
        .iter()
        .zip(poly_mesh.areas.iter())
        .enumerate()
    {
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
        cpp_poly_mesh.areas.len(),
        poly_mesh.areas.len(),
        "{project}/{reference_name}: poly mesh areas len"
    );
    for (i, (cpp_flag, flag)) in cpp_poly_mesh
        .flags
        .iter()
        .zip(poly_mesh.flags.iter())
        .enumerate()
    {
        assert_eq!(
            cpp_flag, flag,
            "{project}/{reference_name}: {i} poly mesh flag"
        );
    }
}

fn assert_eq_detail_mesh(detail_mesh: &DetailNavmesh, project: &str, reference_name: &str) {
    let cpp_detail_mesh = load_json::<CppDetailPolyMesh>(project, reference_name);

    assert_eq!(
        cpp_detail_mesh.meshes.len(),
        detail_mesh.meshes.len(),
        "{project}/{reference_name}: detail mesh meshes len"
    );
    for (i, (cpp_mesh, mesh)) in cpp_detail_mesh
        .meshes
        .iter()
        .zip(detail_mesh.meshes.iter())
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
        cpp_detail_mesh.tris.len(),
        detail_mesh.triangles.len(),
        "{project}/{reference_name}: detail mesh triangles len"
    );
    for (i, ((cpp_tri, tri), flags)) in cpp_detail_mesh
        .tris
        .iter()
        .zip(detail_mesh.triangles.iter())
        .zip(detail_mesh.triangle_flags.iter())
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
        cpp_detail_mesh.verts.len(),
        detail_mesh.vertices.len(),
        "{project}/{reference_name}: detail mesh vertices len"
    );
    for (i, (cpp_vert, vert)) in cpp_detail_mesh
        .verts
        .iter()
        .zip(detail_mesh.vertices.iter())
        .enumerate()
    {
        // the jitter functions are sliiiiiightly different in Rust and C++
        assert!(
            vert.distance(Vec3::from_array(*cpp_vert)) < 1.0e-5,
            "{project}/{reference_name}: {cpp_vert:?} != {vert} failed: {i} detail mesh vertex"
        );
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

#[derive(Debug, Deserialize, Clone)]
struct CppVolumes {
    volumes: Vec<CppVolumeArea>,
}

#[derive(Debug, Deserialize, Clone)]
struct CppVolumeArea {
    verts: Vec<[f32; 3]>,
    hmin: f32,
    hmax: f32,
    area: u8,
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
struct CppContourSet {
    bmin: [f32; 3],
    bmax: [f32; 3],
    cs: f32,
    ch: f32,
    width: u16,
    height: u16,
    #[serde(rename = "borderSize")]
    border_size: u16,
    #[serde(rename = "maxError")]
    max_error: f32,
    contours: Vec<CppContour>,
}

#[derive(Debug, Deserialize, Clone)]
struct CppContour {
    reg: u16,
    area: u8,
    verts: Vec<[u32; 4]>,
    rverts: Vec<[u32; 4]>,
}

#[derive(Debug, Deserialize, Clone)]
struct CppPolyMesh {
    verts: Vec<[u16; 3]>,
    polys: Vec<u16>,
    flags: Vec<u16>,
    areas: Vec<u8>,
    nvp: u16,
    cs: f32,
    ch: f32,
    #[serde(rename = "borderSize")]
    border_size: u16,
    #[serde(rename = "maxEdgeError")]
    max_edge_error: f32,
    bmin: [f32; 3],
    bmax: [f32; 3],
}

#[derive(Debug, Deserialize, Clone)]
struct CppDetailPolyMesh {
    meshes: Vec<[u16; 4]>,
    tris: Vec<[u8; 4]>,
    verts: Vec<[f32; 3]>,
}

#[derive(Debug, Deserialize, Clone)]
struct CppGeometry {
    verts: Vec<[f32; 3]>,
    tris: Vec<[u32; 3]>,
}

impl CppGeometry {
    fn to_trimesh(&self) -> TriMesh {
        TriMesh {
            vertices: self.verts.iter().map(|v| Vec3A::from(*v)).collect(),
            indices: self.tris.iter().map(|i| UVec3::from(*i)).collect(),
            area_types: vec![AreaType::NOT_WALKABLE; self.tris.len()],
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct CppConfig {
    width: u16,
    height: u16,
    #[serde(rename = "tileSize")]
    tile_size: u16,
    #[serde(rename = "borderSize")]
    border_size: u16,
    cs: f32,
    ch: f32,
    bmin: [f32; 3],
    bmax: [f32; 3],
    #[serde(rename = "walkableSlopeAngle")]
    walkable_slope_angle: f32,
    #[serde(rename = "walkableHeight")]
    walkable_height: u16,
    #[serde(rename = "walkableClimb")]
    walkable_climb: u16,
    #[serde(rename = "walkableRadius")]
    walkable_radius: u16,
    #[serde(rename = "maxEdgeLen")]
    max_edge_len: u16,
    #[serde(rename = "maxSimplificationError")]
    max_simplification_error: f32,
    #[serde(rename = "minRegionArea")]
    min_region_area: u16,
    #[serde(rename = "mergeRegionArea")]
    merge_region_area: u16,
    #[serde(rename = "maxVertsPerPoly")]
    max_verts_per_poly: u16,
    #[serde(rename = "detailSampleDist")]
    detail_sample_dist: f32,
    #[serde(rename = "detailSampleMaxError")]
    detail_sample_max_error: f32,
}

fn reference_data_dir() -> PathBuf {
    env::current_dir()
        .unwrap()
        .join("tests")
        .join("reference_data")
}

#[track_caller]
fn load_json<T: DeserializeOwned>(project: &str, name: &str) -> T {
    let test_path = reference_data_dir()
        .join(project)
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
