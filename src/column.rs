use crate::span::Span;

#[derive(Default)]
pub(crate) struct Column {
    /// The spans of the column, from bottom to top
    pub(crate) spans: Vec<Span>,
}
