use std::{fs::File, io};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};
use bevy_rerecast::Navmesh;
use rfd::FileHandle;
use thiserror::Error;

use crate::backend::NavmeshHandle;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<WriteTasks>();
    app.add_systems(
        Update,
        (
            poll_save_task.run_if(resource_exists::<SaveTask>),
            poll_write_tasks,
        )
            .chain(),
    );
}

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct SaveTask(pub(crate) Task<Option<FileHandle>>);

fn poll_save_task(
    mut commands: Commands,
    mut task: ResMut<SaveTask>,
    navmesh: Res<NavmeshHandle>,
    navmeshes: Res<Assets<Navmesh>>,
    mut write_tasks: ResMut<WriteTasks>,
) {
    let Some(file_handle) = future::block_on(future::poll_once(&mut task.0)) else {
        return;
    };
    commands.remove_resource::<SaveTask>();
    let Some(file) = file_handle else {
        // User canceled the save operation
        return;
    };

    let Some(navmesh) = navmeshes.get(navmesh.id()) else {
        // There's no navmesh to save
        return;
    };
    let thread_pool = AsyncComputeTaskPool::get();

    let navmesh = navmesh.clone();
    let future = async move {
        let path = file.path();
        let mut file = File::create(path)?;
        let config = bincode::config::standard();
        bincode::serde::encode_into_std_write(navmesh, &mut file, config)?;
        Ok(())
    };
    write_tasks.push(thread_pool.spawn(future));
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("Failed to create file: {0}")]
    CreateFile(#[from] io::Error),
    #[error("Failed to encode navmesh: {0}")]
    WriteNavmesh(#[from] bincode::error::EncodeError),
}

#[derive(Resource, Default, Deref, DerefMut)]
struct WriteTasks(Vec<Task<Result<(), SaveError>>>);

fn poll_write_tasks(mut write_tasks: ResMut<WriteTasks>) {
    let mut tasks = std::mem::take(&mut *write_tasks);
    tasks.retain_mut(|task| {
        let Some(result) = future::block_on(future::poll_once(task)) else {
            return true;
        };
        match result {
            Ok(()) => false,
            Err(err) => {
                error!("Failed to save navmesh: {}", err);
                false
            }
        }
    });
}
