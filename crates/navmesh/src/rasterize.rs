//! Contains methods for rasterizing triangles of a [`TrimeshedCollider`] into a [`Heightfield`].

use bevy::math::{Dir3, InvalidDirectionError, primitives::Triangle3d};

use crate::{
    heightfield::Heightfield,
    span::AreaType,
    trimesh::{ToTrimesh, TrimeshedCollider},
};

impl TrimeshedCollider {
    /// Rasterizes the trimesh into a [`Heightfield`].
    pub fn rasterize(&self, heightfield: &mut Heightfield) -> Heightfield {
        let area_types = mark_walkable_triangles(self).expect("Triangle is degenerate");

        todo!()
    }
}

fn mark_walkable_triangles(
    trimesh: &TrimeshedCollider,
) -> Result<Vec<AreaType>, InvalidDirectionError> {
    let mut walkable_triangles = vec![AreaType::NOT_WALKABLE; trimesh.indices.len()];
    let verts = &trimesh.vertices;
    for (i, [a, b, c]) in trimesh.indices.iter().enumerate() {
        let a = verts[*a as usize * 3];
        let b = verts[*b as usize * 3];
        let c = verts[*c as usize * 3];
        todo!("compile lol");
        /*
        let tri = Triangle3d::new(a, b, c);
        todo!("Triangle3d doesn't know about SIMD types?");
        let normal = tri.normal()?;

        if normal.y > 0.0 {
            walkable_triangles[i] = AreaType::WALKABLE;
        } */
    }
    Ok(walkable_triangles)
}
