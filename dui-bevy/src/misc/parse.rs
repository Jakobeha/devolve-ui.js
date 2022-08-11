use std::ops::Range;
use derive_more::{Display, Error};

#[derive(Debug, Clone, PartialEq, Display, Error)]
#[display(fmt = "{{at {}..{}, {}}}", "span.start", "span.end", reason)]
pub struct ParseError<T> {
    pub reason: T,
    pub span: Range<usize>
}

impl<T> ParseError<T> {
    pub fn offset(&mut self, offset: usize) {
        self.span.start += offset;
        self.span.end += offset;
    }
}