use bevy::{math::bounding::Aabb3d, prelude::*};

pub(crate) trait TriangleIndices {
    fn normal(&self, vertices: &[Vec3A]) -> Vec3A;
}

impl TriangleIndices for UVec3 {
    #[inline]
    fn normal(&self, vertices: &[Vec3A]) -> Vec3A {
        let a = vertices[self[0] as usize];
        let b = vertices[self[1] as usize];
        let c = vertices[self[2] as usize];
        let ab = b - a;
        let ac = c - a;
        ab.cross(ac).normalize_or_zero()
    }
}

pub(crate) trait TriangleVertices {
    fn aabb(&self) -> Aabb3d;
}

impl TriangleVertices for [Vec3A; 3] {
    #[inline]
    fn aabb(&self) -> Aabb3d {
        let min = self[0].min(self[1]).min(self[2]);
        let max = self[0].max(self[1]).max(self[2]);
        Aabb3d { min, max }
    }
}

/// Gets the standard width (x-axis) offset for the specified direction.
/// # Arguments
/// - `direction`: The direction. [Limits: 0 <= value < 4]
/// # Returns
///
/// The width offset to apply to the current cell position to move in the direction.
pub(crate) fn dir_offset_x(direction: u8) -> i8 {
    const OFFSET: [i8; 4] = [-1, 0, 1, 0];
    OFFSET[direction as usize & 0x03]
}

/// Gets the standard height (z-axis) offset for the specified direction.
/// # Arguments
/// - `direction`: The direction. [Limits: 0 <= value < 4]
/// # Returns
///
/// The height offset to apply to the current cell position to move in the direction.
pub(crate) fn dir_offset_z(direction: u8) -> i8 {
    const OFFSET: [i8; 4] = [0, 1, 0, -1];
    OFFSET[direction as usize & 0x03]
}
