use bevy::prelude::*;
use slotmap::SlotMap;

use crate::{
    region::Region,
    span::{AreaType, SpanKey},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompactSpan {
    /// The lower extent of the span. (Measured from the heightfield's base.)
    pub y: u16,
    /// The id of the region the span belongs to. (Or [`Region::None`] if not in a region.)
    pub region: Region,
    /// 24 bits: packed neighbor connection data
    /// 8 bits: the height of the span
    data: u32,
}

impl CompactSpan {
    pub fn con(&self) -> u32 {
        todo!()
    }

    pub fn set_con(&mut self, con: u32) {
        todo!()
    }

    pub fn height(&self) -> u8 {
        todo!()
    }

    pub fn set_height(&mut self, height: u8) {
        todo!()
    }
}

slotmap::new_key_type! {
    /// A key for a span in [`CompactSpans`](crate::compact_span::CompactSpans).
    pub struct CompactSpanKey;
}

#[derive(Deref, DerefMut)]
pub struct CompactSpans(SlotMap<CompactSpanKey, CompactSpan>);

impl CompactSpans {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(SlotMap::with_capacity_and_key(capacity))
    }
}
