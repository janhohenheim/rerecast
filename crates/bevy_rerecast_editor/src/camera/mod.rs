use bevy::{asset::embedded_asset, prelude::*};

use crate::camera::camera_controller::CameraController;

mod camera_controller;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_plugins(camera_controller::CameraControllerPlugin);
    embedded_asset!(
        app,
        "assets/environment_maps/voortrekker_interior_1k_diffuse.ktx2"
    );
    embedded_asset!(
        app,
        "assets/environment_maps/voortrekker_interior_1k_specular.ktx2"
    );
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        CameraController::default(),
        Transform::from_xyz(19.769, 50.702, 20.619).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("embedded://bevy_rerecast_editor/camera/assets/environment_maps/voortrekker_interior_1k_diffuse.ktx2"),
            specular_map: asset_server
                .load("embedded://bevy_rerecast_editor/camera/assets/environment_maps/voortrekker_interior_1k_specular.ktx2"),
            intensity: 2000.0,
            ..default()
        },
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Vec3::new(0.5, -1.0, 0.3), Vec3::Y),
    ));
}
