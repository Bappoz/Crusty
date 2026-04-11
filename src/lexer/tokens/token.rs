use crate::lexer::tokens::token_kind::TokenKind;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Exibir o lexema é o que o teste pede
        write!(f, "{}", self.lexeme)
    }
}
