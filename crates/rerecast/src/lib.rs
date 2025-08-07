//! Rust port of [Recast](https://github.com/recastnavigation/recastnavigation), the industry-standard navigation mesh generator used
//! by Unreal, Unity, Godot, and other game engines.
//!
//! This crate contains low-level API.
//! See the [`rerecast` repo](https://github.com/janhohenheim/rerecast) for instructions for high-level engine integrations

#![no_std]
#[macro_use]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

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
pub(crate) mod ops;
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
pub use config::{Config, ConfigBuilder};
pub use contours::{BuildContoursFlags, Contour, ContourSet, RegionVertexId};
pub use detail_mesh::{DetailNavmesh, SubMesh};
pub use heightfield::{Heightfield, HeightfieldBuilder, HeightfieldBuilderError};
pub use mark_convex_poly_area::ConvexVolume;
pub use math::{Aabb2d, Aabb3d};
pub use poly_mesh::PolygonNavmesh;
pub use region::RegionId;
pub use span::{AreaType, Span, SpanKey, Spans};
pub use trimesh::TriMesh;
