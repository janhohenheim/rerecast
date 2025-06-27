use crate::{
    CompactHeightfield, Region,
    math::{dir_offset_x, dir_offset_z},
};

impl CompactHeightfield {
    /// Non-null regions will consist of connected, non-overlapping walkable spans that form a single contour.
    /// Contours will form simple polygons.
    ///
    /// If multiple regions form an area that is smaller than `min_region_area`, then all spans will be
    /// re-assigned to [`AreaType::NotWalkable`].
    ///
    /// Watershed partitioning can result in smaller than necessary regions, especially in diagonal corridors.
    /// `merge_region_area` helps reduce unnecessarily small regions.
    ///
    /// See the #rcConfig documentation for more information on the configuration parameters.
    ///
    /// The region data will be available via the [`CompactHeightfield::max_region`]
    /// and [`CompactSpan::regions`] fields.
    ///
    /// @warning The distance field must be created using [`CompactHeightfield::build_distance_field`] before attempting to build regions.
    ///
    /// @see rcCompactHeightfield, rcCompactSpan, rcBuildDistanceField, rcBuildRegionsMonotone, rcConfig
    pub fn build_regions(&mut self, border_size: u16, min_region_area: u16, max_region_area: u16) {
        const LOG_NB_STACKS: usize = 3;
        const NB_STACKS: usize = 1 << LOG_NB_STACKS;
        let mut level_stacks: [Vec<LevelStackEntry>; NB_STACKS] = [const { Vec::new() }; NB_STACKS];
        for stack in &mut level_stacks {
            stack.reserve(256);
        }

        let mut stack: Vec<LevelStackEntry> = Vec::with_capacity(256);

        let mut src_reg = vec![Region::NONE; self.spans.len()];
        let mut src_dist = vec![0_u16; self.spans.len()];

        let mut region_id = Region::from(1);
        let mut level = (self.max_distance + 1) & !1;

        // Jan: The following comment is taken from the original implementation.
        // TODO: Figure better formula, expandIters defines how much the
        // watershed "overflows" and simplifies the regions. Tying it to
        // agent radius was usually good indication how greedy it could be.
        //	const int expandIters = 4 + walkableRadius * 2;
        let expand_iters = 8;

        if border_size > 0 {
            // Make sure border will not overflow.
            let border_width = border_size.min(self.width);
            let border_height = border_size.min(self.height);

            // Paint regions
            self.paint_rect_region(
                0,
                border_width,
                0,
                self.height,
                region_id | Region::BORDER,
                &mut src_reg,
            );
            region_id += 1;
            self.paint_rect_region(
                self.width - border_width,
                self.width,
                0,
                self.height,
                region_id | Region::BORDER,
                &mut src_reg,
            );
            region_id += 1;
            self.paint_rect_region(
                0,
                self.width,
                0,
                border_height,
                region_id | Region::BORDER,
                &mut src_reg,
            );
            region_id += 1;
            self.paint_rect_region(
                0,
                border_width,
                self.height - border_height,
                self.height,
                region_id | Region::BORDER,
                &mut src_reg,
            );
            region_id += 1;
        }
        self.border_size = border_size;

        let mut s_id = -1_i32;
        while level > 0 {
            level = level.saturating_sub(2);
            s_id = (s_id + 1) & (NB_STACKS as i32 - 1);

            if s_id == 0 {
                self.sort_cells_by_level(level, &mut src_reg, NB_STACKS, &mut level_stacks, 1);
            } else {
                // copy left overs from last level
                let (src, dst) = level_stacks.split_at_mut(s_id as usize);
                append_stacks(&src[s_id as usize - 1], &mut dst[0], &src_reg);
            }

            self.expand_regions(
                expand_iters,
                level,
                &mut src_reg,
                &mut src_dist,
                &mut level_stacks[s_id as usize],
                false,
            );
        }
    }

    fn paint_rect_region(
        &mut self,
        min_x: u16,
        max_x: u16,
        min_z: u16,
        max_z: u16,
        region: Region,
        src_reg: &mut [Region],
    ) {
        for z in min_z..max_z {
            for x in min_x..max_x {
                let cell = self.cell_at(x, z);
                let max_index = cell.index() as usize + cell.count() as usize;
                #[expect(clippy::needless_range_loop)]
                for i in cell.index() as usize..max_index {
                    if self.areas[i].is_walkable() {
                        src_reg[i] = region;
                    }
                }
            }
        }
    }

    fn sort_cells_by_level(
        &mut self,
        start_level: u16,
        src_reg: &mut [Region],
        nb_stacks: usize,
        stacks: &mut [Vec<LevelStackEntry>],
        log_levels_per_stack: u16,
    ) {
        let start_level = start_level >> log_levels_per_stack;
        for stack in stacks.iter_mut().take(nb_stacks) {
            stack.clear();
        }

        // put all cells in the level range into the appropriate stacks
        for z in 0..self.height {
            for x in 0..self.width {
                let cell = self.cell_at(x, z);
                let max_index = cell.index() as usize + cell.count() as usize;
                #[expect(clippy::needless_range_loop)]
                for i in cell.index() as usize..max_index {
                    if !self.areas[i].is_walkable() || src_reg[i] != Region::NONE {
                        continue;
                    }
                    let level = self.dist[i] >> log_levels_per_stack;
                    // Jan: The original can underflow here FYI
                    let s_id = start_level.saturating_sub(level);
                    if s_id >= nb_stacks as u16 {
                        continue;
                    }
                    stacks[s_id as usize].push(LevelStackEntry {
                        x,
                        z,
                        index: Some(i),
                    });
                }
            }
        }
    }

    fn expand_regions(
        &mut self,
        max_iter: u16,
        level: u16,
        src_reg: &mut [Region],
        src_dist: &mut [u16],
        stack: &mut Vec<LevelStackEntry>,
        fill_stack: bool,
    ) {
        if fill_stack {
            // Find cells revealed by the raised level.
            stack.clear();
            for z in 0..self.height {
                for x in 0..self.width {
                    let cell = self.cell_at(x, z);
                    let max_index = cell.index() as usize + cell.count() as usize;
                    #[expect(clippy::needless_range_loop)]
                    for i in cell.index() as usize..max_index {
                        if self.dist[i] >= level
                            && src_reg[i] == Region::NONE
                            && self.areas[i].is_walkable()
                        {
                            stack.push(LevelStackEntry {
                                x,
                                z,
                                index: Some(i),
                            });
                        }
                    }
                }
            }
        } else {
            // use cells in the input stack
            // mark all cells which already have a region
            for entry in stack.iter_mut() {
                let Some(i) = entry.index else {
                    continue;
                };
                if src_reg[i] != Region::NONE {
                    entry.index = None;
                }
            }
        }

        let mut dirty_entries = Vec::new();
        let mut iter = 0;
        // Jan: I don't think stack is ever made smaller? Is this just an `if` in disguise?
        while stack.len() > 0 {
            let mut failed = 0;
            dirty_entries.clear();

            for entry in stack.iter_mut() {
                let x = entry.x;
                let z = entry.z;
                let Some(i) = entry.index else {
                    failed += 1;
                    continue;
                };

                let mut r = src_reg[i];
                let mut d2 = u16::MAX;
                let area = self.areas[i];
                let span = &self.spans[i];
                for dir in 0..4 {
                    let Some(con) = span.con(dir) else {
                        continue;
                    };
                    let a_x = (x as i32 + dir_offset_x(dir) as i32) as u16;
                    let a_z = (z as i32 + dir_offset_z(dir) as i32) as u16;
                    let a_index = self.cell_at(a_x, a_z).index() as usize + con as usize;
                    if self.areas[a_index] != area {
                        continue;
                    }
                    let a_region = src_reg[a_index];
                    let a_dist = src_dist[a_index] + 2;
                    if a_region != Region::NONE && a_region.contains(Region::BORDER) && a_dist < d2
                    {
                        r = a_region;
                        d2 = a_dist;
                    }
                }
                if r != Region::NONE {
                    // Mark as used
                    entry.index = None;
                    dirty_entries.push(DirtyEntry {
                        index: i,
                        region: r,
                        distance2: d2,
                    });
                } else {
                    failed += 1;
                }
            }
            // Copy entries that differ between src and dst to keep them in sync.
            for dirty_entry in dirty_entries.iter() {
                let index = dirty_entry.index;
                src_reg[index] = dirty_entry.region;
                src_dist[index] = dirty_entry.distance2;
            }

            if failed == stack.len() {
                break;
            }

            if level > 0 {
                iter += 1;
                if iter >= max_iter {
                    break;
                }
            }
        }
    }
}

fn append_stacks(
    src_stack: &[LevelStackEntry],
    dst_stack: &mut Vec<LevelStackEntry>,
    src_region: &[Region],
) {
    for stack in src_stack.iter() {
        let Some(i) = stack.index else {
            continue;
        };
        if src_region[i as usize] != Region::NONE {
            continue;
        }
        dst_stack.push(stack.clone());
    }
}

#[derive(Clone, Debug)]
struct LevelStackEntry {
    x: u16,
    z: u16,
    index: Option<usize>,
}

#[derive(Clone, Debug)]
struct DirtyEntry {
    index: usize,
    region: Region,
    distance2: u16,
}
