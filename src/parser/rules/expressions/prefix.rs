use crate::common::ast::ast::QualifierType;
use crate::common::ast::expr::{Expr, Literal, PrefixOp, UnOp};
use crate::common::errors::error_data::Span;
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;

/// Parseia uma expressão prefix: operadores unários, literais, identificadores, agrupamentos e casts.
/// É o ponto de entrada principal do lado esquerdo no algoritmo Pratt.
pub fn parse_prefix_expr(parser: &mut Parser) -> Result<Expr, CompilerError> {
    let token = parser.peek().clone();
    let kind = parser.peek_kind().clone();

    if looks_like_cast(parser) {
        return parse_cast_expr(parser);
    }

    match kind {
        TokenKind::Bang
        | TokenKind::Tilde
        | TokenKind::Minus
        | TokenKind::PlusPlus
        | TokenKind::MinusMinus
        | TokenKind::Star
        | TokenKind::Ampersand
        | TokenKind::Sizeof => {
            let op = parser.advance().clone();
            let bp =
                crate::parser::precedence::prefix_binding_power(&op.kind).ok_or_else(|| {
                    parser.syntax_error(&op, "operador prefixo", &format!("{:?}", op.kind))
                })?;
            let rhs = parser.parse_expr(bp)?;
            let span = parser.join_span(parser.span_of(&op), rhs.span());
            build_prefix_expr(parser, op.kind, rhs, span)
        }
        TokenKind::LeftParen => {
            parser.advance();
            let expr = parser.parse_expr(0)?;
            parser.expect(&TokenKind::RightParen, "')' para fechar agrupamento")?;
            Ok(expr)
        }
        TokenKind::IntLiteral(v) => {
            parser.advance();
            Ok(Expr::Literal(Literal::Int(v), parser.span_of(&token)))
        }
        TokenKind::FloatLiteral(v) => {
            parser.advance();
            Ok(Expr::Literal(Literal::Double(v), parser.span_of(&token)))
        }
        TokenKind::StringLiteral(v) => {
            parser.advance();
            Ok(Expr::Literal(Literal::String(v), parser.span_of(&token)))
        }
        TokenKind::CharLiteral(v) => {
            parser.advance();
            Ok(Expr::Literal(Literal::Char(v), parser.span_of(&token)))
        }
        TokenKind::Identifier(name) => {
            parser.advance();
            Ok(Expr::Ident(name, parser.span_of(&token)))
        }
        _ => Err(parser.syntax_error(&token, "expressão", &format!("{:?}", token.kind))),
    }
}

/// Parseia uma expressão de cast do tipo `(tipo) expr`, consumindo os parênteses e o tipo.
pub fn parse_cast_expr(parser: &mut Parser) -> Result<Expr, CompilerError> {
    let lpar = parser
        .expect(&TokenKind::LeftParen, "'(' para iniciar cast")?
        .clone();
    let ty = parse_cast_type(parser)?;
    parser.expect(&TokenKind::RightParen, "')' após tipo no cast")?;
    let expr = parser.parse_expr(30)?;
    let span = parser.join_span(parser.span_of(&lpar), expr.span());
    Ok(Expr::Cast(ty, Box::new(expr), span))
}

/// Retorna `true` se o token atual parece ser o início de um cast `(tipo)`, usando lookahead de 1.
pub fn looks_like_cast(parser: &Parser) -> bool {
    if !parser.check(&TokenKind::LeftParen) {
        return false;
    }

    let Some(next) = parser.tokens.get(parser.pos + 1) else {
        return false;
    };

    matches!(
        next.kind,
        TokenKind::Const
            | TokenKind::Unsigned
            | TokenKind::Int
            | TokenKind::Char
            | TokenKind::Float
            | TokenKind::Double
            | TokenKind::Void
            | TokenKind::Struct
    )
}

/// Parseia o tipo dentro de um cast, incluindo qualificadores `const`/`unsigned` e ponteiros `*`.
pub fn parse_cast_type(parser: &mut Parser) -> Result<QualifierType, CompilerError> {
    crate::parser::rules::declarations::parse_type(parser)
}

/// Constrói o nó de expressão prefix correto para o operador `op` aplicado sobre `rhs`.
pub fn build_prefix_expr(
    parser: &Parser,
    op: TokenKind,
    rhs: Expr,
    span: Span,
) -> Result<Expr, CompilerError> {
    let expr = match op {
        TokenKind::Bang => Expr::Unary(UnOp::Not, Box::new(rhs), span),
        TokenKind::Minus => Expr::Unary(UnOp::Neg, Box::new(rhs), span),
        TokenKind::Star => Expr::Unary(UnOp::Deref, Box::new(rhs), span),
        TokenKind::Ampersand => Expr::Unary(UnOp::AddrOf, Box::new(rhs), span),
        TokenKind::Tilde => Expr::Unary(UnOp::BitNot, Box::new(rhs), span),
        TokenKind::Sizeof => Expr::Sizeof(Box::new(rhs), span),
        TokenKind::PlusPlus => Expr::Prefix(PrefixOp::Inc, Box::new(rhs), span),
        TokenKind::MinusMinus => Expr::Prefix(PrefixOp::Dec, Box::new(rhs), span),
        _ => {
            return Err(parser.syntax_error_from_span(
                span,
                "operador prefixo suportado",
                &format!("{:?}", op),
            ));
        }
    };

    Ok(expr)
}
