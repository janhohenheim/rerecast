//! A test scene that loads a GLTF model as a level.

use avian3d::prelude::*;
use bevy::{
    prelude::*,
    remote::{RemotePlugin, http::RemoteHttpPlugin},
};
use bevy_rerecast::RerecastPlugin;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()))
        .add_plugins((RemotePlugin::default(), RemoteHttpPlugin::default()))
        .add_plugins(RerecastPlugin::default())
        .add_systems(Startup, setup)
        .add_observer(configure_camera)
        .run()
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Name::new("Level"),
        SceneRoot(asset_server.load("models/dungeon.glb#Scene0")),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Vec3::new(0.5, -1.0, 0.3), Vec3::Y),
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
