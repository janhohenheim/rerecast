//! A test scene that only uses primitive shapes.

use avian_rerecast::prelude::*;
use avian3d::prelude::*;
use bevy::{
    color::palettes::tailwind,
    input::common_conditions::input_just_pressed,
    math::bounding::Aabb3d,
    platform::collections::HashSet,
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

#[derive(Resource)]
struct NavmeshFilter(HashSet<Entity>);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material_gray = materials.add(Color::from(tailwind::GRAY_300).with_alpha(0.2));
    let material_red = materials.add(Color::from(tailwind::RED_500));
    let material_blue = materials.add(Color::from(tailwind::SKY_500));
    let cube = Cuboid::new(1.0, 1.0, 1.0);
    let plane = Cuboid::new(1000.0, 1.0, 1000.0);
    let ground = commands
        .spawn((
            Name::new("Ground"),
            Mesh3d(meshes.add(plane)),
            RigidBody::Static,
            Collider::from(plane),
            MeshMaterial3d(material_blue.clone()),
        ))
        .id();
    commands.spawn((
        Name::new("Outer Cube"),
        Mesh3d(meshes.add(cube)),
        RigidBody::Static,
        Collider::from(cube),
        Transform::default().with_scale(Vec3::splat(1000.0)),
        MeshMaterial3d(material_gray.clone()),
    ));
    let inner_cube = commands
        .spawn((
            Name::new("Inner Cube"),
            Mesh3d(meshes.add(cube)),
            RigidBody::Static,
            Collider::from(cube),
            Transform::default().with_scale(Vec3::splat(100.0)),
            MeshMaterial3d(material_red.clone()),
        ))
        .id();

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(0.5, -2.0, -2.0), Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2000.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Text::new("Press space to generate navmesh"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    commands.insert_resource(NavmeshFilter([ground, inner_cube].into_iter().collect()));
}

#[derive(Resource)]
#[allow(dead_code)]
struct NavmeshHandle(Handle<Navmesh>);

fn generate_navmesh(
    mut generator: NavmeshGenerator,
    mut commands: Commands,
    filter: Res<NavmeshFilter>,
) {
    let settings = NavmeshSettings {
        filter: Some(filter.0.clone()),
        aabb: Some(Aabb3d::new(Vec3::ZERO, Vec3::new(500.0, 10.0, 500.0))),
        ..NavmeshSettings::from_agent(5.0, 10.0)
    };
    let navmesh = generator.generate(settings);
    commands.spawn(DetailNavmeshGizmo::new(&navmesh));
    commands.insert_resource(NavmeshHandle(navmesh));
}

fn configure_camera(
    trigger: Trigger<OnAdd, Camera>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands
        .entity(trigger.target())
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/voortrekker_interior_1k_diffuse.ktx2"),
            specular_map: asset_server
                .load("environment_maps/voortrekker_interior_1k_specular.ktx2"),
            intensity: 2000.0,
            ..default()
        });
}
