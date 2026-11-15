//! Duplicates of [`rerecast`] types for remote reflection.

use alloc::vec::Vec;
use bevy_reflect::{prelude::*, reflect_remote};
use glam::{U16Vec3, Vec2, Vec3};

#[reflect_remote(rerecast::PolygonNavmesh)]
pub(crate) struct PolygonNavmesh {
    vertices: Vec<U16Vec3>,
    polygons: Vec<u16>,
    polygon_neighbors: Vec<u16>,
    flags: Vec<u16>,
    #[reflect(remote = RemoteVec<RemoteRegionId>)]
    regions: Vec<rerecast::RegionId>,
    #[reflect(remote = RemoteVec<RemoteAreaType>)]
    areas: Vec<rerecast::AreaType>,
    max_vertices_per_polygon: u16,
    #[reflect(remote = RemoteAabb3d)]
    aabb: rerecast::Aabb3d,
    cell_size: f32,
    cell_height: f32,
    border_size: u16,
    max_edge_error: f32,
}

#[reflect_remote(rerecast::RegionId)]
pub(crate) struct RemoteRegionId(u16);

#[reflect_remote(rerecast::AreaType)]
pub(crate) struct RemoteAreaType(u8);

#[reflect_remote(rerecast::Aabb3d)]
pub(crate) struct RemoteAabb3d {
    min: Vec3,
    max: Vec3,
}

#[reflect_remote(rerecast::BuildContoursFlags)]
pub(crate) struct RemoteBuildContoursFlags(u8);

#[reflect_remote(rerecast::ConvexVolume)]
pub(crate) struct RemoteConvexVolume {
    vertices: Vec<Vec2>,
    min_y: f32,
    max_y: f32,
    #[reflect(remote = RemoteAreaType)]
    area: rerecast::AreaType,
}

#[reflect_remote(rerecast::DetailNavmesh)]
pub(crate) struct RemoteDetailNavmesh {
    #[reflect(remote = RemoteVec<RemoteSubMesh>)]
    meshes: Vec<rerecast::SubMesh>,
    vertices: Vec<Vec3>,
    triangles: Vec<[u8; 3]>,
    triangle_flags: Vec<u8>,
}

#[reflect_remote(rerecast::SubMesh)]
pub(crate) struct RemoteSubMesh {
    base_vertex_index: u32,
    vertex_count: u32,
    base_triangle_index: u32,
    triangle_count: u32,
}

/// Dirty hack to have reflection for `Vec<RemoteType>`
#[derive(Reflect)]
#[repr(transparent)]
pub(crate) struct RemoteVec<W: bevy_reflect::ReflectRemote>(Vec<W>);

impl<W: bevy_reflect::ReflectRemote> RemoteVec<W> {
    const LAYOUT_OK: () = assert!(
        size_of::<W>() == size_of::<W::Remote>() && align_of::<W>() == align_of::<W::Remote>(),
        "RemoteVec wrapper must be layout-compatible with its remote type"
    );

    fn cast_vec<A, B>(v: Vec<A>) -> Vec<B> {
        let () = Self::LAYOUT_OK;
        let mut v = core::mem::ManuallyDrop::new(v);
        // SAFETY: `A` and `B` have identical size and alignment (checked above).
        unsafe { Vec::from_raw_parts(v.as_mut_ptr().cast(), v.len(), v.capacity()) }
    }
}

impl<W> bevy_reflect::ReflectRemote for RemoteVec<W>
where
    W: bevy_reflect::ReflectRemote
        + FromReflect
        + bevy_reflect::GetTypeRegistration
        + bevy_reflect::Typed,
    Self: Reflect,
{
    type Remote = Vec<W::Remote>;

    fn as_remote(&self) -> &Self::Remote {
        let () = Self::LAYOUT_OK;
        // SAFETY: `Self` is transparent over `Vec<W>`, and `W` is layout-compatible with `W::Remote`.
        unsafe { &*(self as *const Self).cast::<Self::Remote>() }
    }

    fn as_remote_mut(&mut self) -> &mut Self::Remote {
        let () = Self::LAYOUT_OK;
        // SAFETY: see `as_remote`
        unsafe { &mut *(self as *mut Self).cast::<Self::Remote>() }
    }

    fn into_remote(self) -> Self::Remote {
        Self::cast_vec(self.0)
    }

    fn as_wrapper(remote: &Self::Remote) -> &Self {
        let () = Self::LAYOUT_OK;
        // SAFETY: see `as_remote`
        unsafe { &*(remote as *const Self::Remote).cast::<Self>() }
    }

    fn as_wrapper_mut(remote: &mut Self::Remote) -> &mut Self {
        let () = Self::LAYOUT_OK;
        // SAFETY: see `as_remote`
        unsafe { &mut *(remote as *mut Self::Remote).cast::<Self>() }
    }

    fn into_wrapper(remote: Self::Remote) -> Self {
        Self(Self::cast_vec(remote))
    }
}
