use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(build_navmesh);
}

#[derive(Event)]
pub(crate) struct BuildNavmesh;

fn build_navmesh(_trigger: Trigger<BuildNavmesh>, mut commands: Commands) {
    commands.trigger(BuildNavmesh);
}
