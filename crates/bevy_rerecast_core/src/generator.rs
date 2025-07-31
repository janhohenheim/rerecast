use std::{collections::VecDeque, marker::PhantomData};

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::{prelude::*, system::SystemParam};
use rerecast::{DetailNavmesh, NavmeshConfig, PolygonNavmesh};

use crate::Navmesh;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<NavmeshQueue>();
}

#[derive(SystemParam)]
pub struct NavmeshGenerator<'w, Marker: 'static> {
    #[system_param(
        validation_message = "Failed to find `Assets<Navmesh>`. Did you forget to add `NavmeshPlugins` to your app?"
    )]
    navmeshes: Res<'w, Assets<Navmesh>>,
    marker: PhantomData<Marker>,
}

impl<'w, Marker: 'static> NavmeshGenerator<'w, Marker> {
    pub fn generate(&self, config: NavmeshConfig) -> Handle<Navmesh> {
        self.navmeshes.reserve_handle()
    }
}

#[derive(Resource, Default)]
struct NavmeshQueue(VecDeque<(Handle<Navmesh>, NavmeshConfig)>);
