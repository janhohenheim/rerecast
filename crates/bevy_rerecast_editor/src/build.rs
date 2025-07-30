use anyhow::Context;
use bevy::prelude::*;
use bevy_rerecast::{
    prelude::*,
    rerecast::{BuildContoursFlags, DetailNavMesh, HeightfieldBuilder, TriMesh},
};

use crate::visualization::Navmesh;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(build_navmesh);
    app.init_resource::<BuildNavmeshConfig>();
}

#[derive(Event)]
pub(crate) struct BuildNavmesh;

#[derive(Resource)]
pub(crate) struct BuildNavmeshConfig {
    pub(crate) cell_size: f32,
    pub(crate) cell_height: f32,
    pub(crate) walkable_slope: f32,
    pub(crate) walkable_height: u16,
    pub(crate) walkable_climb: u16,
    pub(crate) walkable_radius: u16,
    pub(crate) min_region_area: u16,
    pub(crate) merge_region_area: u16,
    pub(crate) border_size: u16,
    pub(crate) max_simplification_error: f32,
    pub(crate) max_edge_len: u16,
    pub(crate) max_vertices_per_polygon: usize,
    pub(crate) contour_flags: BuildContoursFlags,
    pub(crate) detail_sample_dist: f32,
    pub(crate) detail_sample_max_error: f32,
}

impl Default for BuildNavmeshConfig {
    fn default() -> Self {
        Self {
            cell_size: 0.3,
            cell_height: 0.2,
            walkable_slope: 45.0_f32.to_radians(),
            walkable_height: 10,
            walkable_climb: 4,
            walkable_radius: 2,
            min_region_area: 64,
            merge_region_area: 400,
            border_size: 5,
            max_simplification_error: 1.3,
            max_edge_len: 40,
            max_vertices_per_polygon: 6,
            contour_flags: BuildContoursFlags::TESSELLATE_SOLID_WALL_EDGES,
            detail_sample_dist: 1.8,
            detail_sample_max_error: 0.2,
        }
    }
}

fn build_navmesh(
    _trigger: Trigger<BuildNavmesh>,
    affectors: Query<(&Mesh3d, &GlobalTransform), With<NavmeshAffector<Mesh3d>>>,
    meshes: Res<Assets<Mesh>>,
    config: Res<BuildNavmeshConfig>,
    mut commands: Commands,
) -> Result {
    let mut trimesh = TriMesh::default();
    for (mesh, transform) in affectors.iter() {
        let Some(mesh) = meshes.get(mesh) else {
            warn!("Failed to get mesh for navmesh build. Skipping.");
            continue;
        };
        let Some(mut current_trimesh) = TriMesh::from_mesh(mesh) else {
            warn!("Failed to convert collider to trimesh. Skipping.");
            continue;
        };
        let transform = transform.compute_transform();
        for vertex in &mut current_trimesh.vertices {
            *vertex = transform.transform_point(Vec3::from(*vertex)).into();
        }
        trimesh.extend(current_trimesh);
    }

    let aabb = trimesh.compute_aabb().context("Trimesh is empty")?;

    trimesh.mark_walkable_triangles(config.walkable_slope);

    let mut heightfield = HeightfieldBuilder {
        aabb,
        cell_size: config.cell_size,
        cell_height: config.cell_height,
    }
    .build()?;

    heightfield.rasterize_triangles(&trimesh, config.walkable_climb)?;

    // Once all geometry is rasterized, we do initial pass of filtering to
    // remove unwanted overhangs caused by the conservative rasterization
    // as well as filter spans where the character cannot possibly stand.
    heightfield.filter_low_hanging_walkable_obstacles(config.walkable_climb);
    heightfield.filter_ledge_spans(config.walkable_height, config.walkable_climb);
    heightfield.filter_walkable_low_height_spans(config.walkable_height);

    let mut compact_heightfield =
        heightfield.into_compact(config.walkable_height, config.walkable_climb)?;

    compact_heightfield.erode_walkable_area(config.walkable_radius);

    /*
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
    */

    compact_heightfield.build_distance_field();

    compact_heightfield.build_regions(
        config.border_size,
        config.min_region_area,
        config.merge_region_area,
    )?;

    let contours = compact_heightfield.build_contours(
        config.max_simplification_error,
        config.max_edge_len,
        config.contour_flags,
    );

    let poly_mesh = contours.into_polygon_mesh(config.max_vertices_per_polygon)?;

    let detail_mesh = DetailNavMesh::new(
        &poly_mesh,
        &compact_heightfield,
        config.detail_sample_dist,
        config.detail_sample_max_error,
    )?;

    commands.insert_resource(Navmesh {
        poly_mesh,
        detail_mesh,
    });

    Ok(())
}
