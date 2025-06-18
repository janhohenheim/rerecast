use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(19.769, 50.702, 20.619).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/voortrekker_interior_1k_diffuse.ktx2"),
            specular_map: asset_server
                .load("environment_maps/voortrekker_interior_1k_specular.ktx2"),
            intensity: 2000.0,
            ..default()
        },
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Vec3::new(0.5, -1.0, 0.3), Vec3::Y),
    ));
}
