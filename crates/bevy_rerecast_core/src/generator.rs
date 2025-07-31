//! Utilities for generating navmeshes at runtime.

use std::{collections::VecDeque, marker::PhantomData};

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemParam};
use rerecast::NavmeshConfig;

use crate::Navmesh;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<NavmeshQueue>();
}

/// System parameter for generating navmeshes.
#[derive(SystemParam)]
pub struct NavmeshGenerator<'w, Marker: 'static> {
    #[system_param(
        validation_message = "Failed to find `Assets<Navmesh>`. Did you forget to add `NavmeshPlugins` to your app?"
    )]
    navmeshes: Res<'w, Assets<Navmesh>>,
    queue: ResMut<'w, NavmeshQueue>,
    marker: PhantomData<Marker>,
}

impl<'w, Marker: 'static> NavmeshGenerator<'w, Marker> {
    /// Queue a navmesh generation task.
    /// When you call this method, a new navmesh will be generated asynchronously.
    /// Calling it multiple times will queue multiple navmeshes to be generated in a FIFO order.
    pub fn generate(&mut self, config: NavmeshConfig) -> Handle<Navmesh> {
        let handle = self.navmeshes.reserve_handle();
        self.queue.push_back((handle.clone(), config));
        handle
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
struct NavmeshQueue(VecDeque<(Handle<Navmesh>, NavmeshConfig)>);
