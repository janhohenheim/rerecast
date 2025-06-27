use crate::{
    Aabb3d,
    compact_cell::CompactCell,
    compact_span::CompactSpan,
    heightfield::Heightfield,
    math::{dir_offset_x, dir_offset_z},
    region::Region,
    span::AreaType,
};

/// A packed representation of a [`Heightfield`].
#[derive(Debug, Clone)]
pub struct CompactHeightfield {
    /// The width of the heightfield along the x-axis in cell units
    pub width: u16,
    /// The height of the heightfield along the z-axis in cell units
    pub height: u16,
    /// The walkable height used during the build of the field
    pub walkable_height: u16,
    /// The walkable climb used during the build of the field.
    pub walkable_climb: u16,
    /// The AABB border size used during the build of the field.
    pub border_size: u16,
    /// The maximum distance value of any span within the field.
    pub max_distance: u16,
    /// The maximum region id of any span within the field.
    pub max_region: Region,
    /// The AABB of the heightfield
    pub aabb: Aabb3d,
    /// The size of each cell on the xz-plane
    pub cell_size: f32,
    /// The size of each cell along the y-axis
    pub cell_height: f32,
    /// The cells in the heightfield [Size: `width * height`]
    pub cells: Vec<CompactCell>,
    /// All walkable spans in the heightfield
    pub spans: Vec<CompactSpan>,
    /// Vector containing border distance data. [Size: `spans.len()`]
    pub dist: Vec<u16>,
    /// Vector containing area type data. [Size: `spans.len()`]
    pub areas: Vec<AreaType>,
}

impl CompactHeightfield {
    const MAX_HEIGHT: u16 = u16::MAX;

    /// Builds a compact heightfield from a heightfield.
    ///
    /// # Errors
    ///
    /// Returns an error if the heightfield has too many layers.
    pub fn from_heightfield(
        heightfield: Heightfield,
        walkable_height: u16,
        walkable_climb: u16,
    ) -> Result<Self, CompactHeightfieldError> {
        let walkable_span_count = heightfield
            .allocated_spans
            .values()
            .filter(|span| span.area().is_walkable())
            .count();

        let mut compact_heightfield = Self {
            width: heightfield.width,
            height: heightfield.height,
            walkable_height,
            walkable_climb,
            border_size: 0,
            aabb: heightfield.aabb,
            max_distance: 0,
            max_region: Region::NONE,
            cell_size: heightfield.cell_size,
            cell_height: heightfield.cell_height,
            cells: vec![
                CompactCell::default();
                heightfield.width as usize * heightfield.height as usize
            ],
            spans: vec![CompactSpan::default(); walkable_span_count],
            dist: vec![],
            areas: vec![AreaType::NOT_WALKABLE; walkable_span_count],
        };
        compact_heightfield.aabb.max.y += walkable_height as f32 * compact_heightfield.cell_height;

        let mut cell_index = 0_usize;
        // Fill in cells and spans
        for z in 0..heightfield.height {
            for x in 0..heightfield.width {
                let Some(span_key) = heightfield.span_key_at(x, z) else {
                    // If there are no spans at this cell, just leave the data to index=0, count=0.
                    continue;
                };
                let mut span_key_iter = Some(span_key);
                let column_index = heightfield.column_index(x, z);

                let cell = &mut compact_heightfield.cells[column_index];
                cell.set_index(cell_index as u32);
                cell.set_count(0);

                while let Some(span_key) = span_key_iter {
                    let span = heightfield.span(span_key);
                    span_key_iter = span.next();
                    if !span.area().is_walkable() {
                        continue;
                    }
                    let bot = span.max();
                    let top = span
                        .next()
                        .map(|span| heightfield.span(span).min())
                        .unwrap_or(Self::MAX_HEIGHT);
                    compact_heightfield.spans[cell_index].y = bot.clamp(0, Self::MAX_HEIGHT);
                    let height = (top.saturating_sub(bot)).min(u8::MAX.into()) as u8;
                    compact_heightfield.spans[cell_index].set_height(height);
                    compact_heightfield.areas[cell_index] = span.area();
                    cell_index += 1;
                    cell.inc_count();
                }
            }
        }

        // Find neighbour connections
        const MAX_LAYERS: u8 = CompactSpan::NOT_CONNECTED - 1;
        let mut max_layer_index = 0_u32;
        for z in 0..heightfield.height {
            for x in 0..heightfield.width {
                let column_index = x as usize + z as usize * heightfield.width as usize;
                let cell = &mut compact_heightfield.cells[column_index];
                let index_count = cell.index() as usize + cell.count() as usize;
                for i in cell.index() as usize..index_count as usize {
                    for dir in 0..4_u8 {
                        compact_heightfield.spans[i].set_con(dir, None);
                        let neighbor_x = x as i32 + dir_offset_x(dir) as i32;
                        let neighbor_z = z as i32 + dir_offset_z(dir) as i32;
                        // First check that the neighbour cell is in bounds.
                        if !heightfield.contains(neighbor_x, neighbor_z) {
                            continue;
                        }
                        let neighbor_x = neighbor_x as u16;
                        let neighbor_z = neighbor_z as u16;

                        // Iterate over all neighbour spans and check if any of the is
                        // accessible from current cell.
                        let column_index = heightfield.column_index(neighbor_x, neighbor_z);
                        let neighbor_cell = &compact_heightfield.cells[column_index];
                        let neighbor_index_count =
                            neighbor_cell.index() as usize + neighbor_cell.count() as usize;
                        let span_clone = compact_heightfield.spans[i].clone();
                        for k in neighbor_cell.index() as usize..neighbor_index_count as usize {
                            let neighbor_span = &compact_heightfield.spans[k];
                            let bot = span_clone.y.max(neighbor_span.y);
                            let top = (span_clone.y + span_clone.height() as u16)
                                .min(neighbor_span.y + neighbor_span.height() as u16);

                            // Check that the gap between the spans is walkable,
                            // and that the climb height between the gaps is not too high.
                            let is_walkable = (top as i32 - bot as i32) >= walkable_height as i32;
                            let is_climbable = (neighbor_span.y as i32 - span_clone.y as i32).abs()
                                <= walkable_climb as i32;
                            if !is_walkable || !is_climbable {
                                continue;
                            }
                            // Mark direction as walkable.
                            let layer_index = k as i32 - neighbor_cell.index() as i32;
                            if layer_index < 0 || layer_index >= MAX_LAYERS as i32 {
                                max_layer_index = max_layer_index.max(layer_index as u32);
                                continue;
                            }
                            let layer_index = layer_index as u8;
                            compact_heightfield.spans[i].set_con(dir, Some(layer_index));
                            break;
                        }
                    }
                }
            }
        }
        if max_layer_index > MAX_LAYERS as u32 {
            return Err(CompactHeightfieldError::TooManyLayers {
                max_layer_index: MAX_LAYERS,
                layer_index: max_layer_index,
            });
        }
        Ok(compact_heightfield)
    }

    #[inline]
    pub(crate) fn column_index(&self, x: u16, z: u16) -> usize {
        x as usize + z as usize * self.width as usize
    }

    /// Returns the cell at the given coordinates. Returns `None` if the coordinates are invalid.
    #[inline]
    pub fn get_cell_at(&self, x: u16, z: u16) -> Option<&CompactCell> {
        let Some(cell) = self.cells.get(self.column_index(x, z)) else {
            // Invalid coordinates
            return None;
        };
        Some(cell)
    }

    /// Returns the cell at the given coordinates. Panics if the coordinates are invalid.
    #[inline]
    pub fn cell_at(&self, x: u16, z: u16) -> &CompactCell {
        &self.cells[self.column_index(x, z)]
    }

    /// Returns the cell mutably at the given coordinates. Returns `None` if the coordinates are invalid.
    #[inline]
    pub fn get_cell_at_mut(&mut self, x: u16, z: u16) -> Option<&mut CompactCell> {
        let index = self.column_index(x, z);
        let Some(cell) = self.cells.get_mut(index) else {
            // Invalid coordinates
            return None;
        };
        Some(cell)
    }

    /// Returns the cell mutably at the given coordinates. Panics if the coordinates are invalid.
    #[inline]
    pub fn cell_at_mut(&mut self, x: u16, z: u16) -> &mut CompactCell {
        let index = self.column_index(x, z);
        &mut self.cells[index]
    }
}

/// Errors that can occur when building a compact heightfield.
#[derive(Debug, thiserror::Error)]
pub enum CompactHeightfieldError {
    /// The heightfield has too many layers.
    #[error(
        "Heightfield has too many layers. Max layer index is {max_layer_index}, but got {layer_index}"
    )]
    TooManyLayers {
        /// The maximum layer index.
        max_layer_index: u8,
        /// The layer index that caused the error.
        layer_index: u32,
    },
}
