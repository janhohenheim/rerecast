use std::collections::HashSet;

use bevy::{color::palettes::tailwind, prelude::*};
use bevy_rerecast::{TriMeshFromBevyMesh as _, debug::NavmeshGizmoConfig, rerecast::TriMesh};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<GizmosToDraw>();
    app.add_systems(
        Update,
        (
            draw_poly_mesh.run_if(toggled_gizmo_on(AvailableGizmos::PolyMesh)),
            draw_detail_mesh.run_if(toggled_gizmo_on(AvailableGizmos::DetailMesh)),
            draw_obstacles.run_if(toggled_gizmo_on(AvailableGizmos::Obstacles)),
            draw_visual.run_if(toggled_gizmo_on(AvailableGizmos::Visual)),
            hide_poly_mesh.run_if(toggled_gizmo_off(AvailableGizmos::PolyMesh)),
            hide_detail_mesh.run_if(toggled_gizmo_off(AvailableGizmos::DetailMesh)),
            hide_obstacles.run_if(toggled_gizmo_off(AvailableGizmos::Obstacles)),
            hide_visual.run_if(toggled_gizmo_off(AvailableGizmos::Visual)),
        ),
    );
}

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct GizmosToDraw(HashSet<AvailableGizmos>);

impl GizmosToDraw {
    pub(crate) fn set(&mut self, gizmo: AvailableGizmos, enabled: bool) {
        if enabled {
            self.insert(gizmo);
        } else {
            self.remove(&gizmo);
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) enum AvailableGizmos {
    Visual,
    Obstacles,
    PolyMesh,
    DetailMesh,
}

fn toggled_gizmo_on(gizmo: AvailableGizmos) -> impl SystemCondition<()> {
    IntoSystem::into_system(move |gizmos: Res<GizmosToDraw>| {
        gizmos.is_changed() && gizmos.contains(&gizmo)
    })
}

fn toggled_gizmo_off(gizmo: AvailableGizmos) -> impl SystemCondition<()> {
    IntoSystem::into_system(move |gizmos: Res<GizmosToDraw>| {
        gizmos.is_changed() && !gizmos.contains(&gizmo)
    })
}

impl Default for GizmosToDraw {
    fn default() -> Self {
        Self(
            vec![AvailableGizmos::DetailMesh, AvailableGizmos::Visual]
                .into_iter()
                .collect(),
        )
    }
}

fn draw_poly_mesh(mut config: ResMut<NavmeshGizmoConfig>) {
    config.polygon_navmesh.enabled = true;
}

fn draw_detail_mesh(mut config: ResMut<NavmeshGizmoConfig>) {
    config.detail_navmesh.enabled = true;
}

fn draw_obstacles(
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    obstacles: Query<(&Mesh3d, &Gizmo), With<ObstacleGizmo>>,
    meshes: Res<Assets<Mesh>>,
) {
    for (mesh, gizmo) in &obstacles {
        let Some(mut gizmo) = gizmos.get_mut(&gizmo.handle) else {
            error!("Failed to get gizmo asset");
            return;
        };
        let Some(mesh) = meshes.get(&mesh.0) else {
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
}

fn draw_visual(mut visibility: Query<&mut Visibility, With<VisualMesh>>) {
    for mut visibility in visibility.iter_mut() {
        *visibility = Visibility::Inherited;
    }
}

fn hide_obstacles(
    gizmo_handles: Query<&Gizmo, With<ObstacleGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
) {
    for gizmo in &gizmo_handles {
        let Some(mut gizmo) = gizmos.get_mut(&gizmo.handle) else {
            error!("Failed to get gizmo asset");
            return;
        };
        gizmo.clear();
    }
}

fn hide_visual(mut visibility: Query<&mut Visibility, With<VisualMesh>>) {
    for mut visibility in visibility.iter_mut() {
        *visibility = Visibility::Hidden;
    }
}

fn hide_poly_mesh(mut config: ResMut<NavmeshGizmoConfig>) {
    config.polygon_navmesh.enabled = false;
}

fn hide_detail_mesh(mut config: ResMut<NavmeshGizmoConfig>) {
    config.detail_navmesh.enabled = false;
}

#[derive(Component)]
pub(crate) struct VisualMesh;

#[derive(Component)]
pub(crate) struct ObstacleGizmo;
