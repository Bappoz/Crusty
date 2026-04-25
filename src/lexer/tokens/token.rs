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
    /// Formata o token exibindo o intervalo de bytes que ele ocupa no source.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}..{}]", self.span.start, self.span.end)
    }
}
