use crate::lexer::tokens::token_kind::TokenKind;

// Mantemos apenas as binding powers efetivamente usadas pelo parser.
// Se um Pratt table-driven completo voltar a ser necessário, um enum de precedência
// pode ser reintroduzido aqui com uso real no fluxo de parse.

/// Retorna `(lbp, rbp, is_ternary)` para operadores infix; `None` se o token não for infix.
/// `lbp` é o poder de ligação à esquerda e `rbp` à direita, seguindo a tabela de precedência do C.
pub fn infix_binding_power(op: &TokenKind) -> Option<(u8, u8, bool)> {
    let bp = match op {
        TokenKind::Equal => (1, 1, false),
        TokenKind::OrOr => (2, 3, false),
        TokenKind::AndAnd => (4, 5, false),
        TokenKind::Pipe => (6, 7, false),
        TokenKind::Caret => (8, 9, false),
        TokenKind::Ampersand => (10, 11, false),
        TokenKind::EqualEqual | TokenKind::BangEqual => (12, 13, false),
        TokenKind::Less | TokenKind::Greater | TokenKind::LessEqual | TokenKind::GreaterEqual => {
            (14, 15, false)
        }
        TokenKind::Plus | TokenKind::Minus => (18, 19, false),
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => (20, 21, false),
        _ => return None,
    };
    Some(bp)
}

/// Retorna o poder de ligação (rbp) de um operador prefix; `None` se não for prefix reconhecido.
pub fn prefix_binding_power(op: &TokenKind) -> Option<u8> {
    match op {
        TokenKind::Bang
        | TokenKind::Tilde
        | TokenKind::Minus
        | TokenKind::PlusPlus
        | TokenKind::MinusMinus
        | TokenKind::Star
        | TokenKind::Ampersand
        | TokenKind::Sizeof => Some(30),
        _ => None,
    }
}
