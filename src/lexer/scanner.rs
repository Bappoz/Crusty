use crate::common::errors::{
    error_data::Span,
    types::{CompilerError, LexicalError},
};

use crate::common::input::source::SourceFile;
use crate::common::utils::char_utils::*;
use crate::lexer::tokens::{token::Token, token_kind::TokenKind};

pub struct Scanner {
    pub src: SourceFile,
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<CompilerError>,
}

impl Scanner {
    // Construtor
    pub fn new(src: SourceFile) -> Self {
        Self {
            src,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    // Emite um token com posição explícita.
    pub fn emit_at(&mut self, kind: TokenKind, lexeme: &str, line: usize, col: usize) {
        self.tokens.push(Token {
            kind,
            lexeme: lexeme.to_string(),
            line,
            col,
        });
    }

    // Emite um token Unknown e registra o diagnostico de error
    pub fn emit_unknown(&mut self, c: char, line: usize, col: usize) {
        self.diagnostics.push(CompilerError::Lexical(LexicalError {
            span: Span {
                line,
                column_start: col,
                column_end: col + 1,
            },
            invalid_char: c,
        }));
        self.emit_at(TokenKind::Unknown(c), &c.to_string(), line, col);
    }
}
