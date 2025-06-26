use crate::{
    CompactHeightfield,
    math::{dir_offset_x, dir_offset_z},
};

impl CompactHeightfield {
    /// Erode the walkable area by agent radius.
    pub fn erode_walkable_area(&mut self, walkable_radius: u16) {
        let mut distance_to_boundary = vec![u8::MAX; self.spans.len()];

        // Mark boundary cells.
        for z in 0..self.height {
            for x in 0..self.width {
                let cell = self.cell_at(x, z);
                let max_span_index = cell.index() as usize + cell.count() as usize;
                for span_index in cell.index() as usize..max_span_index {
                    if !self.areas[span_index].is_walkable() {
                        distance_to_boundary[span_index] = 0;
                        continue;
                    }
                    let span = &self.spans[span_index];
                    // Check that there is a non-null adjacent span in each of the 4 cardinal directions.
                    let mut neighbor_count = 0;
                    for direction in 0..4 {
                        let Some(neighbor_connection) = span.con(direction) else {
                            break;
                        };
                        let neighbor_x = x as i32 + dir_offset_x(direction) as i32;
                        let neighbor_z = z as i32 + dir_offset_z(direction) as i32;
                        let neighbor_span_index =
                            self.cell_at(neighbor_x as u16, neighbor_z as u16).index() as usize
                                + neighbor_connection as usize;

                        if !self.areas[neighbor_span_index].is_walkable() {
                            break;
                        }
                        neighbor_count += 1;
                    }

                    // At least one missing neighbour, so this is a boundary cell.
                    if neighbor_count != 4 {
                        distance_to_boundary[span_index] = 0;
                    }
                }
            }
        }
    }
}
