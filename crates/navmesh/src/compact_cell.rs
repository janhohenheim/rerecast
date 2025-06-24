/// Provides information on the content of a cell column in a [`CompactHeightfield`](crate::compact_heightfield::CompactHeightfield).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompactCell {
    // original: 24 bits
    /// Index to the first span in the column.
    index: u32,
    // original: 8 bits
    /// Number of spans in the column.
    count: u8,
}

impl CompactCell {
    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn count(&self) -> u8 {
        self.count
    }

    pub fn set_index(&mut self, index: u32) {
        self.index = index;
    }

    pub fn set_count(&mut self, count: u8) {
        self.count = count;
    }

    pub fn inc_count(&mut self) {
        self.count += 1;
    }
}
