use glam::{IVec3, Vec3A};

use crate::{Aabb3d, AreaType, CompactHeightfield};

impl CompactHeightfield {
    /// Sets the [`AreaType`] of the spans within the given convex volume.
    pub fn mark_convex_poly_area(&mut self, volume: ConvexVolume) {
        // Compute the bounding box of the polygon
        let Some(mut aabb) = Aabb3d::from_verts(&volume.vertices) else {
            // The volume is empty
            return;
        };
        aabb.min.y = volume.min_y;
        aabb.max.y = volume.max_y;

        // Compute the grid footprint of the polygon
        let mut min = aabb.min - self.aabb.min;
        min.x /= self.cell_size;
        min.y /= self.cell_height;
        min.z /= self.cell_size;
        let mut max = aabb.max - self.aabb.min;
        max.x /= self.cell_size;
        max.y /= self.cell_height;
        max.z /= self.cell_size;
        let mut min = IVec3::new(min.x as i32, min.y as i32, min.z as i32);
        let mut max = IVec3::new(max.x as i32, max.y as i32, max.z as i32);

        // Early-out if the polygon lies entirely outside the grid.
        if max.x < 0 || min.x >= self.width as i32 || max.z < 0 || min.z >= self.height as i32 {
            return;
        }

        // Clamp the polygon footprint to the grid
        min.x = min.x.max(0);
        max.x = max.x.min(self.width as i32 - 1);
        min.z = min.z.max(0);
        max.z = max.z.min(self.height as i32 - 1);

        // Jan: This comment is taken from the original
        // TODO: Optimize.
        for z in min.z..=max.z {
            for x in min.x..=max.x {
                let cell_index = (x + z * self.width as i32) as usize;
                let cell = &self.cells[cell_index];
                let max_index = cell.index() as usize + cell.count() as usize;
                #[expect(clippy::needless_range_loop)]
                for i in cell.index() as usize..max_index {
                    let span = &self.spans[i];

                    // Skip if  span is removed.
                    if !self.areas[i].is_walkable() {
                        continue;
                    }

                    // Skip if y extents don't overlap.
                    if (span.y as i32) < min.y || (span.y as i32) > max.y {
                        continue;
                    }

                    let point = Vec3A::new(
                        self.aabb.min.x + (x as f32 + 0.5) * self.cell_size,
                        0.0,
                        self.aabb.min.z + (z as f32 + 0.5) * self.cell_size,
                    );
                    if point_in_poly(&point, &volume.vertices) {
                        self.areas[i] = volume.area;
                    }
                }
            }
        }
    }
}

fn point_in_poly(point: &Vec3A, vertices: &[Vec3A]) -> bool {
    let mut inside = false;
    let mut j = vertices.len() - 1;
    for i in 0..vertices.len() {
        let xi = vertices[i].x;
        let yi = vertices[i].z;
        let xj = vertices[j].x;
        let yj = vertices[j].z;
        if ((yi > point.z) != (yj > point.z))
            && (point.x < (xj - xi) * (point.z - yi) / (yj - yi) + xi)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub struct ConvexVolume {
    pub vertices: Vec<Vec3A>,
    pub min_y: f32,
    pub max_y: f32,
    pub area: AreaType,
}
