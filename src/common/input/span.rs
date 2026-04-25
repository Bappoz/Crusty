#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSpan {
    pub start: usize,
    pub end: usize,
}

impl ByteSpan {
    /// Cria um novo `ByteSpan` com os offsets de início e fim em bytes no source.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}
