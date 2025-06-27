//! Watershed partitioning
//!   - the classic Recast partitioning
//!   - creates the nicest tessellation
//!   - usually slowest
//!   - partitions the heightfield into nice regions without holes or overlaps
//!   - the are some corner cases where this method creates produces holes and overlaps
//!      - holes may appear when a small obstacles is close to large open area (triangulation can handle this)
//!      - overlaps may occur if you have narrow spiral corridors (i.e stairs), this make triangulation to fail
//!   * generally the best choice if you precompute the navmesh, use this if you have large open areas

use crate::CompactHeightfield;

impl CompactHeightfield {
    /// Prepare for region partitioning, by calculating distance field along the walkable surface.
    pub fn build_distance_field(&mut self) {
        let mut src = vec![0_u16; self.spans.len()];
        let mut dst = vec![0_u16; self.spans.len()];

        self.max_distance = self.calculate_max_distance(&mut src);
        self.box_blur(1, &src, &mut dst);
        // Jan: looking at the code carefully, it seems like the dst is always the one being picked de facto
        self.dist = dst;
    }

    fn calculate_max_distance(&mut self, src: &mut [u16]) -> u16 {
        todo!()
    }

    fn box_blur(&mut self, threshold: i32, src: &[u16], dst: &mut [u16]) {
        todo!()
    }
}
