use crate::lexer::tokens::token_kind::TokenKind;

// Define a ordem de binding power usada pelo parser Pratt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None,
    Assignment,
    LogicalOr,
    LogicalAnd,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

// Mapeia cada token infixo para seu nível de precedência.
pub fn precedence_of(token: &TokenKind) -> Precedence {
    match token {
        TokenKind::Equal => Precedence::Assignment,
        TokenKind::OrOr => Precedence::LogicalOr,
        TokenKind::AndAnd => Precedence::LogicalAnd,
        TokenKind::EqualEqual | TokenKind::BangEqual => Precedence::Equality,
        TokenKind::Less | TokenKind::LessEqual | TokenKind::Greater | TokenKind::GreaterEqual => {
            Precedence::Comparison
        }
        TokenKind::Plus | TokenKind::Minus => Precedence::Term,
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
        TokenKind::LeftParen | TokenKind::LeftBracket => Precedence::Call,
        _ => Precedence::None,
    }
}