use crate::common::input::span::ByteSpan;
use crate::lexer::tokens::token_kind::TokenKind;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: ByteSpan,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{}]", self.span.start, self.span.end)
    }
}
