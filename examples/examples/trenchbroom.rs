//! A test scene that loads a TrenchBroom map.

use avian_rerecast::prelude::*;
use avian3d::prelude::*;
use bevy::{
    input::common_conditions::input_just_pressed,
    prelude::*,
    remote::{RemotePlugin, http::RemoteHttpPlugin},
};
use bevy_rerecast::{debug::DetailNavmeshGizmo, prelude::*};
use bevy_trenchbroom::prelude::*;
use bevy_trenchbroom_avian::AvianPhysicsBackend;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../assets".to_string(),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins((
            TrenchBroomPlugins(
                TrenchBroomConfig::new("bevy_rerecast")
                    .assets_path("scenes/trenchbroom/assets")
                    .default_solid_scene_hooks(|| {
                        SceneHooks::new()
                            .convex_collider()
                            .smooth_by_default_angle()
                    }),
            ),
            TrenchBroomPhysicsPlugin::new(AvianPhysicsBackend),
        ))
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Name::new("Level"),
        SceneRoot(asset_server.load("maps/scene.map#Scene")),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Vec3::new(0.5, -1.0, 0.3), Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(10.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
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
}

#[derive(Resource)]
#[allow(dead_code)]
struct NavmeshHandle(Handle<Navmesh>);

fn generate_navmesh(mut generator: NavmeshGenerator, mut commands: Commands) {
    let settings = NavmeshSettings::from_agent_3d(0.3, 1.0);
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
