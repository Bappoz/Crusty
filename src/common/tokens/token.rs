use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Palavras reservadas de controle de fluxo
    If,
    While,
    For,
    Else,
    Do,
    Switch,
    Case,
    Default,
    Break,
    Continue,

    // Palavras reservadas de tipos
    Int,
    Char,
    Float,
    Double,
    Void,
    Struct,
    Enum,
    Union,

    // Outras palavras reservadas de C
    Typedef,
    Const,
    Static,
    Extern,
    Auto,
    Register,
    Signed,
    Unsigned,
    Short,
    Long,
    Volatile,
    Inline,
    Sizeof,
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

    // Operadores lógicos / bitwise básicos
    Bang,          // !
    AndAnd,        // &&
    OrOr,          // ||
    Ampersand,     // &
    Pipe,          // |

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