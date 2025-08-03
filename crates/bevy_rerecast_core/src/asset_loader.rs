use bevy_app::prelude::*;
use bevy_asset::{AssetApp as _, AssetLoader, AsyncReadExt as _, LoadContext, io::Reader};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Navmesh;

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<Navmesh>();
    app.init_asset_loader::<NavmeshLoader>();
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NavmeshLoader;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NavmeshLoaderSettings;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NavmeshLoaderError {
    #[error("Could not load navmesh: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Could not deserialize navmesh: {0}")]
    DeserializeError(#[from] serde_json::Error),
}

impl AssetLoader for NavmeshLoader {
    type Asset = Navmesh;
    type Settings = NavmeshLoaderSettings;
    type Error = NavmeshLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let value = serde_json::from_slice(&bytes)?;
        Ok(value)
    }

    fn extensions(&self) -> &[&str] {
        &["nav"]
    }
}
