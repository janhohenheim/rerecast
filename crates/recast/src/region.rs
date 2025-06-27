bitflags::bitflags! {
    /// A region in a [`CompactHeightfield`](crate::compact_heightfield::CompactHeightfield).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct Region: u16 {
        /// The default region, which is used for spans that are not in a region, i.e. not walkable.
        const NONE = 0;
        /// Heightfield border flag.
        /// If a heightfield region ID has this bit set, then the region is a border
        /// region and its spans are considered un-walkable.
        /// (Used during the region and contour build process.)
        const BORDER = 0x8000;
    }
}

impl Region {
    pub(crate) fn inc(&mut self) {
        *self = Region::from(self.bits() + 1);
    }
}

impl Default for Region {
    fn default() -> Self {
        Self::NONE
    }
}

impl From<u16> for Region {
    fn from(value: u16) -> Self {
        Region::from_bits_truncate(value)
    }
}
