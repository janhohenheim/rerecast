//! A test scene that only uses primitive shapes.

use avian_rerecast::prelude::*;
use avian3d::prelude::*;
use bevy::{
    color::palettes::tailwind,
    input::common_conditions::input_just_pressed,
    prelude::*,
    remote::{RemotePlugin, http::RemoteHttpPlugin},
};
use bevy_rerecast::{debug::DetailNavmeshGizmo, prelude::*};

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins((RemotePlugin::default(), RemoteHttpPlugin::default()))
        .add_plugins((NavmeshPlugins::default(), AvianBackendPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            generate_navmesh.run_if(input_just_pressed(KeyCode::Space)),
        )
        .add_observer(configure_camera)
        .run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material_gray = materials.add(Color::from(tailwind::GRAY_300));
    let material_red = materials.add(Color::from(tailwind::RED_500));

    commands.spawn((
        Name::new("Ground"),
        RigidBody::Static,
        Collider::cylinder(25.0, 0.2),
        Mesh3d(meshes.add(Cylinder::new(25.0, 0.2))),
        MeshMaterial3d(material_gray.clone()),
    ));

    let ball_scales = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0];
    for (i, &scale) in ball_scales.iter().enumerate() {
        commands.spawn((
            Name::new(format!("Ball {}", i + 1)),
            RigidBody::Static,
            Collider::sphere(1.0),
            Mesh3d(meshes.add(Sphere::new(1.0))),
            Transform::from_xyz(-20.0 + i as f32 * 5.0, 0.0, 5.0 + i as f32)
                .with_scale(Vec3::splat(scale)),
            MeshMaterial3d(material_red.clone()),
        ));
    }

    commands.spawn((
        Name::new("Cube"),
        RigidBody::Static,
        Collider::cuboid(10.0, 1.0, 10.0),
        Mesh3d(meshes.add(Cuboid::new(10.0, 1.0, 10.0))),
        Transform::from_xyz(-10.0, 3.0, -10.0),
        MeshMaterial3d(material_red.clone()),
    ));

    commands.spawn((
        Name::new("RotatedCube"),
        RigidBody::Static,
        Collider::cuboid(3.0, 1.0, 3.0),
        Mesh3d(meshes.add(Cuboid::new(3.0, 1.0, 3.0))),
        Transform::from_xyz(2.0, 0.5, -2.0)
            .with_rotation(Quat::from_rotation_y(45.0_f32.to_radians())),
        MeshMaterial3d(material_red.clone()),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(0.5, -1.0, 0.3), Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(30.0, 30.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Text::new("Press space to generate navmesh from avian colliders"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

#[derive(Resource)]
#[allow(dead_code)]
struct NavmeshHandle(Handle<Navmesh>);

fn generate_navmesh(mut generator: NavmeshGenerator, mut commands: Commands) {
    let settings = NavmeshSettings::default();
    let navmesh = generator.generate(settings);
    commands.spawn(DetailNavmeshGizmo::new(&navmesh));
    commands.insert_resource(NavmeshHandle(navmesh));
}

fn configure_camera(
    trigger: On<Add, Camera>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.entity(trigger.entity).insert(EnvironmentMapLight {
        diffuse_map: asset_server.load("environment_maps/voortrekker_interior_1k_diffuse.ktx2"),
        specular_map: asset_server.load("environment_maps/voortrekker_interior_1k_specular.ktx2"),
        intensity: 2000.0,
        ..default()
    });
}
