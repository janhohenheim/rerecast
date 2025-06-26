//! The span module contains the types and functions for working with spans.
//!
//! A span is a voxel column with a floor and a ceiling.
//! It is used to represent the height of a non-occupied voxel column.
//!
//! The spans are stored in a [`Spans`](crate::span::Spans) collection.

use bevy::prelude::*;
use slotmap::SlotMap;

slotmap::new_key_type! {
    /// A key for a span in [`Spans`](crate::span::Spans).
    pub struct SpanKey;
}

/// A collection of spans.
#[derive(Deref, DerefMut, Debug, Clone)]
pub struct Spans(SlotMap<SpanKey, Span>);

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
    pub(crate) area: AreaType,
    pub(crate) next: Option<SpanKey>,
}

impl SpanBuilder {
    pub(crate) fn build(self) -> Span {
        Span {
            min: self.min,
            max: self.max,
            area: self.area,
            next: self.next,
        }
    }
}

impl From<SpanBuilder> for Span {
    fn from(builder: SpanBuilder) -> Self {
        builder.build()
    }
}

/// Corresponds to <https://github.com/recastnavigation/recastnavigation/blob/bd98d84c274ee06842bf51a4088ca82ac71f8c2d/Recast/Include/Recast.h#L294>
/// Build with [`SpanBuilder`]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Span {
    /// Height of the floor.
    ///
    /// Original uses 13 bits, but that results in the same alignment AFAIK, so we don't bother
    min: u16,
    /// Height of the ceiling.
    ///
    /// Original uses 13 bits, but that results in the same alignment AFAIK, so we don't bother
    max: u16,
    /// Area type ID.
    ///
    /// Original uses 6 bits, but that results in the same alignment AFAIK, so we don't bother
    area: AreaType,
    /// The key of the next-higher span in the column
    next: Option<SpanKey>,
}

impl Span {
    pub(crate) const MAX_HEIGHT: u16 = u16::MAX;

    #[inline]
    pub fn min(&self) -> u16 {
        self.min
    }

    #[inline]
    pub fn set_min(&mut self, min: u16) {
        self.min = min;
    }

    #[inline]
    pub fn max(&self) -> u16 {
        self.max
    }

    #[inline]
    pub fn set_max(&mut self, max: u16) {
        self.max = max;
    }

    #[inline]
    pub fn area(&self) -> AreaType {
        self.area
    }

    #[inline]
    pub fn set_area(&mut self, area: impl Into<AreaType>) {
        self.area = area.into();
    }

    #[inline]
    pub fn next(&self) -> Option<SpanKey> {
        self.next
    }

    #[inline]
    pub fn set_next(&mut self, next: impl Into<Option<SpanKey>>) {
        self.next = next.into();
    }
}

/// An identifier for the area type of a span.
/// The values 0 ([`AreaType::NOT_WALKABLE`]) and [`u8::MAX`] ([`AreaType::WALKABLE`]) are reserved.
/// The rest can be used for custom area types to e.g. assign different costs to different areas.
/// When two spans are merged, the area type of the merged span is the maximum of the two area types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deref, DerefMut)]
#[repr(transparent)]
pub struct AreaType(pub u8);

impl Default for AreaType {
    fn default() -> Self {
        Self::NOT_WALKABLE
    }
}

impl From<u8> for AreaType {
    fn from(value: u8) -> Self {
        AreaType(value)
    }
}

impl AreaType {
    /// The area type 0. Triangles with this area type are not walkable.
    /// All other area types are walkable.
    pub(crate) const NOT_WALKABLE: Self = Self(0);
    /// Default area type for walkable triangles. The highest possible area type.
    /// Other area types that are not [`AreaType::NOT_WALKABLE`] are also walkable.
    pub(crate) const DEFAULT_WALKABLE: Self = Self(u8::MAX);

    #[inline]
    pub(crate) fn is_walkable(&self) -> bool {
        self != &Self::NOT_WALKABLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        SpanBuilder {
            min: 2,
            max: 10,
            area: AreaType(4),
            next: None,
        }
        .build()
    }

    #[test]
    fn can_retrieve_span_data_after_building() {
        let span = span();
        assert_eq!(span.min(), 2);
        assert_eq!(span.max(), 10);
        assert_eq!(span.area(), AreaType(4));
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
        assert_eq!(span.area(), AreaType(3));
        assert_eq!(span.next(), Some(span_key));
    }
}
