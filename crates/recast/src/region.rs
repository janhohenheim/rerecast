use std::ops::{Deref, DerefMut};

/// A region in a [`CompactHeightfield`](crate::compact_heightfield::CompactHeightfield).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Region(pub u16);

impl Deref for Region {
    type Target = u16;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Region {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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
