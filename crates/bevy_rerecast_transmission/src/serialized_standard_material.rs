use bevy_asset::Assets;
use bevy_color::prelude::*;
use bevy_image::Image;
use bevy_math::Affine2;
use bevy_pbr::{OpaqueRendererMethod, UvChannel, prelude::*};
use bevy_render::alpha::AlphaMode;
use serde::{Deserialize, Serialize};
use wgpu_types::Face;

use crate::SerializedImage;

/// Serialized representation of a [`StandardMaterial`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedStandardMaterial {
    base_color: Color,
    base_color_channel: SerializedUvChannel,
    base_color_texture: Option<SerializedImage>,
    emissive: LinearRgba,
    emissive_exposure_weight: f32,
    emissive_channel: SerializedUvChannel,
    emissive_texture: Option<SerializedImage>,
    perceptual_roughness: f32,
    metallic: f32,
    metallic_roughness_channel: SerializedUvChannel,
    metallic_roughness_texture: Option<SerializedImage>,
    reflectance: f32,
    specular_tint: Color,
    diffuse_transmission: f32,
    #[cfg(feature = "pbr_transmission_textures")]
    diffuse_transmission_channel: UvChannel,
    #[cfg(feature = "pbr_transmission_textures")]
    diffuse_transmission_texture: Option<SerializedImage>,
    specular_transmission: f32,
    #[cfg(feature = "pbr_transmission_textures")]
    specular_transmission_channel: UvChannel,
    #[cfg(feature = "pbr_transmission_textures")]
    specular_transmission_texture: Option<SerializedImage>,
    thickness: f32,
    #[cfg(feature = "pbr_transmission_textures")]
    thickness_channel: UvChannel,
    #[cfg(feature = "pbr_transmission_textures")]
    thickness_texture: Option<SerializedImage>,
    ior: f32,
    attenuation_distance: f32,
    attenuation_color: Color,
    normal_map_channel: SerializedUvChannel,
    normal_map_texture: Option<SerializedImage>,
    flip_normal_map_y: bool,
    occlusion_channel: SerializedUvChannel,
    occlusion_texture: Option<SerializedImage>,
    #[cfg(feature = "pbr_specular_textures")]
    specular_channel: UvChannel,
    #[cfg(feature = "pbr_specular_textures")]
    specular_texture: Option<SerializedImage>,
    #[cfg(feature = "pbr_specular_textures")]
    specular_tint_channel: UvChannel,
    #[cfg_attr(feature = "pbr_specular_textures", texture(29))]
    #[cfg_attr(feature = "pbr_specular_textures", sampler(30))]
    #[cfg(feature = "pbr_specular_textures")]
    specular_tint_texture: Option<SerializedImage>,
    clearcoat: f32,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_texture: Option<SerializedImage>,
    clearcoat_perceptual_roughness: f32,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_roughness_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_roughness_texture: Option<SerializedImage>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_normal_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_normal_texture: Option<SerializedImage>,
    anisotropy_strength: f32,
    anisotropy_rotation: f32,
    #[cfg(feature = "pbr_anisotropy_texture")]
    anisotropy_channel: UvChannel,
    #[cfg(feature = "pbr_anisotropy_texture")]
    anisotropy_texture: Option<SerializedImage>,
    double_sided: bool,
    cull_mode: Option<Face>,
    unlit: bool,
    fog_enabled: bool,
    alpha_mode: SerializedAlphaMode,
    depth_bias: f32,
    depth_map: Option<SerializedImage>,
    parallax_depth_scale: f32,
    parallax_mapping_method: SerializedParallaxMappingMethod,
    max_parallax_layer_count: f32,
    lightmap_exposure: f32,
    opaque_render_method: SerializedOpaqueRendererMethod,
    deferred_lighting_pass_id: u8,
    uv_transform: Affine2,
}

impl SerializedStandardMaterial {
    /// Serialize a [`StandardMaterial`] into a [`SerializedStandardMaterial`]. Returns `None` if any of the images are not found in `images`.
    pub fn try_from_standard_material(
        material: StandardMaterial,
        images: &Assets<Image>,
    ) -> Option<Self> {
        Some(Self {
            base_color: material.base_color,
            base_color_channel: SerializedUvChannel::from_uv_channel(material.base_color_channel),
            base_color_texture: if let Some(image) = material.base_color_texture {
                let Some(image) = images.get(&image) else {
                    return None;
                };
                Some(SerializedImage::from_image(image.clone()))
            } else {
                None
            },
            emissive: material.emissive,
            emissive_exposure_weight: material.emissive_exposure_weight,
            emissive_channel: SerializedUvChannel::from_uv_channel(material.emissive_channel),
            emissive_texture: if let Some(image) = material.emissive_texture {
                let Some(image) = images.get(&image) else {
                    return None;
                };
                Some(SerializedImage::from_image(image.clone()))
            } else {
                None
            },
            perceptual_roughness: material.perceptual_roughness,
            metallic: material.metallic,
            metallic_roughness_channel: SerializedUvChannel::from_uv_channel(
                material.metallic_roughness_channel,
            ),
            metallic_roughness_texture: if let Some(image) = material.metallic_roughness_texture {
                let Some(image) = images.get(&image) else {
                    return None;
                };
                Some(SerializedImage::from_image(image.clone()))
            } else {
                None
            },
            reflectance: material.reflectance,
            specular_tint: material.specular_tint,
            diffuse_transmission: material.diffuse_transmission,
            specular_transmission: material.specular_transmission,
            thickness: material.thickness,
            ior: material.ior,
            attenuation_distance: material.attenuation_distance,
            attenuation_color: material.attenuation_color,
            normal_map_channel: SerializedUvChannel::from_uv_channel(material.normal_map_channel),
            normal_map_texture: if let Some(image) = material.normal_map_texture {
                let Some(image) = images.get(&image) else {
                    return None;
                };
                Some(SerializedImage::from_image(image.clone()))
            } else {
                None
            },
            flip_normal_map_y: material.flip_normal_map_y,
            occlusion_channel: SerializedUvChannel::from_uv_channel(material.occlusion_channel),
            occlusion_texture: if let Some(image) = material.occlusion_texture {
                let Some(image) = images.get(&image) else {
                    return None;
                };
                Some(SerializedImage::from_image(image.clone()))
            } else {
                None
            },
            clearcoat: material.clearcoat,
            clearcoat_perceptual_roughness: material.clearcoat_perceptual_roughness,
            anisotropy_strength: material.anisotropy_strength,
            anisotropy_rotation: material.anisotropy_rotation,
            double_sided: material.double_sided,
            cull_mode: material.cull_mode,
            unlit: material.unlit,
            fog_enabled: material.fog_enabled,
            alpha_mode: SerializedAlphaMode::from_alpha_mode(material.alpha_mode),
            depth_bias: material.depth_bias,
            depth_map: if let Some(image) = material.depth_map {
                let Some(image) = images.get(&image) else {
                    return None;
                };
                Some(SerializedImage::from_image(image.clone()))
            } else {
                None
            },
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
    pub fn into_standard_material(self, images: &mut Assets<Image>) -> StandardMaterial {
        StandardMaterial {
            base_color: self.base_color,
            base_color_channel: self.base_color_channel.into_uv_channel(),
            base_color_texture: self
                .base_color_texture
                .map(|image| images.add(image.into_image())),
            emissive: self.emissive,
            emissive_exposure_weight: self.emissive_exposure_weight,
            emissive_channel: self.emissive_channel.into_uv_channel(),
            emissive_texture: self
                .emissive_texture
                .map(|image| images.add(image.into_image())),
            perceptual_roughness: self.perceptual_roughness,
            metallic: self.metallic,
            metallic_roughness_channel: self.metallic_roughness_channel.into_uv_channel(),
            metallic_roughness_texture: self
                .metallic_roughness_texture
                .map(|image| images.add(image.into_image())),
            reflectance: self.reflectance,
            specular_tint: self.specular_tint,
            diffuse_transmission: self.diffuse_transmission,
            specular_transmission: self.specular_transmission,
            thickness: self.thickness,
            ior: self.ior,
            attenuation_distance: self.attenuation_distance,
            attenuation_color: self.attenuation_color,
            normal_map_channel: self.normal_map_channel.into_uv_channel(),
            normal_map_texture: self
                .normal_map_texture
                .map(|image| images.add(image.into_image())),
            flip_normal_map_y: self.flip_normal_map_y,
            occlusion_channel: self.occlusion_channel.into_uv_channel(),
            occlusion_texture: self
                .occlusion_texture
                .map(|image| images.add(image.into_image())),
            clearcoat: self.clearcoat,
            clearcoat_perceptual_roughness: self.clearcoat_perceptual_roughness,
            anisotropy_strength: self.anisotropy_strength,
            anisotropy_rotation: self.anisotropy_rotation,
            double_sided: self.double_sided,
            cull_mode: self.cull_mode,
            unlit: self.unlit,
            fog_enabled: self.fog_enabled,
            alpha_mode: self.alpha_mode.into_alpha_mode(),
            depth_bias: self.depth_bias,
            depth_map: self.depth_map.map(|image| images.add(image.into_image())),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SerializedUvChannel {
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
