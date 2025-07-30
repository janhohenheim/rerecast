use bevy_asset::{Assets, Handle};
use bevy_color::prelude::*;
use bevy_image::Image;
use bevy_math::Affine2;
use bevy_pbr::{OpaqueRendererMethod, UvChannel, prelude::*};
use bevy_platform::collections::HashMap;
use bevy_render::alpha::AlphaMode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wgpu_types::Face;

use crate::SerializedImage;

/// Serialized representation of a [`StandardMaterial`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedStandardMaterial {
    base_color: Color,
    base_color_channel: SerializedUvChannel,
    base_color_texture: Option<u32>,
    emissive: LinearRgba,
    emissive_exposure_weight: f32,
    emissive_channel: SerializedUvChannel,
    emissive_texture: Option<u32>,
    perceptual_roughness: f32,
    metallic: f32,
    metallic_roughness_channel: SerializedUvChannel,
    metallic_roughness_texture: Option<u32>,
    reflectance: f32,
    specular_tint: Color,
    diffuse_transmission: f32,
    #[cfg(feature = "pbr_transmission_textures")]
    #[cfg_attr(feature = "pbr_transmission_textures", serde(default))]
    diffuse_transmission_channel: SerializedUvChannel,
    #[cfg(feature = "pbr_transmission_textures")]
    #[cfg_attr(feature = "pbr_transmission_textures", serde(default))]
    diffuse_transmission_texture: Option<u32>,
    specular_transmission: f32,
    #[cfg(feature = "pbr_transmission_textures")]
    #[cfg_attr(feature = "pbr_transmission_textures", serde(default))]
    specular_transmission_channel: SerializedUvChannel,
    #[cfg(feature = "pbr_transmission_textures")]
    #[cfg_attr(feature = "pbr_transmission_textures", serde(default))]
    specular_transmission_texture: Option<u32>,
    thickness: f32,
    #[cfg(feature = "pbr_transmission_textures")]
    #[cfg_attr(feature = "pbr_transmission_textures", serde(default))]
    thickness_channel: SerializedUvChannel,
    #[cfg(feature = "pbr_transmission_textures")]
    #[cfg_attr(feature = "pbr_transmission_textures", serde(default))]
    thickness_texture: Option<u32>,
    ior: f32,
    attenuation_distance: f32,
    attenuation_color: Color,
    normal_map_channel: SerializedUvChannel,
    normal_map_texture: Option<u32>,
    flip_normal_map_y: bool,
    occlusion_channel: SerializedUvChannel,
    occlusion_texture: Option<u32>,
    #[cfg(feature = "pbr_specular_textures")]
    #[cfg_attr(feature = "pbr_specular_textures", serde(default))]
    specular_channel: SerializedUvChannel,
    #[cfg(feature = "pbr_specular_textures")]
    #[cfg_attr(feature = "pbr_specular_textures", serde(default))]
    specular_texture: Option<u32>,
    #[cfg(feature = "pbr_specular_textures")]
    #[cfg_attr(feature = "pbr_specular_textures", serde(default))]
    specular_tint_channel: SerializedUvChannel,
    #[cfg(feature = "pbr_specular_textures")]
    #[cfg_attr(feature = "pbr_specular_textures", serde(default))]
    specular_tint_texture: Option<u32>,
    clearcoat: f32,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", serde(default))]
    clearcoat_channel: SerializedUvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", serde(default))]
    clearcoat_texture: Option<u32>,
    clearcoat_perceptual_roughness: f32,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", serde(default))]
    clearcoat_roughness_channel: SerializedUvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", serde(default))]
    clearcoat_roughness_texture: Option<u32>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", serde(default))]
    clearcoat_normal_channel: SerializedUvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    #[cfg_attr(feature = "pbr_multi_layer_material_textures", serde(default))]
    clearcoat_normal_texture: Option<u32>,
    anisotropy_strength: f32,
    anisotropy_rotation: f32,
    #[cfg(feature = "pbr_anisotropy_texture")]
    #[cfg_attr(feature = "pbr_anisotropy_texture", serde(default))]
    anisotropy_channel: SerializedUvChannel,
    #[cfg_attr(feature = "pbr_anisotropy_texture", serde(default))]
    #[cfg(feature = "pbr_anisotropy_texture")]
    anisotropy_texture: Option<u32>,
    double_sided: bool,
    cull_mode: Option<Face>,
    unlit: bool,
    fog_enabled: bool,
    alpha_mode: SerializedAlphaMode,
    depth_bias: f32,
    depth_map: Option<u32>,
    parallax_depth_scale: f32,
    parallax_mapping_method: SerializedParallaxMappingMethod,
    max_parallax_layer_count: f32,
    lightmap_exposure: f32,
    opaque_render_method: SerializedOpaqueRendererMethod,
    deferred_lighting_pass_id: u8,
    uv_transform: Affine2,
}

#[derive(Error, Debug)]
pub enum SerializedStandardMaterialError {
    #[error("Image not found: {0}")]
    ImageNotFound(&'static str),
}

impl SerializedStandardMaterial {
    /// Serialize a [`StandardMaterial`] into a [`SerializedStandardMaterial`]. Returns `None` if any of the images are not found in `images`.
    pub fn try_from_standard_material(
        material: StandardMaterial,
        indices: &mut HashMap<Handle<Image>, u32>,
        images: &Assets<Image>,
        cache: &mut Vec<SerializedImage>,
    ) -> Result<Self, SerializedStandardMaterialError> {
        let mut serialize_image = |image_handle, name| {
            if let Some(image_handle) = image_handle {
                let index = match indices.get(&image_handle) {
                    Some(&index) => index,
                    None => {
                        let Some(image) = images.get(&image_handle) else {
                            return Err(SerializedStandardMaterialError::ImageNotFound(name));
                        };
                        let index = cache.len() as u32;
                        cache.push(SerializedImage::from_image(image.clone()));
                        indices.insert(image_handle, index);
                        index
                    }
                };
                Ok(Some(index))
            } else {
                Ok(None)
            }
        };
        Ok(Self {
            base_color: material.base_color,
            base_color_channel: SerializedUvChannel::from_uv_channel(material.base_color_channel),
            base_color_texture: serialize_image(material.base_color_texture, "base_color_texture")?,
            emissive: material.emissive,
            emissive_exposure_weight: material.emissive_exposure_weight,
            emissive_channel: SerializedUvChannel::from_uv_channel(material.emissive_channel),
            emissive_texture: serialize_image(material.emissive_texture, "emissive_texture")?,
            perceptual_roughness: material.perceptual_roughness,
            metallic: material.metallic,
            metallic_roughness_channel: SerializedUvChannel::from_uv_channel(
                material.metallic_roughness_channel,
            ),
            metallic_roughness_texture: serialize_image(
                material.metallic_roughness_texture,
                "metallic_roughness_texture",
            )?,
            reflectance: material.reflectance,
            specular_tint: material.specular_tint,
            diffuse_transmission: material.diffuse_transmission,
            #[cfg(feature = "pbr_transmission_textures")]
            diffuse_transmission_channel: SerializedUvChannel::from_uv_channel(
                material.diffuse_transmission_channel,
            ),
            #[cfg(feature = "pbr_transmission_textures")]
            diffuse_transmission_texture: serialize_image(
                material.diffuse_transmission_texture,
                "diffuse_transmission_texture",
            )?,
            specular_transmission: material.specular_transmission,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_channel: SerializedUvChannel::from_uv_channel(
                material.specular_transmission_channel,
            ),
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_texture: serialize_image(
                material.specular_transmission_texture,
                "specular_transmission_texture",
            )?,
            thickness: material.thickness,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_channel: SerializedUvChannel::from_uv_channel(material.thickness_channel),
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_texture: serialize_image(material.thickness_texture, "thickness_texture")?,
            ior: material.ior,
            attenuation_distance: material.attenuation_distance,
            attenuation_color: material.attenuation_color,
            normal_map_channel: SerializedUvChannel::from_uv_channel(material.normal_map_channel),
            normal_map_texture: serialize_image(material.normal_map_texture, "normal_map_texture")?,
            flip_normal_map_y: material.flip_normal_map_y,
            occlusion_channel: SerializedUvChannel::from_uv_channel(material.occlusion_channel),
            occlusion_texture: serialize_image(material.occlusion_texture, "occlusion_texture")?,
            #[cfg(feature = "pbr_specular_textures")]
            specular_channel: SerializedUvChannel::from_uv_channel(material.specular_channel),
            #[cfg(feature = "pbr_specular_textures")]
            specular_texture: serialize_image(material.specular_texture, "specular_texture")?,
            #[cfg(feature = "pbr_specular_textures")]
            specular_tint_channel: SerializedUvChannel::from_uv_channel(
                material.specular_tint_channel,
            ),
            #[cfg(feature = "pbr_specular_textures")]
            specular_tint_texture: serialize_image(
                material.specular_tint_texture,
                "specular_tint_texture",
            )?,
            clearcoat: material.clearcoat,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_channel: SerializedUvChannel::from_uv_channel(material.clearcoat_channel),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_texture: serialize_image(material.clearcoat_texture, "clearcoat_texture")?,
            clearcoat_perceptual_roughness: material.clearcoat_perceptual_roughness,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_channel: SerializedUvChannel::from_uv_channel(
                material.clearcoat_roughness_channel,
            ),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_texture: serialize_image(
                material.clearcoat_roughness_texture,
                "clearcoat_roughness_texture",
            )?,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_channel: SerializedUvChannel::from_uv_channel(
                material.clearcoat_normal_channel,
            ),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_texture: serialize_image(
                material.clearcoat_normal_texture,
                "clearcoat_normal_texture",
            )?,
            anisotropy_strength: material.anisotropy_strength,
            anisotropy_rotation: material.anisotropy_rotation,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel: SerializedUvChannel::from_uv_channel(material.anisotropy_channel),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture: serialize_image(material.anisotropy_texture, "anisotropy_texture")?,
            double_sided: material.double_sided,
            cull_mode: material.cull_mode,
            unlit: material.unlit,
            fog_enabled: material.fog_enabled,
            alpha_mode: SerializedAlphaMode::from_alpha_mode(material.alpha_mode),
            depth_bias: material.depth_bias,
            depth_map: serialize_image(material.depth_map, "depth_map")?,
            parallax_depth_scale: material.parallax_depth_scale,
            parallax_mapping_method: SerializedParallaxMappingMethod::from_parallax_mapping_method(
                material.parallax_mapping_method,
            ),
            max_parallax_layer_count: material.max_parallax_layer_count,
            lightmap_exposure: material.lightmap_exposure,
            opaque_render_method: SerializedOpaqueRendererMethod::from_opaque_renderer_method(
                material.opaque_render_method,
            ),
            deferred_lighting_pass_id: material.deferred_lighting_pass_id,
            uv_transform: material.uv_transform,
        })
    }

    /// Deserialize a [`SerializedStandardMaterial`] into a [`StandardMaterial`].
    pub fn into_standard_material(
        self,
        indices: &mut HashMap<u32, Handle<Image>>,
        images: &mut Assets<Image>,
        serialized_images: &[SerializedImage],
    ) -> StandardMaterial {
        let mut deserialize_image = |index| {
            let index = index?;
            let handle = if let Some(handle) = indices.get(&index) {
                handle.clone()
            } else {
                let image = serialized_images[index as usize].clone().into_image();
                let handle = images.add(image);
                indices.insert(index, handle.clone());
                handle
            };
            Some(handle)
        };
        StandardMaterial {
            base_color: self.base_color,
            base_color_channel: self.base_color_channel.into_uv_channel(),
            base_color_texture: deserialize_image(self.base_color_texture),
            emissive: self.emissive,
            emissive_exposure_weight: self.emissive_exposure_weight,
            emissive_channel: self.emissive_channel.into_uv_channel(),
            emissive_texture: deserialize_image(self.emissive_texture),
            perceptual_roughness: self.perceptual_roughness,
            metallic: self.metallic,
            metallic_roughness_channel: self.metallic_roughness_channel.into_uv_channel(),
            metallic_roughness_texture: deserialize_image(self.metallic_roughness_texture),
            reflectance: self.reflectance,
            specular_tint: self.specular_tint,
            diffuse_transmission: self.diffuse_transmission,
            #[cfg(feature = "pbr_transmission_textures")]
            diffuse_transmission_channel: self.diffuse_transmission_channel.into_uv_channel(),
            #[cfg(feature = "pbr_transmission_textures")]
            diffuse_transmission_texture: deserialize_image(self.diffuse_transmission_texture),
            specular_transmission: self.specular_transmission,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_channel: self.specular_transmission_channel.into_uv_channel(),
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_texture: deserialize_image(self.specular_transmission_texture),
            thickness: self.thickness,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_channel: self.thickness_channel.into_uv_channel(),
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_texture: deserialize_image(self.thickness_texture),
            ior: self.ior,
            attenuation_distance: self.attenuation_distance,
            attenuation_color: self.attenuation_color,
            normal_map_channel: self.normal_map_channel.into_uv_channel(),
            normal_map_texture: deserialize_image(self.normal_map_texture),
            flip_normal_map_y: self.flip_normal_map_y,
            occlusion_channel: self.occlusion_channel.into_uv_channel(),
            occlusion_texture: deserialize_image(self.occlusion_texture),
            #[cfg(feature = "pbr_specular_textures")]
            specular_channel: self.specular_channel.into_uv_channel(),
            #[cfg(feature = "pbr_specular_textures")]
            specular_texture: deserialize_image(self.specular_texture),
            #[cfg(feature = "pbr_specular_textures")]
            specular_tint_channel: self.specular_tint_channel.into_uv_channel(),
            #[cfg(feature = "pbr_specular_textures")]
            specular_tint_texture: deserialize_image(self.specular_tint_texture),
            clearcoat: self.clearcoat,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_channel: self.clearcoat_channel.into_uv_channel(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_texture: deserialize_image(self.clearcoat_texture),
            clearcoat_perceptual_roughness: self.clearcoat_perceptual_roughness,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_channel: self.clearcoat_roughness_channel.into_uv_channel(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_texture: deserialize_image(self.clearcoat_roughness_texture),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_channel: self.clearcoat_normal_channel.into_uv_channel(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_texture: deserialize_image(self.clearcoat_normal_texture),
            anisotropy_strength: self.anisotropy_strength,
            anisotropy_rotation: self.anisotropy_rotation,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel: self.anisotropy_channel.into_uv_channel(),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture: deserialize_image(self.anisotropy_texture),
            double_sided: self.double_sided,
            cull_mode: self.cull_mode,
            unlit: self.unlit,
            fog_enabled: self.fog_enabled,
            alpha_mode: self.alpha_mode.into_alpha_mode(),
            depth_bias: self.depth_bias,
            depth_map: deserialize_image(self.depth_map),
            parallax_depth_scale: self.parallax_depth_scale,
            parallax_mapping_method: self.parallax_mapping_method.into_parallax_mapping_method(),
            max_parallax_layer_count: self.max_parallax_layer_count,
            lightmap_exposure: self.lightmap_exposure,
            opaque_render_method: self.opaque_render_method.into_opaque_renderer_method(),
            deferred_lighting_pass_id: self.deferred_lighting_pass_id,
            uv_transform: self.uv_transform,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
enum SerializedUvChannel {
    #[default]
    Uv0,
    Uv1,
}

impl SerializedUvChannel {
    fn from_uv_channel(channel: UvChannel) -> Self {
        match channel {
            UvChannel::Uv0 => SerializedUvChannel::Uv0,
            UvChannel::Uv1 => SerializedUvChannel::Uv1,
        }
    }
    fn into_uv_channel(self) -> UvChannel {
        match self {
            SerializedUvChannel::Uv0 => UvChannel::Uv0,
            SerializedUvChannel::Uv1 => UvChannel::Uv1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SerializedAlphaMode {
    Opaque,
    Mask(f32),
    Blend,
    Premultiplied,
    AlphaToCoverage,
    Add,
    Multiply,
}

impl SerializedAlphaMode {
    fn from_alpha_mode(alpha_mode: AlphaMode) -> Self {
        match alpha_mode {
            AlphaMode::Opaque => SerializedAlphaMode::Opaque,
            AlphaMode::Mask(threshold) => SerializedAlphaMode::Mask(threshold),
            AlphaMode::Blend => SerializedAlphaMode::Blend,
            AlphaMode::Premultiplied => SerializedAlphaMode::Premultiplied,
            AlphaMode::AlphaToCoverage => SerializedAlphaMode::AlphaToCoverage,
            AlphaMode::Add => SerializedAlphaMode::Add,
            AlphaMode::Multiply => SerializedAlphaMode::Multiply,
        }
    }
    fn into_alpha_mode(self) -> AlphaMode {
        match self {
            SerializedAlphaMode::Opaque => AlphaMode::Opaque,
            SerializedAlphaMode::Mask(threshold) => AlphaMode::Mask(threshold),
            SerializedAlphaMode::Blend => AlphaMode::Blend,
            SerializedAlphaMode::Premultiplied => AlphaMode::Premultiplied,
            SerializedAlphaMode::AlphaToCoverage => AlphaMode::AlphaToCoverage,
            SerializedAlphaMode::Add => AlphaMode::Add,
            SerializedAlphaMode::Multiply => AlphaMode::Multiply,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SerializedParallaxMappingMethod {
    Occlusion,
    Relief { max_steps: u32 },
}

impl SerializedParallaxMappingMethod {
    fn from_parallax_mapping_method(method: ParallaxMappingMethod) -> Self {
        match method {
            ParallaxMappingMethod::Occlusion => SerializedParallaxMappingMethod::Occlusion,
            ParallaxMappingMethod::Relief { max_steps } => {
                SerializedParallaxMappingMethod::Relief { max_steps }
            }
        }
    }
    fn into_parallax_mapping_method(self) -> ParallaxMappingMethod {
        match self {
            SerializedParallaxMappingMethod::Occlusion => ParallaxMappingMethod::Occlusion,
            SerializedParallaxMappingMethod::Relief { max_steps } => {
                ParallaxMappingMethod::Relief { max_steps }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SerializedOpaqueRendererMethod {
    Forward,
    Deferred,
    Auto,
}

impl SerializedOpaqueRendererMethod {
    fn from_opaque_renderer_method(method: OpaqueRendererMethod) -> Self {
        match method {
            OpaqueRendererMethod::Forward => SerializedOpaqueRendererMethod::Forward,
            OpaqueRendererMethod::Deferred => SerializedOpaqueRendererMethod::Deferred,
            OpaqueRendererMethod::Auto => SerializedOpaqueRendererMethod::Auto,
        }
    }
    fn into_opaque_renderer_method(self) -> OpaqueRendererMethod {
        match self {
            SerializedOpaqueRendererMethod::Forward => OpaqueRendererMethod::Forward,
            SerializedOpaqueRendererMethod::Deferred => OpaqueRendererMethod::Deferred,
            SerializedOpaqueRendererMethod::Auto => OpaqueRendererMethod::Auto,
        }
    }
}
