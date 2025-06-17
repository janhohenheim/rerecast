use bevy::prelude::*;
use bevy_full_asset_path::prelude::*;
use std::path::PathBuf;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(FullAssetPathPlugin::default());
    app.register_type::<FullSceneAssetPath>();
    app.add_observer(reflect_scene_root);
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Reflect, Deref, DerefMut)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
pub struct FullSceneAssetPath(pub PathBuf);

fn reflect_scene_root(
    trigger: Trigger<OnAdd, SceneRoot>,
    scene_root: Query<(&SceneRoot, NameOrEntity)>,
    full_asset_path_provider: Res<FullAssetPathProvider>,
    mut commands: Commands,
) {
    let entity = trigger.target();
    let Ok((scene_root, name)) = scene_root.get(entity) else {
        debug!("Scene root not found: {entity}");
        return;
    };
    let Some(asset_path) = scene_root.0.path() else {
        debug!("Scene root has no asset path: {name}");
        return;
    };
    let Ok(full_path) = full_asset_path_provider.full_asset_path(asset_path) else {
        debug!("Failed to get full asset path for scene root: {name}");
        return;
    };

    commands
        .entity(entity)
        .insert(FullSceneAssetPath(full_path));
}
