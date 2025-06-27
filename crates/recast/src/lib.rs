#![doc = include_str!("../../../readme.md")]

mod compact_cell;
mod compact_heightfield;
mod compact_span;
mod erosion;
mod heightfield;
mod main_api;
pub(crate) mod math;
mod pre_filter;
mod rasterize;
mod region;
mod span;
mod trimesh;
mod watershed;

pub use compact_cell::CompactCell;
pub use compact_heightfield::CompactHeightfield;
pub use compact_span::CompactSpan;
pub use heightfield::{Heightfield, HeightfieldBuilder, HeightfieldBuilderError};
pub use region::Region;
pub use span::{AreaType, Span, SpanKey};
pub use trimesh::TriMesh;
