use bevy_asset::RenderAssetUsages;
use bevy_image::{Image, ImageSampler};
use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use wgpu_types::{
    TextureAspect, TextureDescriptor, TextureFormat, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

/// A version of [`Image`] suitable for serializing for short-term transfer.
///
/// [`Image`] does not implement [`Serialize`] / [`Deserialize`] because it is made with the renderer in mind.
/// It is not a general-purpose image implementation, and its internals are subject to frequent change.
/// As such, storing a [`Image`] on disk is highly discouraged.
///
/// But there are still some valid use cases for serializing a [`Image`], namely transferring meshes between processes.
/// To support this, you can create a [`SerializedImage`] from a [`Image`] with [`SerializedImage::from_image`],
/// and then deserialize it with [`SerializedImage::into_image`].
///
/// The caveats are:
/// - The image representation is not valid across different versions of Bevy.
/// - This conversion is lossy. The following information is not preserved:
///   - texture descriptor and texture view descriptor labels
///   - texture descriptor view formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedImage {
    data: Option<Vec<u8>>,
    texture_descriptor: TextureDescriptor<(), ()>,
    sampler: ImageSampler,
    texture_view_descriptor: Option<SerializedTextureViewDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializedTextureViewDescriptor {
    format: Option<TextureFormat>,
    dimension: Option<TextureViewDimension>,
    usage: Option<TextureUsages>,
    aspect: TextureAspect,
    base_mip_level: u32,
    mip_level_count: Option<u32>,
    base_array_layer: u32,
    array_layer_count: Option<u32>,
}

impl SerializedTextureViewDescriptor {
    fn from_texture_view_descriptor(
        descriptor: TextureViewDescriptor<Option<&'static str>>,
    ) -> Self {
        Self {
            format: descriptor.format,
            dimension: descriptor.dimension,
            usage: descriptor.usage,
            aspect: descriptor.aspect,
            base_mip_level: descriptor.base_mip_level,
            mip_level_count: descriptor.mip_level_count,
            base_array_layer: descriptor.base_array_layer,
            array_layer_count: descriptor.array_layer_count,
        }
    }

    fn into_texture_view_descriptor(self) -> TextureViewDescriptor<Option<&'static str>> {
        TextureViewDescriptor {
            // Not used for asset-based images other than debugging
            label: None,
            format: self.format,
            dimension: self.dimension,
            usage: self.usage,
            aspect: self.aspect,
            base_mip_level: self.base_mip_level,
            mip_level_count: self.mip_level_count,
            base_array_layer: self.base_array_layer,
            array_layer_count: self.array_layer_count,
        }
    }
}

impl SerializedImage {
    /// Creates a new [`SerializedImage`] from an [`Image`].
    pub fn from_image(image: Image) -> Self {
        Self {
            data: image.data,
            texture_descriptor: TextureDescriptor {
                label: (),
                size: image.texture_descriptor.size,
                mip_level_count: image.texture_descriptor.mip_level_count,
                sample_count: image.texture_descriptor.sample_count,
                dimension: image.texture_descriptor.dimension,
                format: image.texture_descriptor.format,
                usage: image.texture_descriptor.usage,
                view_formats: (),
            },
            sampler: image.sampler,
            texture_view_descriptor: image.texture_view_descriptor.map(|descriptor| {
                SerializedTextureViewDescriptor::from_texture_view_descriptor(descriptor)
            }),
        }
    }

    /// Create an [`Image`] from a [`SerializedImage`].
    pub fn into_image(self) -> Image {
        Image {
            data: self.data,
            texture_descriptor: TextureDescriptor {
                // Not used for asset-based images other than debugging
                label: None,
                size: self.texture_descriptor.size,
                mip_level_count: self.texture_descriptor.mip_level_count,
                sample_count: self.texture_descriptor.sample_count,
                dimension: self.texture_descriptor.dimension,
                format: self.texture_descriptor.format,
                usage: self.texture_descriptor.usage,
                // Not used for asset-based images
                view_formats: &[],
            },
            sampler: self.sampler,
            texture_view_descriptor: self
                .texture_view_descriptor
                .map(SerializedTextureViewDescriptor::into_texture_view_descriptor),
            asset_usage: RenderAssetUsages::all(),
        }
    }
}
