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
    Tilde, // ~
    Arrow, // ->
    Dot,   // .

    // Operadores aritméticos
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Caret,      // ^
    PlusPlus,   // ++
    MinusMinus, // --
    PlusEqual,  // +=
    MinusEqual, // -=
    StarEqual,  // *=
    SlashEqual, // /=

    // Operadores relacionais / igualdade
    EqualEqual,   // ==
    BangEqual,    // !=
    LessEqual,    // <=
    GreaterEqual, // >=
    Less,         // <
    Greater,      // >

    // Atribuição
    Equal, // =

    // Operadores lógicos / bitwise básicos
    Bang,      // !
    AndAnd,    // &&
    OrOr,      // ||
    Ampersand, // &
    Pipe,      // |

    // Operadores de shift
    LessLess,              // <<
    GreaterGreater,        // >>
    LessLessEqual,         // <<=
    GreaterGreaterEqual,   // >>= 

    // Delimitadores
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    Semicolon,    // ;
    Comma,        // ,
    LeftBracket,  // [
    RightBracket, // ]
    Colon,        // :

    // Literais
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),

    // Identificador
    Identifier(String),

    // Fim de arquivo
    Eof,

    // Desconhecido -> Guarda o char problematico descrito no char_utils (new)
    Unknown(char),
}
