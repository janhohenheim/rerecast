//! Contains methods for rasterizing triangles of a [`TrimeshedCollider`] into a [`Heightfield`].

use bevy::math::Vec3A;

use crate::{heightfield::Heightfield, span::AreaType, trimesh::TrimeshedCollider};

impl TrimeshedCollider {
    /// Rasterizes the trimesh into a [`Heightfield`].
    pub fn rasterize(self, _heightfield: &mut Heightfield) -> Heightfield {
        todo!()
    }

    pub fn mark_walkable_triangles(&mut self, threshold_rad: f32) {
        let threshold_cos = threshold_rad.cos();
        for (i, indices) in self.indices.iter().enumerate() {
            let normal = indices.normal(&self.vertices);

            if normal.y > threshold_cos {
                self.area_types[i] = AreaType::WALKABLE;
            }
        }
    }
}

trait TriangleIndices {
    fn normal(&self, vertices: &[Vec3A]) -> Vec3A;
}

impl TriangleIndices for [u32; 3] {
    fn normal(&self, vertices: &[Vec3A]) -> Vec3A {
        let a = vertices[self[0] as usize];
        let b = vertices[self[1] as usize];
        let c = vertices[self[2] as usize];
        let ab = b - a;
        let ac = c - a;
        ab.cross(ac).normalize_or_zero()
    }
}
