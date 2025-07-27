use bevy::{color::palettes::tailwind, prelude::*};
use bevy_rerecast::{
    prelude::*,
    rerecast::{DetailPolygonMesh, PolygonMesh, RC_MESH_NULL_IDX, TriMesh},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_gizmos);
    app.add_systems(
        Update,
        (
            draw_poly_mesh.run_if(resource_exists_and_changed::<Navmesh>),
            draw_navmesh_affector.run_if(|| false),
        ),
    );
}

#[derive(Resource)]
pub(crate) struct Navmesh {
    pub(crate) poly_mesh: PolygonMesh,
    pub(crate) detail_mesh: DetailPolygonMesh,
}

#[derive(Component)]
struct NavmeshGizmo;

#[derive(Component)]
struct NavmeshAffectorGizmo;

fn spawn_gizmos(mut gizmos: ResMut<Assets<GizmoAsset>>, mut commands: Commands) {
    commands.spawn((
        NavmeshGizmo,
        Gizmo {
            handle: gizmos.add(GizmoAsset::new()),
            line_config: GizmoLineConfig {
                perspective: true,
                width: 10.0,
                ..default()
            },
            depth_bias: -0.001,
        },
    ));

    commands.spawn((
        NavmeshAffectorGizmo,
        Gizmo {
            handle: gizmos.add(GizmoAsset::new()),
            line_config: GizmoLineConfig {
                perspective: true,
                width: 10.0,
                ..default()
            },
            depth_bias: -0.001,
        },
    ));
}

fn draw_poly_mesh(
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
    let origin = Vec3::from(mesh.aabb.min);
    let to_local = vec3(mesh.cell_size, mesh.cell_height, mesh.cell_size);
    for i in 0..mesh.polygon_count() {
        let poly = &mesh.polygons[i * 2 * nvp..];
        let mut verts = poly[..nvp]
            .iter()
            .filter(|i| **i != RC_MESH_NULL_IDX)
            .map(|i| {
                let vert_local = mesh.vertices[*i as usize];

                origin + vert_local.as_vec3() * to_local
            })
            .collect::<Vec<_>>();
        // Connect back to first vertex to finish the polygon
        verts.push(verts[0]);

        gizmo.linestrip(verts, tailwind::SKY_700);
    }
}

fn draw_navmesh_affector(
    gizmo: Single<&Gizmo, With<NavmeshAffectorGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    affector: Single<&Mesh3d, (With<NavmeshAffector<Mesh3d>>, Changed<Mesh3d>)>,
    meshes: Res<Assets<Mesh>>,
) {
    let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
        error!("Failed to get gizmo asset");
        return;
    };
    let Some(mesh) = meshes.get(&affector.0) else {
        error!("Failed to get mesh asset");
        return;
    };

    gizmo.clear();
    let mesh = TriMesh::from_mesh(mesh).unwrap();
    for indices in mesh.indices {
        let mut verts = indices
            .to_array()
            .iter()
            .map(|i| Vec3::from(mesh.vertices[*i as usize]))
            .collect::<Vec<_>>();
        // Connect back to first vertex to finish the polygon
        verts.push(verts[0]);

        gizmo.linestrip(verts, tailwind::ORANGE_700);
    }
}
