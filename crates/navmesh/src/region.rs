use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut)]
#[repr(transparent)]
pub struct Region(pub u16);

impl Default for Region {
    fn default() -> Self {
        Self::None
    }
}

impl From<u16> for Region {
    fn from(value: u16) -> Self {
        Region(value)
    }
}
impl Region {
    pub const None: Self = Self(0);
}
