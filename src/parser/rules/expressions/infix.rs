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
        TokenKind::LessLess => BinOp::Shl,
        TokenKind::GreaterGreater => BinOp::Shr,
        TokenKind::Plus => BinOp::Add,
        TokenKind::Minus => BinOp::Sub,
        TokenKind::Star => BinOp::Mul,
        TokenKind::Slash => BinOp::Div,
        TokenKind::Percent => BinOp::Mod,
        _ => return Err(parser.syntax_error(found, "operador binário", &format!("{:?}", kind))),
    };
    Ok(op)
}

/// Converte um `TokenKind` de atribuição composta no `BinOp` da operação subjacente.
/// Por exemplo, `+=` → `BinOp::Add`, `>>=` → `BinOp::Shr`.
/// Retorna erro sintático se o token não for um operador de atribuição composta suportado.
pub fn token_to_compound_bin_op(
    parser: &Parser,
    kind: &TokenKind,
    found: &Token,
) -> Result<BinOp, CompilerError> {
    let op = match kind {
        TokenKind::PlusEqual => BinOp::Add,
        TokenKind::MinusEqual => BinOp::Sub,
        TokenKind::StarEqual => BinOp::Mul,
        TokenKind::SlashEqual => BinOp::Div,
        TokenKind::PercentEqual => BinOp::Mod,
        TokenKind::AmpersandEqual => BinOp::BitAnd,
        TokenKind::PipeEqual => BinOp::BitOr,
        TokenKind::CaretEqual => BinOp::BitXor,
        TokenKind::LessLessEqual => BinOp::Shl,
        TokenKind::GreaterGreaterEqual => BinOp::Shr,
        _ => {
            return Err(parser.syntax_error(
                found,
                "operador de atribuição composta",
                &format!("{:?}", kind),
            ))
        }
    };
    Ok(op)
}

/// Parseia o operador ternário `? then : else` após o `?` já ter sido consumido.
/// O then aceita qualquer expressão (min_bp=0); o else é right-assoc com rbp do `?`.
pub fn parse_ternary(parser: &mut Parser, lhs: Expr, rbp: u8) -> Result<Expr, CompilerError> {
    let then_expr = parser.parse_expr(0)?;
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
