use crate::region::Region;

/// A span in a [`CompactHeightfield`](crate::compact_heightfield::CompactHeightfield).
/// Packed for memory efficiency.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompactSpan {
    /// The lower extent of the span. (Measured from the heightfield's base.)
    pub y: u16,
    /// The id of the region the span belongs to. (Or [`Region::None`] if not in a region.)
    pub region: Region,
    /// 24 bits: packed neighbor connection data
    /// 8 bits: the height of the span
    pub data: u32,
}

impl CompactSpan {
    pub(crate) const NOT_CONNECTED: u8 = 0x3f;

    /// Sets the neighbor connection data for the given direction.
    /// `None` if the neighbor is not connected.
    pub fn set_con(&mut self, direction: u8, neighbor: impl Into<Option<u8>>) {
        let shift = (direction as u32) * 6;
        let con = self.data;
        let value =
            neighbor.into().unwrap_or(Self::NOT_CONNECTED) as u32 & Self::NOT_CONNECTED as u32;
        self.data = (con & !(0x3f << shift)) | (value << shift);
    }

    /// Returns the neighbor connection data for the given direction.
    /// `None` if the neighbor is not connected.
    pub fn con(&self, direction: u8) -> Option<u8> {
        let shift = (direction as u32) * 6;
        let value = ((self.data >> shift) & Self::NOT_CONNECTED as u32) as u8;
        if value == Self::NOT_CONNECTED {
            None
        } else {
            Some(value)
        }
    }

    /// Returns the height of the span.
    pub fn height(&self) -> u8 {
        (self.data >> 24) as u8
    }

    /// Sets the height of the span.
    pub fn set_height(&mut self, height: u8) {
        self.data = (self.data & 0x00FF_FFFF) | ((height as u32) << 24);
    }
}

slotmap::new_key_type! {
    /// A key for a span in [`CompactSpans`](crate::compact_span::CompactSpans).
    pub struct CompactSpanKey;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_span() {
        let mut span = CompactSpan::default();
        span.set_height(10);
        assert_eq!(span.height(), 10);
    }

    #[test]
    fn test_compact_span_con() {
        let mut span = CompactSpan::default();
        span.set_con(0, Some(1));
        assert_eq!(span.con(0), Some(1));

        span.set_con(1, Some(3));
        assert_eq!(span.con(1), Some(3));

        span.set_con(2, Some(5));
        assert_eq!(span.con(2), Some(5));

        span.set_con(0, Some(2));
        assert_eq!(span.con(0), Some(2));

        span.set_con(1, None);
        assert_eq!(span.con(1), None);

        span.set_con(2, None);
        assert_eq!(span.con(2), None);
    }
}
