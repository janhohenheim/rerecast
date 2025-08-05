use bevy::prelude::*;
use bevy_rerecast::{
    debug::{DetailNavmeshGizmo, PolygonNavmeshGizmo},
    prelude::*,
    rerecast::TriMesh,
};

pub(super) fn plugin(app: &mut App) {
    app.set_navmesh_affector_backend(editor_backend);
    app.add_observer(build_navmesh);
    app.init_resource::<GlobalNavmeshSettings>()
        .init_resource::<NavmeshHandle>();
}

fn editor_backend(
    _: In<NavmeshSettings>,
    affectors: Query<(&GlobalTransform, &NavmeshAffector)>,
) -> Vec<(GlobalTransform, TriMesh)> {
    affectors
        .iter()
        .map(|(transform, affector)| (*transform, affector.0.clone()))
        .collect()
}

#[derive(Component, Deref, DerefMut)]
pub(crate) struct NavmeshAffector(pub(crate) TriMesh);

#[derive(Event)]
pub(crate) struct BuildNavmesh;

#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct GlobalNavmeshSettings(pub(crate) NavmeshSettings);

#[derive(Resource, Default, Deref, DerefMut)]
pub(crate) struct NavmeshHandle(pub(crate) Handle<Navmesh>);

fn build_navmesh(
    _trigger: Trigger<BuildNavmesh>,
    mut commands: Commands,
    config: Res<GlobalNavmeshSettings>,
    mut navmesh_generator: NavmeshGenerator,
) {
    let handle = navmesh_generator.generate(config.0.clone());
    commands.spawn(PolygonNavmeshGizmo(handle.id()));
    commands.spawn(DetailNavmeshGizmo(handle.id()));
    commands.insert_resource(NavmeshHandle(handle));
}
