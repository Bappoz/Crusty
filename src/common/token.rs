use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Palavras reservadas
    If,
    While,
    For,
    Loop,
    Else,
    Match,
    Let,
    Fn,
    Struct,
    Enum,
    Impl,
    Trait,
    Pub,
    Mod,
    Use,
    Const,
    Static,
    Int,
    Return,

    // Operadores aritméticos
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /

    // Operadores relacionais / igualdade
    EqualEqual,    // ==
    BangEqual,     // !=
    LessEqual,     // <=
    GreaterEqual,  // >=
    Less,          // <
    Greater,       // >

    // Atribuição
    Equal,         // =

    // Delimitadores
    LeftParen,   // (
    RightParen,  // )
    LeftBrace,   // {
    RightBrace,  // }
    Semicolon,   // ;
    Comma,       // ,

    // Literais
    IntLiteral,
    FloatLiteral,
    StringLiteral,
    CharLiteral,

    // Identificador
    Identifier,

    // Fim de arquivo
    Eof,

    // Desconhecido
    Unknown,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_display_shows_lexeme() {
        let token = Token {
            kind: TokenKind::Identifier,
            lexeme: "foo".to_string(),
            line: 1,
            col: 1,
        };

        assert_eq!(token.to_string(), "foo");
    }

    #[test]
    fn token_kind_eq_works() {
        let t1 = TokenKind::If;
        let t2 = TokenKind::If;
        let t3 = TokenKind::While;

        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
    }
}