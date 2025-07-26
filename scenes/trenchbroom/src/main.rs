//! A test scene that loads a TrenchBroom map.

use avian3d::prelude::*;
use bevy::{
    prelude::*,
    remote::{RemotePlugin, http::RemoteHttpPlugin},
};
use bevy_rerecast::RerecastPlugin;
use bevy_trenchbroom::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()))
        .add_plugins(TrenchBroomPlugins(
            TrenchBroomConfig::new("bevy_rerecast").assets_path("scenes/trenchbroom/assets"),
        ))
        .register_type::<Worldspawn>()
        .add_plugins((RemotePlugin::default(), RemoteHttpPlugin::default()))
        .add_plugins(RerecastPlugin::default())
        .add_systems(Startup, (write_trenchbroom_config, setup).chain())
        .add_observer(configure_camera)
        .run()
}

#[derive(SolidClass, Component, Reflect)]
#[reflect(QuakeClass, Component)]
#[spawn_hooks(SpawnHooks::new().smooth_by_default_angle().convex_collider())]
struct Worldspawn;

fn write_trenchbroom_config(server: Res<TrenchBroomServer>, type_registry: Res<AppTypeRegistry>) {
    if let Err(err) = server
        .config
        .write_game_config_to_default_directory(&type_registry.read())
    {
        error!("Could not write TrenchBroom game config: {err}");
    }

    if let Err(err) = server.config.add_game_to_preferences_in_default_directory() {
        error!("Could not write TrenchBroom preferences: {err}");
    }
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
