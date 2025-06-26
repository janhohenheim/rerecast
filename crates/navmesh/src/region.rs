use bevy::prelude::*;

/// A region in a [`CompactHeightfield`](crate::compact_heightfield::CompactHeightfield).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut)]
#[repr(transparent)]
pub struct Region(pub u16);

impl Default for Region {
    fn default() -> Self {
        Self::NONE
    }
}

impl From<u16> for Region {
    fn from(value: u16) -> Self {
        Region(value)
    }
}

impl Region {
    /// The default region, which is used for spans that are not in a region, i.e. not walkable.
    // TODO: is that correct?
    pub const NONE: Self = Self(0);
}
