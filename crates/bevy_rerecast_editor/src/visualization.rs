use std::collections::HashSet;

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use bevy_rerecast::{
    prelude::*,
    rerecast::{DetailPolygonMesh, PolygonMesh, RC_MESH_NULL_IDX, TriMesh},
};

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

#[derive(Resource)]
pub(crate) struct Navmesh {
    pub(crate) poly_mesh: PolygonMesh,
    pub(crate) detail_mesh: DetailPolygonMesh,
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

#[derive(Component)]
struct NavmeshAffectorGizmo;

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
    commands.spawn((
        NavmeshAffectorGizmo,
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

fn draw_poly_mesh(
    gizmo: Single<(Entity, &Gizmo, &mut Visibility), With<PolyMeshGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    navmesh: Res<Navmesh>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let (entity, gizmo, mut visibility) = gizmo.into_inner();
    let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
        error!("Failed to get gizmo asset");
        return;
    };

    gizmo.clear();
    *visibility = Visibility::Inherited;

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

    let mut visual_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    let mut visual_verts = Vec::new();
    let mut visual_indices = Vec::new();

    for i in 0..mesh.polygon_count() {
        let poly = &mesh.polygons[i * 2 * nvp..];
        let a = origin + mesh.vertices[poly[0] as usize].as_vec3() * to_local;
        let a_idx = visual_verts.len() as u32;
        visual_verts.push(a);

        // Fan triangulation
        for val in poly[1..nvp].windows(2) {
            let b = val[0];
            let c = val[1];
            if b == RC_MESH_NULL_IDX || c == RC_MESH_NULL_IDX {
                continue;
            }
            let b = origin + mesh.vertices[b as usize].as_vec3() * to_local;
            let c = origin + mesh.vertices[c as usize].as_vec3() * to_local;

            let b_vi = visual_verts.len() as u32;
            visual_verts.push(b);
            let c_vi = visual_verts.len() as u32;
            visual_verts.push(c);

            visual_indices.push(a_idx);
            visual_indices.push(b_vi);
            visual_indices.push(c_vi);
        }
    }
    visual_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, visual_verts);
    visual_mesh.insert_indices(Indices::U32(visual_indices));

    let standard_material = StandardMaterial {
        base_color: tailwind::BLUE_600.with_alpha(0.7).into(),
        unlit: true,
        double_sided: true,
        alpha_mode: AlphaMode::AlphaToCoverage,
        ..default()
    };

    commands.entity(entity).insert((
        Mesh3d(meshes.add(visual_mesh)),
        MeshMaterial3d(materials.add(standard_material)),
    ));
}

fn draw_detail_mesh(
    gizmo: Single<(Entity, &Gizmo, &mut Visibility), With<DetailMeshGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    navmesh: Res<Navmesh>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let (entity, gizmo, mut visibility) = gizmo.into_inner();
    let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
        error!("Failed to get gizmo asset");
        return;
    };

    *visibility = Visibility::Inherited;
    gizmo.clear();

    let mesh = &navmesh.detail_mesh;
    for submesh in &mesh.meshes {
        let submesh_verts = &mesh.vertices[submesh.first_vertex_index..][..submesh.vertex_count];
        let submesh_tris =
            &mesh.triangles[submesh.first_triangle_index..][..submesh.triangle_count];
        for (tri, _data) in submesh_tris {
            let mut verts = tri
                .to_array()
                .iter()
                .map(|i| Vec3::from(submesh_verts[*i as usize]))
                .collect::<Vec<_>>();
            // Connect back to first vertex to finish the polygon
            verts.push(verts[0]);

            gizmo.linestrip(verts, tailwind::GREEN_700);
        }
    }

    let mut visual_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    let mut visual_verts = Vec::new();
    let mut visual_indices = Vec::new();

    for submesh in &mesh.meshes {
        let submesh_verts = &mesh.vertices[submesh.first_vertex_index..][..submesh.vertex_count];

        let submesh_tris =
            &mesh.triangles[submesh.first_triangle_index..][..submesh.triangle_count];
        for (tri, _data) in submesh_tris.iter() {
            for i in tri.to_array() {
                visual_indices.push(i as u32 + visual_verts.len() as u32);
            }
        }
        visual_verts.extend(submesh_verts.iter().map(|v| Vec3::from(*v)));
    }
    visual_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, visual_verts);
    visual_mesh.insert_indices(Indices::U32(visual_indices));

    let standard_material = StandardMaterial {
        base_color: tailwind::EMERALD_200.with_alpha(0.7).into(),
        unlit: true,
        double_sided: true,
        alpha_mode: AlphaMode::AlphaToCoverage,
        ..default()
    };

    commands.entity(entity).insert((
        Mesh3d(meshes.add(visual_mesh)),
        MeshMaterial3d(materials.add(standard_material)),
    ));
}

fn draw_navmesh_affector(
    gizmo: Single<&Gizmo, With<NavmeshAffectorGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
    affector: Query<(&Mesh3d, &GlobalTransform), With<NavmeshAffector<Mesh3d>>>,
    meshes: Res<Assets<Mesh>>,
) {
    let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
        error!("Failed to get gizmo asset");
        return;
    };
    for (mesh, transform) in &affector {
        let Some(mesh) = meshes.get(&mesh.0) else {
            error!("Failed to get mesh asset");
            return;
        };
        let transform = transform.compute_transform();

        gizmo.clear();
        let mesh = TriMesh::from_mesh(mesh).unwrap();
        for indices in mesh.indices {
            let mut verts = indices
                .to_array()
                .iter()
                .map(|i| Vec3::from(mesh.vertices[*i as usize]))
                .map(|v| transform.transform_point(v))
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

fn hide_poly_mesh(
    gizmo: Single<(&Gizmo, &mut Visibility), With<PolyMeshGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
) {
    let (gizmo, mut visibility) = gizmo.into_inner();
    let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
        error!("Failed to get gizmo asset");
        return;
    };
    gizmo.clear();
    *visibility = Visibility::Hidden;
}

fn hide_affector(
    gizmo: Single<&Gizmo, With<NavmeshAffectorGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
) {
    let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
        error!("Failed to get gizmo asset");
        return;
    };
    gizmo.clear();
}

fn hide_detail_mesh(
    gizmo: Single<(&Gizmo, &mut Visibility), With<DetailMeshGizmo>>,
    mut gizmos: ResMut<Assets<GizmoAsset>>,
) {
    let (gizmo, mut visibility) = gizmo.into_inner();
    let Some(gizmo) = gizmos.get_mut(&gizmo.handle) else {
        error!("Failed to get gizmo asset");
        return;
    };
    gizmo.clear();
    *visibility = Visibility::Hidden;
}

fn hide_visual(mut visibility: Query<&mut Visibility, With<VisualMesh>>) {
    for mut visibility in visibility.iter_mut() {
        *visibility = Visibility::Hidden;
    }
}

#[derive(Component)]
pub(crate) struct VisualMesh;
