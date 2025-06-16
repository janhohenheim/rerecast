use bevy::prelude::*;
use slotmap::SlotMap;

slotmap::new_key_type! {
    pub(crate) struct SpanKey;
}

#[derive(Deref, DerefMut)]
pub(crate) struct Spans(SlotMap<SpanKey, Span>);

struct SpanKeyReflect(slotmap::KeyData);

impl Spans {
    const DEFAULT_CAPACITY: usize = 1024;

    pub(crate) fn with_min_capacity(min_capacity: usize) -> Self {
        let capacity = min_capacity.max(Self::DEFAULT_CAPACITY);
        Self(SlotMap::with_capacity_and_key(capacity))
    }
}

pub(crate) struct SpanBuilder {
    pub(crate) min: u16,
    pub(crate) max: u16,
    pub(crate) area: u8,
    pub(crate) next: Option<SpanKey>,
}

impl SpanBuilder {
    pub(crate) fn build(self) -> Span {
        let mut span = Span {
            data: [0; 32],
            next: self.next,
        };
        span.set_min(self.min);
        span.set_max(self.max);
        span.set_area(self.area);
        span
    }
}

impl From<SpanBuilder> for Span {
    fn from(builder: SpanBuilder) -> Self {
        builder.build()
    }
}

/// Corresponds to <https://github.com/recastnavigation/recastnavigation/blob/bd98d84c274ee06842bf51a4088ca82ac71f8c2d/Recast/Include/Recast.h#L294>
/// Build with [`SpanBuilder`]
#[derive(Clone)]
pub(crate) struct Span {
    /// - 13 bits for min
    /// - 13 bits for max
    /// - 6 bits for area
    data: [u8; 32],
    /// The key of the next-higher span in the column
    next: Option<SpanKey>,
}

impl Span {
    const MIN_BITS: usize = 13;
    const MAX_BITS: usize = 13;
    const AREA_BITS: usize = 6;

    #[inline]
    pub(crate) fn min(&self) -> u16 {
        // Safety: we are only indexing known constant indices
        unsafe { Self::read_bits(&self.data, 0, Self::MIN_BITS) }
    }

    #[inline]
    pub(crate) fn set_min(&mut self, min: u16) {
        unsafe { Self::write_bits(&mut self.data, 0, Self::MIN_BITS, min as u16) };
    }

    #[inline]
    pub(crate) fn max(&self) -> u16 {
        // Safety: we are only indexing known constant indices
        unsafe { Self::read_bits(&self.data, Self::MIN_BITS, Self::MAX_BITS) }
    }

    #[inline]
    pub(crate) fn set_max(&mut self, max: u16) {
        unsafe { Self::write_bits(&mut self.data, Self::MIN_BITS, Self::MAX_BITS, max as u16) };
    }

    #[inline]
    pub(crate) fn area(&self) -> u8 {
        // Safety: we are only indexing known constant indices
        unsafe {
            Self::read_bits(&self.data, Self::MIN_BITS + Self::MAX_BITS, Self::AREA_BITS) as u8
        }
    }

    #[inline]
    pub(crate) fn set_area(&mut self, area: u8) {
        unsafe {
            Self::write_bits(
                &mut self.data,
                Self::MIN_BITS + Self::MAX_BITS,
                Self::AREA_BITS,
                area as u16,
            )
        };
    }

    /// Reads `bit_count` bits from `data` starting at `bit_offset` and returns them as a `u16`.
    ///
    /// # Safety
    /// - Caller must ensure `data` has at least `(bit_offset + bit_count + 7) / 8` bytes.
    /// - `bit_count` must be <= 16.
    ///
    /// The function reads up to 3 bytes starting from the byte containing `bit_offset`,
    /// assembles them into a `u32`, then shifts and masks to extract the desired bits.
    ///
    /// No bounds checks or validation are performed for performance, so misuse can cause undefined behavior.
    #[inline]
    unsafe fn read_bits(data: &[u8], bit_offset: usize, bit_count: usize) -> u16 {
        let byte_offset = bit_offset / 8;
        let bit_in_byte = bit_offset % 8;

        // Read 3 bytes starting at byte_offset into a u32 for bit manipulation
        let val = (data[byte_offset] as u32)
            | ((data[byte_offset + 1] as u32) << 8)
            | ((data[byte_offset + 2] as u32) << 16);

        // Shift right to discard unwanted lower bits, then mask to keep only bit_count bits
        ((val >> bit_in_byte) & ((1 << bit_count) - 1)) as u16
    }

    /// Writes `bit_count` bits from `val` into `data` starting at `bit_offset`.
    ///
    /// # Safety
    /// - Caller must ensure `data` has at least `(bit_offset + bit_count + 7) / 8` bytes.
    /// - `bit_count` must be <= 16.
    /// - `val` must fit into `bit_count` bits (higher bits truncated).
    #[inline]
    unsafe fn write_bits(data: &mut [u8], bit_offset: usize, bit_count: usize, val: u16) {
        let byte_offset = bit_offset / 8;
        let bit_in_byte = bit_offset % 8;

        // Read 3 bytes into u32 to avoid partial overwrite issues
        let mut current = (data[byte_offset] as u32)
            | ((data[byte_offset + 1] as u32) << 8)
            | ((data[byte_offset + 2] as u32) << 16);

        // Create mask for the bits we're gonna overwrite
        let mask = ((1u32 << bit_count) - 1) << bit_in_byte;

        // Clear the target bits
        current &= !mask;

        // Set the bits from val (mask val to be safe)
        current |= (val as u32 & ((1 << bit_count) - 1)) << bit_in_byte;

        // Write back the bytes
        data[byte_offset] = current as u8;
        data[byte_offset + 1] = (current >> 8) as u8;
        data[byte_offset + 2] = (current >> 16) as u8;
    }

    #[inline]
    pub(crate) fn next(&self) -> Option<SpanKey> {
        self.next
    }

    #[inline]
    pub(crate) fn set_next(&mut self, next: impl Into<Option<SpanKey>>) {
        self.next = next.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        SpanBuilder {
            min: 2,
            max: 10,
            area: 4,
            next: None,
        }
        .build()
    }

    #[test]
    fn can_retrieve_span_data_after_building() {
        let span = span();
        assert_eq!(span.min(), 2);
        assert_eq!(span.max(), 10);
        assert_eq!(span.area(), 4);
        assert_eq!(span.next(), None);
    }

    #[test]
    fn can_retrieve_span_data_after_setting() {
        let mut span = span();
        let mut slotmap = SlotMap::with_key();
        let span_key: SpanKey = slotmap.insert(span.clone());

        span.set_min(1);
        span.set_max(4);
        span.set_area(3);
        span.set_next(span_key);

        assert_eq!(span.min(), 1);
        assert_eq!(span.max(), 4);
        assert_eq!(span.area(), 3);
        assert_eq!(span.next(), Some(span_key));
    }
}
