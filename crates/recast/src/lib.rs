#![doc = include_str!("../../../readme.md")]

mod compact_cell;
mod compact_heightfield;
mod compact_span;
mod erosion;
mod heightfield;
mod main_api;
mod mark_convex_poly_area;
pub(crate) mod math;
mod pre_filter;
mod rasterize;
mod region;
mod span;
mod trimesh;
mod watershed_build_regions;
mod watershed_distance_field;

pub use compact_cell::CompactCell;
pub use compact_heightfield::CompactHeightfield;
pub use compact_span::CompactSpan;
pub use heightfield::{Heightfield, HeightfieldBuilder, HeightfieldBuilderError};
pub use mark_convex_poly_area::ConvexVolume;
pub use math::{Aabb2d, Aabb3d};
pub use region::RegionId;
pub use span::{AreaType, Span, SpanKey, Spans};
pub use trimesh::TriMesh;
