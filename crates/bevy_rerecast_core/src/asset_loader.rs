//! Types for loading [`Navmesh`]es using the [`AssetServer`](bevy_asset::AssetServer).

use bevy_app::prelude::*;
use bevy_asset::{AssetApp as _, AssetLoader, LoadContext, io::Reader};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Navmesh;

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<Navmesh>();
    app.init_asset_loader::<NavmeshLoader>();
}

/// The [`AssetLoader`] for [`Navmesh`] assets. Loads files ending in `.nav`.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NavmeshLoader;

/// Settings for the [`NavmeshLoader`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NavmeshLoaderSettings;

/// Errors that can occur when loading a [`Navmesh`] asset.
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
