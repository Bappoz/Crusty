use crate::common::ast::expr::{BinOp, Expr};
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token::Token;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;

/// Converte um `TokenKind` de operador infix no `BinOp` correspondente do AST.
/// Retorna erro sintático se o token não for um operador binário suportado.
pub fn token_to_bin_op(
    parser: &Parser,
    kind: &TokenKind,
    found: &Token,
) -> Result<BinOp, CompilerError> {
    let op = match kind {
        TokenKind::OrOr => BinOp::Or,
        TokenKind::AndAnd => BinOp::And,
        TokenKind::Pipe => BinOp::BitOr,
        TokenKind::Caret => BinOp::BitXor,
        TokenKind::Ampersand => BinOp::BitAnd,
        TokenKind::EqualEqual => BinOp::Eq,
        TokenKind::BangEqual => BinOp::Neq,
        TokenKind::Less => BinOp::Less,
        TokenKind::Greater => BinOp::Greater,
        TokenKind::LessEqual => BinOp::Leq,
        TokenKind::GreaterEqual => BinOp::Geq,
        TokenKind::Plus => BinOp::Add,
        TokenKind::Minus => BinOp::Sub,
        TokenKind::Star => BinOp::Mul,
        TokenKind::Slash => BinOp::Div,
        TokenKind::Percent => BinOp::Mod,
        _ => return Err(parser.syntax_error(found, "operador binário", &format!("{:?}", kind))),
    };
    Ok(op)
}

/// Parseia o operador ternário `? then : else` após o `?` já ter sido consumido.
pub fn parse_ternary(parser: &mut Parser, lhs: Expr, rbp: u8) -> Result<Expr, CompilerError> {
    let then_expr = parser.parse_expr(rbp)?;
    parser.expect(&TokenKind::Colon, "':' após expressão do braço true em ?:")?;
    let else_expr = parser.parse_expr(rbp)?;

    let span = parser.join_span(lhs.span(), else_expr.span());
    Ok(Expr::Ternary(
        Box::new(lhs),
        Box::new(then_expr),
        Box::new(else_expr),
        span,
    ))
}
