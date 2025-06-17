use avian_navmesh::prelude::*;
use bevy::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NavMeshPlugin::default())
        .run()
}
