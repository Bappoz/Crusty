use crate::common::utils::char_utils::is_ident_continue;
use crate::lexer::scanner::Scanner;
use crate::lexer::tokens::TokenKind;

pub trait IdentifierRules {
    fn lex_identifier(&mut self, first: char, line: usize, col: usize);
}

impl IdentifierRules for Scanner {
    /// Consome caracteres válidos de identificador a partir de `first` e emite o token correspondente,
    /// distinguindo palavras-chave de identificadores de usuário via `lookup_keyword`.
    fn lex_identifier(&mut self, first: char, line: usize, col: usize) {
        let mut buf = String::from(first);

        while let Some(c) = self.src.peek() {
            if is_ident_continue(c) {
                buf.push(c);
                self.src.advance();
            } else {
                break;
            }
        }

        let kind = lookup_keyword(&buf).unwrap_or(TokenKind::Identifier(buf.clone()));

        self.emit_at(kind, line, col);
    }
}

/// Mapeia uma string para o `TokenKind` de keyword correspondente, ou `None` se for identificador de usuário.
fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    match ident {
        // Controle de fluxo
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "do" => Some(TokenKind::Do),
        "switch" => Some(TokenKind::Switch),
        "case" => Some(TokenKind::Case),
        "default" => Some(TokenKind::Default),
        "break" => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "return" => Some(TokenKind::Return),

        // Tipos primitivos
        "int" => Some(TokenKind::Int),
        "char" => Some(TokenKind::Char),
        "float" => Some(TokenKind::Float),
        "double" => Some(TokenKind::Double),
        "void" => Some(TokenKind::Void),
        "struct" => Some(TokenKind::Struct),
        "enum" => Some(TokenKind::Enum),
        "union" => Some(TokenKind::Union),

        // Outras keywords do C
        "typedef" => Some(TokenKind::Typedef),
        "const" => Some(TokenKind::Const),
        "static" => Some(TokenKind::Static),
        "extern" => Some(TokenKind::Extern),
        "auto" => Some(TokenKind::Auto),
        "register" => Some(TokenKind::Register),
        "signed" => Some(TokenKind::Signed),
        "unsigned" => Some(TokenKind::Unsigned),
        "short" => Some(TokenKind::Short),
        "long" => Some(TokenKind::Long),
        "volatile" => Some(TokenKind::Volatile),
        "inline" => Some(TokenKind::Inline),
        "sizeof" => Some(TokenKind::Sizeof),

        // Não é keyword — é identificador de usuário
        _ => None,
    }
}
