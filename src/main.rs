use bevy::prelude::*;
use navmesh::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NavMeshPlugin::default())
        .run()
}
