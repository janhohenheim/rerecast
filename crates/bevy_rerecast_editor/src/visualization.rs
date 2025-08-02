use std::collections::HashSet;

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use bevy_rerecast::{
    TriMeshFromBevyMesh as _,
    rerecast::{DetailNavmesh, PolygonNavmesh, TriMesh},
};

use crate::build::NavmeshAffector;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_gizmos);
    app.init_resource::<GizmosToDraw>();
    app.add_systems(
        Update,
        (
            draw_poly_mesh.run_if(resource_exists::<Navmesh>.and(
                gizmo_enabled(AvailableGizmos::PolyMesh).and(
                    resource_changed::<Navmesh>.or(toggled_gizmo_on(AvailableGizmos::PolyMesh)),
                ),
            )),
            draw_detail_mesh.run_if(resource_exists::<Navmesh>.and(
                gizmo_enabled(AvailableGizmos::DetailMesh).and(
                    resource_changed::<Navmesh>.or(toggled_gizmo_on(AvailableGizmos::DetailMesh)),
                ),
            )),
            draw_navmesh_affector.run_if(toggled_gizmo_on(AvailableGizmos::Affector)),
            draw_visual.run_if(toggled_gizmo_on(AvailableGizmos::Visual)),
            hide_poly_mesh.run_if(toggled_gizmo_off(AvailableGizmos::PolyMesh)),
            hide_detail_mesh.run_if(toggled_gizmo_off(AvailableGizmos::DetailMesh)),
            hide_affector.run_if(toggled_gizmo_off(AvailableGizmos::Affector)),
            hide_visual.run_if(toggled_gizmo_off(AvailableGizmos::Visual)),
        ),
    );
}

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct GizmosToDraw(HashSet<AvailableGizmos>);

impl GizmosToDraw {
    pub(crate) fn toggle(&mut self, gizmo: AvailableGizmos) {
        if self.contains(&gizmo) {
            self.remove(&gizmo);
        } else {
            self.insert(gizmo);
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) enum AvailableGizmos {
    Visual,
    Affector,
    PolyMesh,
    DetailMesh,
}

fn toggled_gizmo_on(gizmo: AvailableGizmos) -> impl Condition<()> {
    IntoSystem::into_system(move |gizmos: Res<GizmosToDraw>| {
        gizmos.is_changed() && gizmos.contains(&gizmo)
    })
}

fn toggled_gizmo_off(gizmo: AvailableGizmos) -> impl Condition<()> {
    IntoSystem::into_system(move |gizmos: Res<GizmosToDraw>| {
        gizmos.is_changed() && !gizmos.contains(&gizmo)
    })
}

fn gizmo_enabled(gizmo: AvailableGizmos) -> impl Condition<()> {
    IntoSystem::into_system(move |gizmos: Res<GizmosToDraw>| gizmos.contains(&gizmo))
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

#[derive(Component)]
struct PolyMeshGizmo;

#[derive(Component)]
struct DetailMeshGizmo;

fn spawn_gizmos(mut gizmos: ResMut<Assets<GizmoAsset>>, mut commands: Commands) {
    commands.spawn((
        PolyMeshGizmo,
        Visibility::Hidden,
        Gizmo {
            handle: gizmos.add(GizmoAsset::new()),
            line_config: GizmoLineConfig {
                perspective: true,
                width: 20.0,
                ..default()
            },
            depth_bias: -0.001,
        },
    ));
    commands.spawn((
        DetailMeshGizmo,
        Visibility::Hidden,
        Gizmo {
            handle: gizmos.add(GizmoAsset::new()),
            line_config: GizmoLineConfig {
                perspective: true,
                width: 20.0,
                joints: GizmoLineJoint::Bevel,
                ..default()
            },
            depth_bias: -0.001,
        },
    ));
}

fn draw_navmesh_affector(
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    affector: Query<(&Mesh3d, &Gizmo), With<NavmeshAffector>>,
    meshes: Res<Assets<Mesh>>,
) {
    for (mesh, gizmo) in &affector {
        let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
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

fn hide_affector(
    gizmo_handles: Query<&Gizmo, With<NavmeshAffector>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
) {
    for gizmo in &gizmo_handles {
        let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
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

#[derive(Component)]
pub(crate) struct VisualMesh;
