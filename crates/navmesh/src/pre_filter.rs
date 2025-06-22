use crate::{
    heightfield::Heightfield,
    span::{AreaType, Span},
};

impl Heightfield {
    pub(crate) fn filter_low_hanging_walkable_obstacles(&mut self, walkable_climb_height: u16) {
        for z in 0..self.height {
            for x in 0..self.width {
                let mut previous_span: Option<Span> = None;
                let mut previous_was_walkable = false;
                let mut previous_area_id = AreaType::NOT_WALKABLE;

                // For each span in the column...
                while let Some(span) = self.span_at_mut(x, z) {
                    let walkable = span.area().is_walkable();

                    // If current span is not walkable, but there is walkable span just below it and the height difference
                    // is small enough for the agent to walk over, mark the current span as walkable too.
                    if let Some(previous_span) = previous_span.as_ref() {
                        if !walkable
                            && previous_was_walkable
                            && (span.max() as i32 - previous_span.max() as i32)
                                <= walkable_climb_height as i32
                        {
                            span.set_area(previous_area_id);
                        }
                    }

                    // Copy the original walkable value regardless of whether we changed it.
                    // This prevents multiple consecutive non-walkable spans from being erroneously marked as walkable.
                    previous_span.replace(span.clone());
                    previous_was_walkable = walkable;
                    previous_area_id = span.area();
                }
            }
        }
    }
}
