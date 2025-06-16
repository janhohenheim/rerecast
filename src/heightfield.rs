use bevy::math::bounding::Aabb3d;
use thiserror::Error;

use crate::span::{Span, SpanKey, Spans};
/// Corresponds to <https://github.com/recastnavigation/recastnavigation/blob/bd98d84c274ee06842bf51a4088ca82ac71f8c2d/Recast/Include/Recast.h#L312>
pub(crate) struct Heightfield {
    /// The width of the heightfield along the x-axis in cell units
    width: u32,
    /// The height of the heightfield along the y-axis in cell units
    height: u32,
    /// The AABB of the heightfield
    aabb: Aabb3d,
    /// The size of each cell on the xz-plane
    cell_size: f32,
    /// The size of each cell along the y-axis
    cell_height: f32,
    /// The indices to the spans in the heightfield in width*height order
    /// Each index corresponds to a column in the heightfield by pointing to the lowest span in the column
    columns: Vec<Option<SpanKey>>,
    /// All spans in the heightfield
    spans: Spans,
}

impl Heightfield {
    /// https://github.com/recastnavigation/recastnavigation/blob/bd98d84c274ee06842bf51a4088ca82ac71f8c2d/Recast/Source/RecastRasterization.cpp#L105
    pub(crate) fn add_span(&mut self, insertion: SpanInsertion) -> Result<(), SpanInsertionError> {
        let column_index = insertion.x as u128 * insertion.y as u128 * self.width as u128;
        if column_index >= self.columns.len() as u128 {
            return Err(SpanInsertionError::ColumnIndexOutOfBounds {
                x: insertion.x,
                y: insertion.y,
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
                // Current span is completely after the new span, break.
                break;
            }
            if current_span.max() < new_span.min() {
                // Current span is completely before the new span.  Keep going.
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
                let area = new_span.area().max(current_span.area());
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

    #[inline]
    fn span(&self, key: SpanKey) -> Span {
        self.spans[key].clone()
    }

    #[inline]
    fn span_mut(&mut self, key: SpanKey) -> &mut Span {
        &mut self.spans[key]
    }
}

pub(crate) struct HeightfieldBuilder {
    width: u32,
    height: u32,
    aabb: Aabb3d,
    cell_size: f32,
    cell_height: f32,
}

impl HeightfieldBuilder {
    pub(crate) fn build(self) -> Heightfield {
        let column_count = self.width as u128 * self.height as u128;
        if column_count > usize::MAX as u128 {
            panic!(
                "Failed to build heightfield: column count is too large using {}x{}",
                self.width, self.height
            );
        }
        let column_count = column_count as usize;
        Heightfield {
            width: self.width,
            height: self.height,
            aabb: self.aabb,
            cell_size: self.cell_size,
            cell_height: self.cell_height,
            columns: Vec::with_capacity(column_count),
            spans: Spans::with_min_capacity(column_count),
        }
    }
}

#[derive(Error, Debug)]
pub enum SpanInsertionError {
    #[error("column index out of bounds: x={x}, y={y}")]
    ColumnIndexOutOfBounds { x: u32, y: u32 },
}

pub(crate) struct SpanInsertion {
    /// The x-coordinate of the span
    pub(crate) x: u32,
    /// The y-coordinate of the span
    pub(crate) y: u32,
    /// How close two spans' maximum extents need to be to merge area type IDs
    pub(crate) flag_merge_threshold: u32,
    /// The span to insert
    pub(crate) span: Span,
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec3A;

    use crate::span::SpanBuilder;

    use super::*;

    fn height_field() -> Heightfield {
        HeightfieldBuilder {
            width: 10,
            height: 10,
            aabb: Aabb3d::new(Vec3A::ZERO, [5.0, 5.0, 5.0]),
            cell_size: 1.0,
            cell_height: 1.0,
        }
        .build()
    }

    #[test]
    fn can_create_heightfield() {
        let _heightfield = height_field();
    }

    #[test]
    fn can_add_span() {
        let mut heightfield = height_field();
        heightfield
            .add_span(SpanInsertion {
                x: 0,
                y: 0,
                flag_merge_threshold: 0,
                span: SpanBuilder {
                    min: 1,
                    max: 3,
                    area: 0,
                    next: None,
                }
                .build(),
            })
            .unwrap();
    }
}
