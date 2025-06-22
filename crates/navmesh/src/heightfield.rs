//! The heightfield module contains the types and functions for working with [`Heightfield`]s.
//!
//! A heightfield is a 3D grid of [`Span`]s, where each column contains 0, 1, or more spans.

use bevy::math::bounding::Aabb3d;
use thiserror::Error;

use crate::span::{Span, SpanKey, Spans};
/// Corresponds to <https://github.com/recastnavigation/recastnavigation/blob/bd98d84c274ee06842bf51a4088ca82ac71f8c2d/Recast/Include/Recast.h#L312>
/// Build with [`HeightfieldBuilder`].
pub struct Heightfield {
    /// The width of the heightfield along the x-axis in cell units
    pub width: u32,
    /// The height of the heightfield along the y-axis in cell units
    pub height: u32,
    /// The AABB of the heightfield
    pub aabb: Aabb3d,
    /// The size of each cell on the xz-plane
    pub cell_size: f32,
    /// The size of each cell along the y-axis
    pub cell_height: f32,
    /// The indices to the spans in the heightfield in width*height order
    /// Each index corresponds to a column in the heightfield by pointing to the lowest span in the column
    pub columns: Vec<Option<SpanKey>>,
    /// All spans in the heightfield
    pub spans: Spans,
}

impl Heightfield {
    /// https://github.com/recastnavigation/recastnavigation/blob/bd98d84c274ee06842bf51a4088ca82ac71f8c2d/Recast/Source/RecastRasterization.cpp#L105
    pub(crate) fn add_span(&mut self, insertion: SpanInsertion) -> Result<(), SpanInsertionError> {
        let column_index = insertion.x as u128 + insertion.z as u128 * self.width as u128;
        if column_index >= self.columns.len() as u128 {
            return Err(SpanInsertionError::ColumnIndexOutOfBounds {
                x: insertion.x,
                y: insertion.z,
            });
        }
        let column_index = column_index as usize;

        let mut new_span = insertion.span;
        let mut previous_span_key = None;
        let mut current_span_key_iter = self.columns[column_index];
        // Insert the new span, possibly merging it with existing spans.
        while let Some(current_span_key) = current_span_key_iter {
            let current_span = self.span_mut(current_span_key);
            if current_span.min() > new_span.max() {
                // Current span is completely below the new span, break.
                break;
            }
            if current_span.max() < new_span.min() {
                // Current span is completely above the new span.  Keep going.
                previous_span_key.replace(current_span_key);
                current_span_key_iter = current_span.next();
                continue;
            }
            // The new span overlaps with an existing span.  Merge them.
            if current_span.min() < new_span.min() {
                new_span.set_min(current_span.min());
            }
            if current_span.max() > new_span.max() {
                new_span.set_max(current_span.max());
            }

            // Merge flags.
            if (new_span.max() as i32 - current_span.max() as i32).unsigned_abs()
                <= insertion.flag_merge_threshold
            {
                // Higher area ID numbers indicate higher resolution priority.
                let area = new_span.area().max(current_span.area().0);
                new_span.set_area(area);
            }

            // Remove the current span since it's now merged with newSpan.
            // Keep going because there might be other overlapping spans that also need to be merged.
            let next_key = current_span.next();
            self.spans.remove(current_span_key);
            if let Some(previous_span_key) = previous_span_key {
                self.span_mut(previous_span_key).set_next(next_key);
            } else {
                self.columns[column_index] = next_key;
            }
            current_span_key_iter = next_key;
        }

        if let Some(previous_span_key) = previous_span_key {
            // Insert new span after prev
            new_span.set_next(self.span(previous_span_key).next());
            let new_span_key = self.spans.insert(new_span);
            self.span_mut(previous_span_key).set_next(new_span_key);
        } else {
            // This span should go before the others in the list
            let lowest_span_key = self.columns[column_index];
            new_span.set_next(lowest_span_key);
            let new_span_key = self.spans.insert(new_span);
            self.columns[column_index] = Some(new_span_key);
        }

        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn span_at(&self, x: u32, y: u32) -> Option<Span> {
        let column_index = x as u128 + y as u128 * self.width as u128;
        let Some(span_key) = self.columns.get(column_index as usize) else {
            // Invalid coordinates
            return None;
        };
        let Some(span_key) = span_key else {
            // No span in this column
            return None;
        };
        Some(self.span(*span_key))
    }

    #[inline]
    fn span(&self, key: SpanKey) -> Span {
        self.spans[key].clone()
    }

    #[inline]
    fn span_mut(&mut self, key: SpanKey) -> &mut Span {
        &mut self.spans[key]
    }
}

/// A builder for [`Heightfield`]s.
pub struct HeightfieldBuilder {
    /// The AABB of the heightfield
    pub aabb: Aabb3d,
    /// The size of each cell on the xz-plane
    pub cell_size: f32,
    /// The size of each cell along the y-axis
    pub cell_height: f32,
}

impl HeightfieldBuilder {
    /// Builds the heightfield.
    ///
    /// # Panics
    ///
    /// Panics if the column count is above `usize::MAX`.
    pub fn build(self) -> Result<Heightfield, HeightfieldBuilderError> {
        let width = (self.aabb.max.x - self.aabb.min.x) / self.cell_size + 0.5;
        let depth = (self.aabb.max.z - self.aabb.min.z) / self.cell_size + 0.5;
        if width != depth {
            return Err(HeightfieldBuilderError::WidthAndDepthMismatch { width, depth });
        }
        let height = (self.aabb.max.y - self.aabb.min.y) / self.cell_height + 0.5;
        let column_count = width as u128 * height as u128;
        if column_count > usize::MAX as u128 {
            return Err(HeightfieldBuilderError::ColumnCountTooLarge { width, height });
        }
        let column_count = column_count as usize;
        Ok(Heightfield {
            width: width as u32,
            height: height as u32,
            aabb: self.aabb,
            cell_size: self.cell_size,
            cell_height: self.cell_height,
            columns: vec![None; column_count],
            spans: Spans::with_min_capacity(column_count),
        })
    }
}

/// Errors that can occur when building a [`Heightfield`] with [`HeightfieldBuilder::build`].
#[derive(Error, Debug)]
pub enum HeightfieldBuilderError {
    /// Happens when the width and depth of the heightfield are not the same.
    #[error("Width and depth must be the same, but got {width} and {depth}")]
    WidthAndDepthMismatch {
        /// The width of the heightfield along the x-axis in cell units
        width: f32,
        /// The depth of the heightfield along the z-axis in cell units
        depth: f32,
    },
    /// Happens when the column count is too large.
    #[error("Column count (width*height) is too large, got {width}*{height}={column_count} but max is {max}", column_count = width * height, max = usize::MAX)]
    ColumnCountTooLarge {
        /// The width of the heightfield along the x-axis in cell units
        width: f32,
        /// The height of the heightfield along the y-axis in cell units
        height: f32,
    },
}

/// Errors that can occur when inserting a span into a [`Heightfield`]
#[derive(Error, Debug)]
pub enum SpanInsertionError {
    /// Happens when the column index is out of bounds.
    #[error("column index out of bounds: x={x}, y={y}")]
    ColumnIndexOutOfBounds {
        /// The x-coordinate of the span
        x: u32,
        /// The z-coordinate of the span
        y: u32,
    },
}

pub(crate) struct SpanInsertion {
    /// The x-coordinate of the span
    pub(crate) x: u32,
    /// The z-coordinate of the span
    pub(crate) z: u32,
    /// Maximum difference between the ceilings of two spans to merge area type IDs
    pub(crate) flag_merge_threshold: u32,
    /// The span to insert
    pub(crate) span: Span,
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec3A;

    use crate::span::{AreaType, SpanBuilder};

    use super::*;

    fn height_field() -> Heightfield {
        HeightfieldBuilder {
            aabb: Aabb3d::new(Vec3A::ZERO, [5.0, 5.0, 5.0]),
            cell_size: 1.0,
            cell_height: 1.0,
        }
        .build()
        .unwrap()
    }

    fn span_low() -> SpanBuilder {
        SpanBuilder {
            min: 2,
            max: 4,
            area: AreaType(2),
            next: None,
        }
    }

    fn span_mid() -> SpanBuilder {
        SpanBuilder {
            min: 4,
            max: 7,
            area: AreaType(2),
            next: None,
        }
    }

    fn span_high() -> SpanBuilder {
        SpanBuilder {
            min: 7,
            max: 10,
            area: AreaType(2),
            next: None,
        }
    }

    #[test]
    fn can_create_heightfield() {
        let _heightfield = height_field();
    }

    #[test]
    fn can_add_span() {
        let mut heightfield = height_field();
        let expected_span = span_low().build();
        heightfield
            .add_span(SpanInsertion {
                x: 1,
                z: 3,
                flag_merge_threshold: 0,
                span: expected_span.clone(),
            })
            .unwrap();
        let span = heightfield.span_at(1, 3).unwrap();
        assert_eq!(span, expected_span);

        let empty_span = heightfield.span_at(3, 1);
        assert_eq!(empty_span, None);
    }

    #[test]
    fn can_add_multiple_spans_next_to_each_other() {
        let mut heightfield = height_field();
        let expected_span_1 = span_low().build();
        heightfield
            .add_span(SpanInsertion {
                x: 1,
                z: 3,
                flag_merge_threshold: 0,
                span: expected_span_1.clone(),
            })
            .unwrap();

        let expected_span_2 = span_mid().build();
        heightfield
            .add_span(SpanInsertion {
                x: 2,
                z: 3,
                flag_merge_threshold: 0,
                span: expected_span_2.clone(),
            })
            .unwrap();

        let span = heightfield.span_at(1, 3).unwrap();
        assert_eq!(span, expected_span_1);
        let span = heightfield.span_at(2, 3).unwrap();
        assert_eq!(span, expected_span_2);

        let empty_span = heightfield.span_at(3, 1);
        assert_eq!(empty_span, None);
    }

    #[test]
    fn can_add_higher_span_in_same_column() {
        let mut heightfield = height_field();
        let span_low = span_low().build();
        heightfield
            .add_span(SpanInsertion {
                x: 1,
                z: 3,
                flag_merge_threshold: 0,
                span: span_low.clone(),
            })
            .unwrap();

        let span_high = span_high().build();
        heightfield
            .add_span(SpanInsertion {
                x: 1,
                z: 3,
                flag_merge_threshold: 0,
                span: span_high.clone(),
            })
            .unwrap();

        let span = heightfield.span_at(1, 3).unwrap();
        assert_eq_without_next(&span, &span_low);
        let next_span = span.next().unwrap();
        let next_span = heightfield.span(next_span);
        assert_eq_without_next(&next_span, &span_high);

        let empty_span = heightfield.span_at(3, 1);
        assert_eq!(empty_span, None);
    }

    #[test]
    fn can_add_lower_span_in_same_column() {
        let mut heightfield = height_field();
        let span_high = span_high().build();
        heightfield
            .add_span(SpanInsertion {
                x: 1,
                z: 3,
                flag_merge_threshold: 0,
                span: span_high.clone(),
            })
            .unwrap();

        let span_low = span_low().build();
        heightfield
            .add_span(SpanInsertion {
                x: 1,
                z: 3,
                flag_merge_threshold: 0,
                span: span_low.clone(),
            })
            .unwrap();

        let span = heightfield.span_at(1, 3).unwrap();
        assert_eq_without_next(&span, &span_low);
        let next_span = span.next().unwrap();
        let next_span = heightfield.span(next_span);
        assert_eq_without_next(&next_span, &span_high);

        let empty_span = heightfield.span_at(3, 1);
        assert_eq!(empty_span, None);
    }

    #[test]
    fn can_merge_spans() {
        let mut heightfield = height_field();
        let span_low = span_low().build();
        heightfield
            .add_span(SpanInsertion {
                x: 1,
                z: 3,
                flag_merge_threshold: 0,
                span: span_low.clone(),
            })
            .unwrap();

        let span_mid: Span = span_mid().build();
        heightfield
            .add_span(SpanInsertion {
                x: 1,
                z: 3,
                flag_merge_threshold: 0,
                span: span_mid.clone(),
            })
            .unwrap();

        let merged_span = SpanBuilder {
            min: span_low.min(),
            max: span_mid.max(),
            area: span_mid.area(),
            next: None,
        }
        .build();

        let span = heightfield.span_at(1, 3).unwrap();
        assert_eq!(span, merged_span);

        let empty_span = heightfield.span_at(3, 1);
        assert_eq!(empty_span, None);
    }

    #[track_caller]
    fn assert_eq_without_next(span: &Span, expected_span: &Span) {
        assert_eq!(span.min(), expected_span.min(), "min is not equal");
        assert_eq!(span.max(), expected_span.max(), "max is not equal");
        assert_eq!(span.area(), expected_span.area(), "area is not equal");
    }
}
