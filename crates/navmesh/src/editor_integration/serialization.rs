//! Contains proxy types needed to serialize and deserialize types that need to be transmitted
//! to and from the editor.

use bevy::{
    prelude::*,
    render::mesh::{Indices, MeshVertexAttribute, VertexAttributeValues, VertexFormat},
};
use serde::{Deserialize, Serialize};
use wgpu_types::PrimitiveTopology;

/// Proxy of [`Mesh`](bevy::render::mesh::Mesh).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMesh {
    /// Topology of the primitives.
    pub primitive_topology: ProxyPrimitiveTopology,
    /// attributes in the form that [`Mesh::insert_attribute`] expects
    attributes: Vec<(ProxyMeshVertexAttribute, ProxyVertexAttributeValues)>,
    indices: Option<ProxyIndices>,
}

pub(crate) trait CloneProxy {
    fn clone_proxy(&self) -> ProxyMesh;
}

impl CloneProxy for Mesh {
    fn clone_proxy(&self) -> ProxyMesh {
        ProxyMesh {
            primitive_topology: self.primitive_topology().into(),
            attributes: self
                .attributes()
                .map(|(attribute, values)| (attribute.clone().into(), values.clone().into()))
                .collect(),
            indices: self.indices().cloned().map(|indices| indices.into()),
        }
    }
}

/// Proxy of [`MeshVertexAttributeId`](bevy::render::mesh::MeshVertexAttributeId).
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
pub struct ProxyMeshVertexAttributeId(pub u64);

/// Proxy of [`MeshVertexAttribute`](bevy::render::mesh::MeshVertexAttribute).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMeshVertexAttribute {
    /// The friendly name of the vertex attribute
    pub name: String,

    /// The _unique_ id of the vertex attribute. This will also determine sort ordering
    /// when generating vertex buffers. Built-in / standard attributes will use "close to zero"
    /// indices. When in doubt, use a random / very large u64 to avoid conflicts.
    pub id: ProxyMeshVertexAttributeId,

    /// The format of the vertex attribute.
    pub format: VertexFormat,
}

impl From<MeshVertexAttribute> for ProxyMeshVertexAttribute {
    fn from(attribute: MeshVertexAttribute) -> Self {
        // Safety: this is just a newtype wrapper around a u64, so we can safely transmute it
        let id: u64 = unsafe { std::mem::transmute(attribute.id) };
        Self {
            name: attribute.name.to_string(),
            id: ProxyMeshVertexAttributeId(id),
            format: attribute.format,
        }
    }
}

/// Proxy of [`VertexAttributeValues`](bevy::render::mesh::VertexAttributeValues).
/// Contains an array where each entry describes a property of a single vertex.
/// Matches the [`VertexFormats`](VertexFormat).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ProxyVertexAttributeValues {
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

impl From<VertexAttributeValues> for ProxyVertexAttributeValues {
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

/// Proxy of [`Indices`](bevy::render::mesh::Indices).
/// An array of indices into the [`VertexAttributeValues`](super::VertexAttributeValues) for a mesh.
///
/// It describes the order in which the vertex attributes should be joined into faces.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ProxyIndices {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl From<Indices> for ProxyIndices {
    fn from(indices: Indices) -> Self {
        match indices {
            Indices::U16(indices) => Self::U16(indices),
            Indices::U32(indices) => Self::U32(indices),
        }
    }
}

/// Proxy of [`PrimitiveTopology`](bevy::render::mesh::PrimitiveTopology).
/// Primitive type the input mesh is composed of.
///
/// Corresponds to [WebGPU `GPUPrimitiveTopology`](
/// https://gpuweb.github.io/gpuweb/#enumdef-gpuprimitivetopology).
#[derive(Reflect, Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum ProxyPrimitiveTopology {
    /// Vertex data is a list of points. Each vertex is a new point.
    PointList = 0,
    /// Vertex data is a list of lines. Each pair of vertices composes a new line.
    ///
    /// Vertices `0 1 2 3` create two lines `0 1` and `2 3`
    LineList = 1,
    /// Vertex data is a strip of lines. Each set of two adjacent vertices form a line.
    ///
    /// Vertices `0 1 2 3` create three lines `0 1`, `1 2`, and `2 3`.
    LineStrip = 2,
    /// Vertex data is a list of triangles. Each set of 3 vertices composes a new triangle.
    ///
    /// Vertices `0 1 2 3 4 5` create two triangles `0 1 2` and `3 4 5`
    #[default]
    TriangleList = 3,
    /// Vertex data is a triangle strip. Each set of three adjacent vertices form a triangle.
    ///
    /// Vertices `0 1 2 3 4 5` create four triangles `0 1 2`, `2 1 3`, `2 3 4`, and `4 3 5`
    TriangleStrip = 4,
}

impl From<PrimitiveTopology> for ProxyPrimitiveTopology {
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
