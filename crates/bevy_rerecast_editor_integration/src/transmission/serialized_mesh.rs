//! Contains proxy types needed to serialize and deserialize types that need to be transmitted
//! to and from the editor.

use bevy_asset::RenderAssetUsages;
use bevy_derive::{Deref, DerefMut};
use bevy_reflect::prelude::*;
use bevy_render::mesh::{
    Indices, Mesh, MeshVertexAttributeId, PrimitiveTopology, VertexAttributeValues,
};
use serde::{Deserialize, Serialize};

/// Serialized version of [`Mesh`].
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct SerializedMesh {
    primitive_topology: SerializedPrimitiveTopology,
    attributes: Vec<(
        SerializedMeshVertexAttributeId,
        SerializedVertexAttributeValues,
    )>,
    indices: Option<SerializedIndices>,
}

impl SerializedMesh {
    /// Serializes a [`Mesh`] to a [`SerializedMesh`].
    pub fn from_mesh(mesh: &Mesh) -> Self {
        SerializedMesh {
            primitive_topology: mesh.primitive_topology().into(),
            attributes: mesh
                .attributes()
                .filter_map(|(attribute, values)| {
                    let Some(id) = attribute.id.try_into().ok() else {
                        tracing::warn!(
                            "Failed to serialize mesh: unknown attribute id: {:?}",
                            attribute.id
                        );
                        return None;
                    };
                    Some((id, values.clone().into()))
                })
                .collect(),
            indices: mesh.indices().cloned().map(|indices| indices.into()),
        }
    }

    /// Deserializes a [`SerializedMesh`] to a [`Mesh`].
    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(self.primitive_topology.into(), RenderAssetUsages::all());
        let attributes = [
            Mesh::ATTRIBUTE_POSITION,
            Mesh::ATTRIBUTE_NORMAL,
            Mesh::ATTRIBUTE_UV_0,
            Mesh::ATTRIBUTE_UV_1,
            Mesh::ATTRIBUTE_TANGENT,
            Mesh::ATTRIBUTE_COLOR,
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            Mesh::ATTRIBUTE_JOINT_INDEX,
        ];
        for (attribute, values) in self.attributes {
            // Safety: this is just a newtype wrapper around a u64, so we can safely transmute it
            let attribute_id: MeshVertexAttributeId = unsafe { std::mem::transmute(attribute) };
            let Some(attribute) = attributes
                .iter()
                .find(|attribute| attribute.id == attribute_id)
            else {
                tracing::warn!(
                    "Failed to deserialize mesh: unknown attribute id: {attribute_id:?}"
                );
                continue;
            };
            mesh.insert_attribute(*attribute, values);
        }
        if let Some(indices) = self.indices {
            mesh.insert_indices(indices.into());
        }
        mesh
    }
}

#[derive(
    Reflect,
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
)]
#[reflect(Serialize, Deserialize)]
struct SerializedMeshVertexAttributeId(u64);

impl TryFrom<MeshVertexAttributeId> for SerializedMeshVertexAttributeId {
    type Error = ();

    fn try_from(id: MeshVertexAttributeId) -> Result<Self, Self::Error> {
        // Copy-pasted the constants from bevy_mesh, don't think there's a better way to do this ATM ;-;
        if id == Mesh::ATTRIBUTE_POSITION.id {
            Ok(Self(0))
        } else if id == Mesh::ATTRIBUTE_NORMAL.id {
            Ok(Self(1))
        } else if id == Mesh::ATTRIBUTE_UV_0.id {
            Ok(Self(2))
        } else if id == Mesh::ATTRIBUTE_UV_1.id {
            Ok(Self(3))
        } else if id == Mesh::ATTRIBUTE_TANGENT.id {
            Ok(Self(4))
        } else if id == Mesh::ATTRIBUTE_COLOR.id {
            Ok(Self(5))
        } else if id == Mesh::ATTRIBUTE_JOINT_WEIGHT.id {
            Ok(Self(6))
        } else if id == Mesh::ATTRIBUTE_JOINT_INDEX.id {
            Ok(Self(7))
        } else {
            Err(())
        }
    }
}

impl TryFrom<SerializedMeshVertexAttributeId> for MeshVertexAttributeId {
    type Error = ();

    fn try_from(id: SerializedMeshVertexAttributeId) -> Result<Self, Self::Error> {
        match id {
            SerializedMeshVertexAttributeId(0) => Ok(Mesh::ATTRIBUTE_POSITION.id),
            SerializedMeshVertexAttributeId(1) => Ok(Mesh::ATTRIBUTE_NORMAL.id),
            SerializedMeshVertexAttributeId(2) => Ok(Mesh::ATTRIBUTE_UV_0.id),
            SerializedMeshVertexAttributeId(3) => Ok(Mesh::ATTRIBUTE_UV_1.id),
            SerializedMeshVertexAttributeId(4) => Ok(Mesh::ATTRIBUTE_TANGENT.id),
            SerializedMeshVertexAttributeId(5) => Ok(Mesh::ATTRIBUTE_COLOR.id),
            SerializedMeshVertexAttributeId(6) => Ok(Mesh::ATTRIBUTE_JOINT_WEIGHT.id),
            SerializedMeshVertexAttributeId(7) => Ok(Mesh::ATTRIBUTE_JOINT_INDEX.id),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
#[allow(missing_docs)]
enum SerializedVertexAttributeValues {
    Float32(Vec<f32>),
    Sint32(Vec<i32>),
    Uint32(Vec<u32>),
    Float32x2(Vec<[f32; 2]>),
    Sint32x2(Vec<[i32; 2]>),
    Uint32x2(Vec<[u32; 2]>),
    Float32x3(Vec<[f32; 3]>),
    Sint32x3(Vec<[i32; 3]>),
    Uint32x3(Vec<[u32; 3]>),
    Float32x4(Vec<[f32; 4]>),
    Sint32x4(Vec<[i32; 4]>),
    Uint32x4(Vec<[u32; 4]>),
    Sint16x2(Vec<[i16; 2]>),
    Snorm16x2(Vec<[i16; 2]>),
    Uint16x2(Vec<[u16; 2]>),
    Unorm16x2(Vec<[u16; 2]>),
    Sint16x4(Vec<[i16; 4]>),
    Snorm16x4(Vec<[i16; 4]>),
    Uint16x4(Vec<[u16; 4]>),
    Unorm16x4(Vec<[u16; 4]>),
    Sint8x2(Vec<[i8; 2]>),
    Snorm8x2(Vec<[i8; 2]>),
    Uint8x2(Vec<[u8; 2]>),
    Unorm8x2(Vec<[u8; 2]>),
    Sint8x4(Vec<[i8; 4]>),
    Snorm8x4(Vec<[i8; 4]>),
    Uint8x4(Vec<[u8; 4]>),
    Unorm8x4(Vec<[u8; 4]>),
}

impl From<VertexAttributeValues> for SerializedVertexAttributeValues {
    fn from(values: VertexAttributeValues) -> Self {
        match values {
            VertexAttributeValues::Float32(values) => Self::Float32(values),
            VertexAttributeValues::Sint32(values) => Self::Sint32(values),
            VertexAttributeValues::Uint32(values) => Self::Uint32(values),
            VertexAttributeValues::Float32x2(values) => Self::Float32x2(values),
            VertexAttributeValues::Sint32x2(values) => Self::Sint32x2(values),
            VertexAttributeValues::Uint32x2(values) => Self::Uint32x2(values),
            VertexAttributeValues::Float32x3(values) => Self::Float32x3(values),
            VertexAttributeValues::Sint32x3(values) => Self::Sint32x3(values),
            VertexAttributeValues::Uint32x3(values) => Self::Uint32x3(values),
            VertexAttributeValues::Float32x4(values) => Self::Float32x4(values),
            VertexAttributeValues::Sint32x4(values) => Self::Sint32x4(values),
            VertexAttributeValues::Uint32x4(values) => Self::Uint32x4(values),
            VertexAttributeValues::Sint16x2(values) => Self::Sint16x2(values),
            VertexAttributeValues::Snorm16x2(values) => Self::Snorm16x2(values),
            VertexAttributeValues::Uint16x2(values) => Self::Uint16x2(values),
            VertexAttributeValues::Unorm16x2(values) => Self::Unorm16x2(values),
            VertexAttributeValues::Sint16x4(values) => Self::Sint16x4(values),
            VertexAttributeValues::Snorm16x4(values) => Self::Snorm16x4(values),
            VertexAttributeValues::Uint16x4(values) => Self::Uint16x4(values),
            VertexAttributeValues::Unorm16x4(values) => Self::Unorm16x4(values),
            VertexAttributeValues::Sint8x2(values) => Self::Sint8x2(values),
            VertexAttributeValues::Snorm8x2(values) => Self::Snorm8x2(values),
            VertexAttributeValues::Uint8x2(values) => Self::Uint8x2(values),
            VertexAttributeValues::Unorm8x2(values) => Self::Unorm8x2(values),
            VertexAttributeValues::Sint8x4(values) => Self::Sint8x4(values),
            VertexAttributeValues::Snorm8x4(values) => Self::Snorm8x4(values),
            VertexAttributeValues::Uint8x4(values) => Self::Uint8x4(values),
            VertexAttributeValues::Unorm8x4(values) => Self::Unorm8x4(values),
        }
    }
}

impl From<SerializedVertexAttributeValues> for VertexAttributeValues {
    fn from(values: SerializedVertexAttributeValues) -> Self {
        match values {
            SerializedVertexAttributeValues::Float32(values) => Self::Float32(values),
            SerializedVertexAttributeValues::Sint32(values) => Self::Sint32(values),
            SerializedVertexAttributeValues::Uint32(values) => Self::Uint32(values),
            SerializedVertexAttributeValues::Float32x2(values) => Self::Float32x2(values),
            SerializedVertexAttributeValues::Sint32x2(values) => Self::Sint32x2(values),
            SerializedVertexAttributeValues::Uint32x2(values) => Self::Uint32x2(values),
            SerializedVertexAttributeValues::Float32x3(values) => Self::Float32x3(values),
            SerializedVertexAttributeValues::Sint32x3(values) => Self::Sint32x3(values),
            SerializedVertexAttributeValues::Uint32x3(values) => Self::Uint32x3(values),
            SerializedVertexAttributeValues::Float32x4(values) => Self::Float32x4(values),
            SerializedVertexAttributeValues::Sint32x4(values) => Self::Sint32x4(values),
            SerializedVertexAttributeValues::Uint32x4(values) => Self::Uint32x4(values),
            SerializedVertexAttributeValues::Sint16x2(values) => Self::Sint16x2(values),
            SerializedVertexAttributeValues::Snorm16x2(values) => Self::Snorm16x2(values),
            SerializedVertexAttributeValues::Uint16x2(values) => Self::Uint16x2(values),
            SerializedVertexAttributeValues::Unorm16x2(values) => Self::Unorm16x2(values),
            SerializedVertexAttributeValues::Sint16x4(values) => Self::Sint16x4(values),
            SerializedVertexAttributeValues::Snorm16x4(values) => Self::Snorm16x4(values),
            SerializedVertexAttributeValues::Uint16x4(values) => Self::Uint16x4(values),
            SerializedVertexAttributeValues::Unorm16x4(values) => Self::Unorm16x4(values),
            SerializedVertexAttributeValues::Sint8x2(values) => Self::Sint8x2(values),
            SerializedVertexAttributeValues::Snorm8x2(values) => Self::Snorm8x2(values),
            SerializedVertexAttributeValues::Uint8x2(values) => Self::Uint8x2(values),
            SerializedVertexAttributeValues::Unorm8x2(values) => Self::Unorm8x2(values),
            SerializedVertexAttributeValues::Sint8x4(values) => Self::Sint8x4(values),
            SerializedVertexAttributeValues::Snorm8x4(values) => Self::Snorm8x4(values),
            SerializedVertexAttributeValues::Uint8x4(values) => Self::Uint8x4(values),
            SerializedVertexAttributeValues::Unorm8x4(values) => Self::Unorm8x4(values),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
#[allow(missing_docs)]
enum SerializedIndices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl From<Indices> for SerializedIndices {
    fn from(indices: Indices) -> Self {
        match indices {
            Indices::U16(indices) => Self::U16(indices),
            Indices::U32(indices) => Self::U32(indices),
        }
    }
}

impl From<SerializedIndices> for Indices {
    fn from(indices: SerializedIndices) -> Self {
        match indices {
            SerializedIndices::U16(indices) => Self::U16(indices),
            SerializedIndices::U32(indices) => Self::U32(indices),
        }
    }
}

#[derive(Reflect, Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
enum SerializedPrimitiveTopology {
    PointList = 0,
    LineList = 1,
    LineStrip = 2,
    #[default]
    TriangleList = 3,
    TriangleStrip = 4,
}

impl From<PrimitiveTopology> for SerializedPrimitiveTopology {
    fn from(topology: PrimitiveTopology) -> Self {
        match topology {
            PrimitiveTopology::PointList => Self::PointList,
            PrimitiveTopology::LineList => Self::LineList,
            PrimitiveTopology::LineStrip => Self::LineStrip,
            PrimitiveTopology::TriangleList => Self::TriangleList,
            PrimitiveTopology::TriangleStrip => Self::TriangleStrip,
        }
    }
}

impl From<SerializedPrimitiveTopology> for PrimitiveTopology {
    fn from(topology: SerializedPrimitiveTopology) -> Self {
        match topology {
            SerializedPrimitiveTopology::PointList => PrimitiveTopology::PointList,
            SerializedPrimitiveTopology::LineList => PrimitiveTopology::LineList,
            SerializedPrimitiveTopology::LineStrip => PrimitiveTopology::LineStrip,
            SerializedPrimitiveTopology::TriangleList => PrimitiveTopology::TriangleList,
            SerializedPrimitiveTopology::TriangleStrip => PrimitiveTopology::TriangleStrip,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use bevy_render::{mesh::Indices, render_asset::RenderAssetUsages};
    use bevy_rerecast_core::TriMeshFromBevyMesh as _;
    use rerecast::TriMesh;

    use super::*;

    #[test]
    fn roundtrip_trimesh() {
        let mesh = star();
        let serialized_mesh = SerializedMesh::from_mesh(&mesh);
        let deserialized_mesh = serialized_mesh.into_mesh();
        let trimesh = TriMesh::from_mesh(&deserialized_mesh).unwrap();

        let expected_pos = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        assert_eq!(expected_pos.len(), trimesh.vertices.len(), "vertex len");

        let Indices::U32(expected_indices) = mesh.indices().unwrap() else {
            panic!("Expected U32 indices");
        };
        assert_eq!(
            expected_indices.len() / 3,
            trimesh.indices.len(),
            "index len"
        );

        for (expected_vert, vert) in expected_pos.iter().zip(trimesh.vertices.iter()) {
            assert_eq!(expected_vert, &vert.to_array());
        }

        for (expected_index, index) in expected_indices.chunks(3).zip(trimesh.indices.iter()) {
            assert_eq!(expected_index, index.to_array());
        }
    }

    /// Taken from <https://bevy.org/examples/2d-rendering/mesh2d-manual/>
    fn star() -> Mesh {
        let mut star = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

        let mut v_pos = vec![[0.0, 0.0, 0.0]];
        for i in 0..10 {
            let a = i as f32 * PI / 5.0;
            let r = (1 - i % 2) as f32 * 100.0 + 100.0;
            v_pos.push([r * a.sin(), r * a.cos(), 0.0]);
        }
        star.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

        let mut indices = vec![0, 1, 10];
        for i in 2..=10 {
            indices.extend_from_slice(&[0, i, i - 1]);
        }
        star.insert_indices(Indices::U32(indices));
        star
    }
}
