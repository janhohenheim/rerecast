use std::{fs::File, io};

use crate::backend::NavmeshHandle;
use bevy::ecs::world::WorldId;
use bevy::{prelude::*};
use bevy_malek_async::async_access;
use bevy_rerecast::Navmesh;
use rfd::FileHandle;
use thiserror::Error;

pub(crate) async fn save_navmesh(
    world_id: WorldId,
    save: impl Future<Output=Option<FileHandle>>,
) -> core::result::Result<(), SaveError> {
    let Some(file_handle) = save.await else {
        return Err(SaveError::UserCanceled);
    };
    let navmesh = async_access::<(Res<NavmeshHandle>, Res<Assets<Navmesh>>), _, _>(
        world_id,
        |(navmesh, navmeshes)| {
            navmeshes
                .get(navmesh.id())
                .ok_or(SaveError::NoNavmesh)
                .cloned()
        },
    )
    .await?;
    let path = file_handle.path();
    let mut file = File::create(path)?;
    let config = bincode::config::standard();
    bincode::serde::encode_into_std_write(navmesh, &mut file, config)?;
    Ok(())
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("User canceled the save operation")]
    UserCanceled,
    #[error("There's no navmesh to save")]
    NoNavmesh,
    #[error("Failed to create file: {0}")]
    CreateFile(#[from] io::Error),
    #[error("Failed to encode navmesh: {0}")]
    WriteNavmesh(#[from] bincode::error::EncodeError),
}
