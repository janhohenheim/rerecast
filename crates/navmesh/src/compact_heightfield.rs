use std::usize;

use bevy::math::bounding::Aabb3d;

use crate::{
    compact_cell::CompactCell,
    compact_span::{CompactSpan, CompactSpanKey, CompactSpans},
    heightfield::Heightfield,
    region::Region,
    span::{AreaType, SpanKey, Spans},
};

pub struct CompactHeightfield {
    /// The width of the heightfield along the x-axis in cell units
    pub width: u32,
    /// The height of the heightfield along the z-axis in cell units
    pub height: u32,
    /// The walkable height used during the build of the field
    pub walkable_height: u16,
    /// The walkable climb used during the build of the field.
    pub walkable_climb: u16,
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

    pub fn from_heightfield(
        heightfield: Heightfield,
        walkable_height: u16,
        walkable_climb: u16,
    ) -> Self {
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
            aabb: heightfield.aabb,
            max_distance: 0,
            max_region: Region::None,
            cell_size: heightfield.cell_size,
            cell_height: heightfield.cell_height,
            cells: vec![
                CompactCell::default();
                heightfield.width as usize * heightfield.height as usize
            ],
            spans: vec![CompactSpan::default(); walkable_span_count],
            dist: vec![0; walkable_span_count],
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
                let mut span = heightfield.span(span_key);
                let column_index = x as usize + z as usize * heightfield.width as usize;

                let cell = &mut compact_heightfield.cells[column_index];
                cell.set_index(cell_index as u32);
                cell.set_count(0);

                while let Some(span_key) = span.next() {
                    span = heightfield.span(span_key);
                    if !span.area().is_walkable() {
                        continue;
                    }
                    let bot = span.max();
                    let top = span
                        .next()
                        .map(|span| heightfield.span(span).min())
                        .unwrap_or(Self::MAX_HEIGHT);
                    compact_heightfield.spans[cell_index].y = bot.clamp(0, Self::MAX_HEIGHT);
                    compact_heightfield.spans[cell_index].set_height(top.saturating_sub(bot) as u8);
                    compact_heightfield.areas[cell_index] = span.area();
                    compact_heightfield.dist[cell_index] = 0;
                    cell_index += 1;
                    cell.inc_count();
                }
            }
        }

        // Find neighbour connections
        // Original is an ugly RC_NOT_CONNECTED - 1 lol
        const MAX_LAYERS: u8 = u8::MAX;
        let mut max_layer_index = 0;
        let z_stride = heightfield.width;
        for z in 0..heightfield.height {
            for x in 0..heightfield.width {
                let column_index = x as usize + z as usize * heightfield.width as usize;
                let cell = &mut compact_heightfield.cells[column_index];
                let index_count = cell.index() as usize + cell.count() as usize;
                for i in cell.index() as usize..index_count as usize {
                    let span = compact_heightfield.spans[i];
                    for dir in 0..4 {
                        todo!()
                    }
                }
            }
        }
        compact_heightfield
    }

    #[inline]
    pub fn get_cell_at(&self, x: u32, z: u32) -> Option<&CompactCell> {
        let column_index = x as u128 + z as u128 * self.width as u128;
        let Some(cell) = self.cells.get(column_index as usize) else {
            // Invalid coordinates
            return None;
        };
        Some(cell)
    }

    #[inline]
    pub fn cell_at(&self, x: u32, z: u32) -> &CompactCell {
        let column_index = x as u128 + z as u128 * self.width as u128;
        &self.cells[column_index as usize]
    }

    #[inline]
    pub fn get_cell_at_mut(&mut self, x: u32, z: u32) -> Option<&mut CompactCell> {
        let column_index = x as u128 + z as u128 * self.width as u128;
        let Some(cell) = self.cells.get_mut(column_index as usize) else {
            // Invalid coordinates
            return None;
        };
        Some(cell)
    }

    #[inline]
    pub fn cell_at_mut(&mut self, x: u32, z: u32) -> &mut CompactCell {
        let column_index = x as u128 + z as u128 * self.width as u128;
        &mut self.cells[column_index as usize]
    }
}
