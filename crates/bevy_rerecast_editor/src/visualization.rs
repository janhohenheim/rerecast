use std::iter;

use bevy::{color::palettes::tailwind, prelude::*};
use bevy_rerecast::rerecast::{DetailPolygonMesh, PolygonMesh, RC_MESH_NULL_IDX};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_gizmo);
    app.add_systems(
        Update,
        draw_navmesh.run_if(resource_exists_and_changed::<Navmesh>),
    );
}

#[derive(Resource)]
pub(crate) struct Navmesh {
    pub(crate) poly_mesh: PolygonMesh,
    pub(crate) detail_mesh: DetailPolygonMesh,
}

#[derive(Component)]
struct NavmeshGizmo;

fn spawn_gizmo(mut gizmos: ResMut<Assets<GizmoAsset>>, mut commands: Commands) {
    commands.spawn((
        NavmeshGizmo,
        Gizmo {
            handle: gizmos.add(GizmoAsset::new()),
            depth_bias: -0.001,
            ..default()
        },
    ));
}

fn draw_navmesh(
    gizmo: Single<&Gizmo, With<NavmeshGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    navmesh: Res<Navmesh>,
) {
    let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
        error!("Failed to get gizmo asset");
        return;
    };

    gizmo.clear();
    let mesh = &navmesh.poly_mesh;
    let nvp = mesh.vertices_per_polygon;
    let origin = mesh.aabb.min;
    let to_local = vec3a(mesh.cell_size, mesh.cell_height, mesh.cell_size);
    for i in 0..mesh.polygon_count() {
        let poly = &mesh.polygons[i * 2 * nvp..];
        let a_range = &poly[..nvp];
        let b_range = poly[1..nvp].iter().chain(iter::once(&poly[0]));
        for (&a_index, &b_index) in a_range.iter().zip(b_range) {
            if a_index == RC_MESH_NULL_IDX || b_index == RC_MESH_NULL_IDX {
                continue;
            }

            let a_vert_local = mesh.vertices[a_index as usize];
            let b_vert_local = mesh.vertices[b_index as usize];

            let a = origin + a_vert_local.as_vec3a() * to_local;
            let b = origin + b_vert_local.as_vec3a() * to_local;
            gizmo.line(a.into(), b.into(), tailwind::SKY_700);
        }
    }
}
