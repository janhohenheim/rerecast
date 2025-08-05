//! Compare the output of the C++ implementation with the Rust implementation.

use std::fs;

use glam::Vec2;
use rerecast::{AreaType, Config, ConvexVolume, DetailNavmesh, HeightfieldBuilder};
use test_utils::*;

#[test]
fn validate_navmesh_against_cpp_implementation() {
    let dir = test_data_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => panic!("Failed to read directory {dir}: {err}", dir = dir.display()),
    };
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }
        let project = path.file_name().unwrap().to_str().unwrap();
        println!("Testing {project}...");
        if !path.join("geometry.json").exists() {
            println!("Skipping {path:?} because it is missing geometry.json");
            continue;
        }

        let geometry = load_json::<CppGeometry>(project, "geometry");
        let mut trimesh = geometry.to_trimesh();
        let config = Config::load_from_test_data(project);
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
        heightfield.assert_eq(project, "heightfield_initial");

        // Once all geometry is rasterized, we do initial pass of filtering to
        // remove unwanted overhangs caused by the conservative rasterization
        // as well as filter spans where the character cannot possibly stand.
        heightfield.filter_low_hanging_walkable_obstacles(config.walkable_climb);
        heightfield.filter_ledge_spans(config.walkable_height, config.walkable_climb);
        heightfield.filter_walkable_low_height_spans(config.walkable_height);

        heightfield.assert_eq(project, "heightfield_filtered");

        let mut compact_heightfield = heightfield
            .into_compact(config.walkable_height, config.walkable_climb)
            .unwrap();

        compact_heightfield.assert_eq(project, "compact_heightfield_initial");

        compact_heightfield.erode_walkable_area(config.walkable_radius);
        compact_heightfield.assert_eq(project, "compact_heightfield_eroded");

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
            compact_heightfield.mark_convex_poly_area(&volume);
        }

        compact_heightfield.build_distance_field();
        compact_heightfield.assert_eq(project, "compact_heightfield_distance_field");

        compact_heightfield
            .build_regions(
                config.border_size,
                config.min_region_area,
                config.merge_region_area,
            )
            .unwrap();
        compact_heightfield.assert_eq(project, "compact_heightfield_regions");

        let contours = compact_heightfield.build_contours(
            config.max_simplification_error,
            config.max_edge_len,
            config.contour_flags,
        );
        compact_heightfield.assert_eq(project, "compact_heightfield_contours");
        contours.assert_eq(project, "contour_set");

        let poly_mesh = contours
            .into_polygon_mesh(config.max_vertices_per_polygon)
            .unwrap();
        poly_mesh.assert_eq(project, "poly_mesh");

        let detail_mesh = DetailNavmesh::new(
            &poly_mesh,
            &compact_heightfield,
            config.detail_sample_dist,
            config.detail_sample_max_error,
        )
        .unwrap();
        detail_mesh.assert_eq(project, "poly_mesh_detail");
        println!("passed!\n")
    }
}
