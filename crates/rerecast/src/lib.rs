#![doc = include_str!("../../../readme.md")]

mod compact_cell;
mod compact_heightfield;
mod compact_span;
mod config;
mod contours;
mod detail_mesh;
mod erosion;
mod heightfield;
mod mark_convex_poly_area;
pub(crate) mod math;
mod poly_mesh;
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
pub use config::NavmeshConfig;
pub use contours::{BuildContoursFlags, Contour, ContourSet, RegionVertexId};
pub use detail_mesh::DetailNavmesh;
pub use heightfield::{Heightfield, HeightfieldBuilder, HeightfieldBuilderError};
pub use mark_convex_poly_area::ConvexVolume;
pub use math::{Aabb2d, Aabb3d};
pub use poly_mesh::{PolygonMesh, RC_MESH_NULL_IDX};
pub use region::RegionId;
pub use span::{AreaType, Span, SpanKey, Spans};
pub use trimesh::TriMesh;
