use crate::common::ast::expr::{Expr, MemberAccess, PostfixOp};
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;

/// Tenta parsear uma operação postfix (`()`, `[]`, `.`, `->`, `++`, `--`) sobre `lhs`.
/// Retorna `Ok(true)` se consumiu um postfix, `Ok(false)` se não há postfix aplicável.
pub fn try_parse_postfix(parser: &mut Parser, lhs: &mut Expr) -> Result<bool, CompilerError> {
    match parser.peek_kind() {
        TokenKind::LeftParen => {
            let start = lhs.span();
            parser.advance();
            let mut args = Vec::new();

            if !parser.check(&TokenKind::RightParen) {
                loop {
                    args.push(parser.parse_expr(0)?);
                    if !parser.match_kind(&TokenKind::Comma) {
                        break;
                    }
                }
            }

            let end = parser
                .expect(&TokenKind::RightParen, "')' ao fechar chamada")?
                .clone();
            let span = parser.join_span(start, parser.span_of(&end));
            *lhs = Expr::Call(Box::new(lhs.clone()), args, span);
            Ok(true)
        }
        TokenKind::LeftBracket => {
            let start = lhs.span();
            parser.advance();
            let index = parser.parse_expr(0)?;
            let end = parser
                .expect(&TokenKind::RightBracket, "']' ao fechar indexação")?
                .clone();
            let span = parser.join_span(start, parser.span_of(&end));
            *lhs = Expr::Index(Box::new(lhs.clone()), Box::new(index), span);
            Ok(true)
        }
        TokenKind::Dot | TokenKind::Arrow => {
            let op = parser.advance().clone();
            let field_token = parser.advance().clone();
            let TokenKind::Identifier(field_name) = field_token.kind.clone() else {
                return Err(parser.syntax_error(
                    &field_token,
                    "identificador de campo",
                    &format!("{:?}", field_token.kind),
                ));
            };

            let span = parser.join_span(lhs.span(), parser.span_of(&field_token));
            let access = if op.kind == TokenKind::Dot {
                MemberAccess::Direct
            } else {
                MemberAccess::Pointer
            };
            *lhs = Expr::Member(Box::new(lhs.clone()), access, field_name, span);
            Ok(true)
        }
        TokenKind::PlusPlus | TokenKind::MinusMinus => {
            let op = parser.advance().clone();
            let span = parser.join_span(lhs.span(), parser.span_of(&op));
            let kind = if op.kind == TokenKind::PlusPlus {
                PostfixOp::Inc
            } else {
                PostfixOp::Dec
            };
            *lhs = Expr::Postfix(kind, Box::new(lhs.clone()), span);
            Ok(true)
        }
        _ => Ok(false),
    }
}
