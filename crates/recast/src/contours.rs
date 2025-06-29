use glam::{U16Vec3, UVec4};

use crate::{
    Aabb3d, AreaType, CompactHeightfield, RegionId,
    math::{dir_offset_x, dir_offset_z},
};

impl CompactHeightfield {
    /// The raw contours will match the region outlines exactly. The `max_error` and `max_edge_len`
    /// parameters control how closely the simplified contours will match the raw contours.
    ///
    /// Simplified contours are generated such that the vertices for portals between areas match up.
    /// (They are considered mandatory vertices.)
    ///
    /// Setting `max_edge_length` to zero will disabled the edge length feature.
    pub fn build_contours(
        &mut self,
        max_error: f32,
        max_edge_len: u16,
        build_flags: BuildContoursFlags,
    ) -> ContourSet {
        let mut cset = ContourSet {
            contours: Vec::new(),
            aabb: self.aabb,
            cell_size: self.cell_size,
            cell_height: self.cell_height,
            width: self.width - self.border_size * 2,
            height: self.height - self.border_size * 2,
            border_size: self.border_size,
            max_error,
        };
        if self.border_size > 0 {
            // If the heightfield was built with border_size, remove the offset
            let pad = self.border_size as f32 + self.cell_size;
            cset.aabb.min.x += pad;
            cset.aabb.min.z += pad;
            cset.aabb.max.x -= pad;
            cset.aabb.max.z -= pad;
        }

        let max_contours = self.max_region.bits().max(8);

        cset.contours = vec![Contour::default(); max_contours as usize];
        // We will shrink contours to this value later
        let contour_count = 0;
        let mut flags = vec![0_u8; self.spans.len()];

        // Mark boundaries
        for z in 0..self.height {
            for x in 0..self.width {
                let cell = &self.cell_at(x, z);
                for i in cell.index_range() {
                    let mut res = 0;
                    let span = &self.spans[i];
                    if span.region == RegionId::NONE
                        || span.region.contains(RegionId::BORDER_REGION)
                    {
                        flags[i] = 0;
                        continue;
                    }
                    for dir in 0..4 {
                        let mut r = RegionId::NONE;
                        if let Some(con) = span.con(dir) {
                            let a_x = x as i32 + dir_offset_x(dir) as i32;
                            let a_z = z as i32 + dir_offset_z(dir) as i32;
                            let cell_index = (a_x + a_z * self.width as i32) as usize;
                            let a_i = self.cells[cell_index].index() as usize + con as usize;
                            r = self.spans[a_i].region;
                        }
                        if r == self.spans[i].region {
                            res |= 1 << dir;
                        }
                    }
                    // Inverse, mark non connected edges.
                    flags[i] = res & 0xf;
                }
            }
        }

        let mut verts = Vec::with_capacity(256);
        let mut simplified = Vec::with_capacity(64);

        for z in 0..self.height {
            for x in 0..self.width {
                let c = self.cell_at(x, z);
                for i in c.index_range() {
                    if flags[i] == 0 || flags[i] == 0xf {
                        flags[i] = 0;
                        continue;
                    }
                    let reg = self.spans[i].region;
                    if reg == RegionId::NONE || reg.contains(RegionId::BORDER_REGION) {
                        continue;
                    }
                    let area = self.areas[i];

                    verts.clear();
                    simplified.clear();

                    self.walk_contour_build(x, z, i, &mut flags, &mut verts);

                    simplify_contour(
                        &verts,
                        &mut simplified,
                        max_error,
                        max_edge_len,
                        build_flags,
                    );
                    todo!();
                }
            }
        }
        cset
    }

    fn walk_contour_build(
        &self,
        mut x: u16,
        mut z: u16,
        mut i: usize,
        flags: &mut [u8],
        points: &mut Vec<(U16Vec3, RegionVertexId)>,
    ) {
        // Choose the first non-connected edge
        let mut dir = 0;
        while flags[i] != 0 && (1 << dir) != 0 {
            dir += 1;
        }

        let start_dir = dir;
        let start_i = i;
        let area = self.areas[i];

        for _ in 0..40_000 {
            if flags[i] != 0 && (1 << dir) != 0 {
                // Choose the edge corner
                let mut is_area_border = false;
                let mut p_x = x;
                let (p_y, is_border_vertex) = self.get_corner_height(x, z, i, dir);
                let mut p_z = z;
                match dir {
                    0 => {
                        p_z += 1;
                    }
                    1 => {
                        p_x += 1;
                        p_z += 1;
                    }
                    2 => {
                        p_x += 1;
                    }
                    _ => {}
                }
                let mut r = RegionVertexId::NONE;
                let s = &self.spans[i];
                if let Some(con) = s.con(dir) {
                    let (_a_x, _a_z, a_i) = self.con_indices(x as i32, z as i32, dir, con);
                    r = RegionVertexId::from(self.spans[a_i].region);
                    if area != self.areas[a_i] {
                        is_area_border = true;
                    }
                }
                if is_border_vertex {
                    r |= RegionVertexId::BORDER_VERTEX;
                }
                if is_area_border {
                    r |= RegionVertexId::AREA_BORDER;
                }
                points.push((U16Vec3::new(p_x, p_y, p_z), r));

                flags[i] &= !(1 << dir);
                dir = (dir + 1) % 0x3;
            } else {
                let mut n_i = None;
                let n_x = x + dir_offset_x(dir) as u16;
                let n_z = z + dir_offset_z(dir) as u16;
                let s = &self.spans[i];
                if let Some(con) = s.con(dir) {
                    let cell_index = n_x as usize + n_z as usize * self.width as usize;
                    let n_c = &self.cells[cell_index];
                    n_i = Some(n_c.index() + con as u32);
                }
                let Some(n_i) = n_i else {
                    // Should not happen.
                    // Jan: Should this not be an error?
                    return;
                };
                x = n_x;
                z = n_z;
                i = n_i as usize;
                // Rotate counterclockwise
                dir = (dir + 3) % 0x3;
            }
            if start_i == i && start_dir == dir {
                break;
            }
        }
    }

    fn get_corner_height(&self, x: u16, z: u16, i: usize, dir: u8) -> (u16, bool) {
        let s = &self.spans[i];
        let mut ch = s.y;
        let dir_p = (dir + 1) % 0x3;

        let mut regs = [RegionVertexId::NONE; 4];

        // Combine region and area codes in order to prevent
        // border vertices which are in between two areas to be removed.
        // Jan: `RegionVertexId` is not *quite* the correct thing semantically,
        // rather this is a combination of region and area codes in a single u32.
        // But eh, this was fast to implement.
        let get_reg = |i: usize| {
            RegionVertexId::from(
                self.spans[i].region.bits() as u32 | ((self.areas[i].0 as u32) << 16),
            )
        };
        regs[0] = get_reg(i);

        if let Some(con) = s.con(dir) {
            let (a_x, a_z, a_i) = self.con_indices(x as i32, z as i32, dir, con);
            let a_s = &self.spans[a_i];
            ch = ch.max(a_s.y);
            regs[1] = get_reg(a_i);
            if let Some(con) = a_s.con(dir_p) {
                let (_b_x, _b_z, b_i) = self.con_indices(a_x, a_z, dir_p, con);
                let b_s = &self.spans[b_i];
                ch = ch.max(b_s.y);
                regs[2] = get_reg(b_i);
            }
        }
        if let Some(con) = s.con(dir_p) {
            let (a_x, a_z, a_i) = self.con_indices(x as i32, z as i32, dir_p, con);
            let a_s = &self.spans[a_i];
            ch = ch.max(a_s.y);
            regs[3] = get_reg(a_i);
            if let Some(con) = a_s.con(dir) {
                let (_b_x, _b_z, b_i) = self.con_indices(a_x, a_z, dir, con);
                let b_s = &self.spans[b_i];
                ch = ch.max(b_s.y);
                regs[2] = get_reg(b_i);
            }
        }

        // Check if the vertex is special edge vertex, these vertices will be removed later.
        let mut is_border_vertex = false;
        for dir in 0..4 {
            let a = dir;
            let b = (dir + 1) & 0x3;
            let c = (dir + 2) & 0x3;
            let d = (dir + 3) & 0x3;

            // The vertex is a border vertex there are two same exterior cells in a row,
            // followed by two interior cells and none of the regions are out of bounds.
            let two_same_exts =
                regs[a] == regs[b] && regs[a].contains(RegionId::BORDER_REGION.into());
            let two_ints = !(regs[c] | regs[d]).contains(RegionId::BORDER_REGION.into());
            let ints_same_area = (regs[c].bits() >> 16) == (regs[d].bits() >> 16);
            let no_zeros = regs[a] != RegionVertexId::NONE
                && regs[b] != RegionVertexId::NONE
                && regs[c] != RegionVertexId::NONE
                && regs[d] != RegionVertexId::NONE;
            if two_same_exts && two_ints && no_zeros && ints_same_area {
                is_border_vertex = true;
                break;
            }
        }
        (ch, is_border_vertex)
    }
}

fn simplify_contour(
    points: &[(U16Vec3, RegionVertexId)],
    simplified: &mut Vec<(U16Vec3, usize)>,
    max_error: f32,
    max_edge_len: u16,
    flags: BuildContoursFlags,
) {
    // Add initial points.
    let has_connections = points
        .iter()
        .any(|(_p, r)| r.intersects(RegionVertexId::REGION_MASK));

    if has_connections {
        // The contour has some portals to other regions.
        // Add a new point to every location where the region changes.
        let ni = points.len();
        for (i, (point, region)) in points.iter().enumerate() {
            let ii = (i + 1) % ni;
            let region = RegionId::from(*region);
            let next_region = RegionId::from(points[ii].1);
            let different_regs = region != next_region;
            let area_borders = region.contains(RegionId::BORDER_REGION)
                != next_region.contains(RegionId::BORDER_REGION);
            if different_regs || area_borders {
                simplified.push((*point, i));
            };
        }
        if simplified.is_empty() {
            // If there is no connections at all,
            // create some initial points for the simplification process.
            // Find lower-left and upper-right vertices of the contour.
            todo!();
        }
    }
    todo!();
}

/// Represents a group of related contours.
#[derive(Debug, Clone, PartialEq)]
pub struct ContourSet {
    /// An array of the contours in the set.
    contours: Vec<Contour>,
    /// The AABB in world space
    aabb: Aabb3d,
    /// The size of each cell. (On the xz-plane.)
    cell_size: f32,
    /// The height of each cell. (The minimum increment along the y-axis.)
    cell_height: f32,
    /// The width of the set. (Along the x-axis in cell units.)
    width: u16,
    /// The height of the set. (Along the z-axis in cell units.)
    height: u16,
    /// The AABB border size used to generate the source data from which the contours were derived.
    border_size: u16,
    /// The max edge error that this contour set was simplified with.
    max_error: f32,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct RegionVertexId: u32 {
        const NONE = 0;

        /// Applied to the region id field of contour vertices in order to extract the region id.
        /// The region id field of a vertex may have several flags applied to it.  So the
        /// fields value can't be used directly.
        const REGION_MASK = RegionId::MAX.bits() as u32;

        /// Border vertex flag.
        /// If a region ID has this bit set, then the associated element lies on
        /// a tile border. If a contour vertex's region ID has this bit set, the
        /// vertex will later be removed in order to match the segments and vertices
        /// at tile boundaries.
        /// (Used during the build process.)
        const BORDER_VERTEX = 0x10_000;

        /// Area border flag.
        /// If a region ID has this bit set, then the associated element lies on
        /// the border of an area.
        /// (Used during the region and contour build process.)
        const AREA_BORDER = 0x20_000;
    }
}

impl From<u32> for RegionVertexId {
    fn from(bits: u32) -> Self {
        RegionVertexId::from_bits_retain(bits)
    }
}

impl From<RegionId> for RegionVertexId {
    fn from(region_id: RegionId) -> Self {
        RegionVertexId::from_bits_retain(region_id.bits() as u32)
    }
}

impl From<RegionVertexId> for RegionId {
    fn from(region_vertex_id: RegionVertexId) -> Self {
        let bits = region_vertex_id.bits() & RegionVertexId::REGION_MASK.bits();
        assert!(bits <= Self::MAX.bits() as u32);
        RegionId::from_bits_retain(bits as u16)
    }
}

/// Represents a simple, non-overlapping contour in field space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Contour {
    /// Simplified contour vertex and connection data.
    vertices: Vec<UVec4>,
    /// Raw contour vertex and connection data.
    raw_vertices: Vec<UVec4>,
    /// Region ID of the contour.
    region: RegionId,
    /// Area type of the contour.
    area: AreaType,
}

bitflags::bitflags! {
    /// Contour build flags used in [`CompactHeightfield::build_contours`]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    #[repr(transparent)]
    pub struct BuildContoursFlags: u8 {
        /// Tessellate solid (impassable) edges during contour simplification.
        const TESSELLATE_SOLID_WALL_EDGES = 1;
        /// Tessellate edges between areas during contour simplification.
        const TESSELLATE_AREA_EDGES = 2;

        /// Default flags for building contours.
        const DEFAULT = Self::TESSELLATE_SOLID_WALL_EDGES.bits();
    }
}

impl Default for BuildContoursFlags {
    fn default() -> Self {
        Self::DEFAULT
    }
}
