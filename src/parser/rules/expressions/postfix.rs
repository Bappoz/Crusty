use crate::common::ast::expr::{Expr, MemberAccess, PostfixOp};
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;

/// Tenta parsear uma operação postfix (`()`, `[]`, `.`, `->`, `++`, `--`) sobre `lhs`.
///
/// Recebe `lhs` **por valor** para encapsulá-lo em um novo nó movendo (e não clonando)
/// a subexpressão. Encadeamentos como `a.b.c.d.e.f` assim crescem em O(n) em vez de O(n²).
///
/// Retorna `Ok((expr, true))` se consumiu um postfix (com o novo nó já construído) ou
/// `Ok((lhs, false))` quando não há postfix aplicável (devolvendo `lhs` intacto).
pub fn try_parse_postfix(parser: &mut Parser, lhs: Expr) -> Result<(Expr, bool), CompilerError> {
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
            let new_expr = Expr::Call(Box::new(lhs), args, span);
            Ok((new_expr, true))
        }
        TokenKind::LeftBracket => {
            let start = lhs.span();
            parser.advance();
            let index = parser.parse_expr(0)?;
            let end = parser
                .expect(&TokenKind::RightBracket, "']' ao fechar indexação")?
                .clone();
            let span = parser.join_span(start, parser.span_of(&end));
            let new_expr = Expr::Index(Box::new(lhs), Box::new(index), span);
            Ok((new_expr, true))
        }
        TokenKind::Dot | TokenKind::Arrow => {
            let start = lhs.span();
            let op = parser.advance().clone();
            let field_token = parser.advance().clone();
            let TokenKind::Identifier(field_name) = field_token.kind.clone() else {
                return Err(parser.syntax_error(
                    &field_token,
                    "identificador de campo",
                    &format!("{:?}", field_token.kind),
                ));
            };

            let span = parser.join_span(start, parser.span_of(&field_token));
            let access = if op.kind == TokenKind::Dot {
                MemberAccess::Direct
            } else {
                MemberAccess::Pointer
            };
            let new_expr = Expr::Member(Box::new(lhs), access, field_name, span);
            Ok((new_expr, true))
        }
        TokenKind::PlusPlus | TokenKind::MinusMinus => {
            let start = lhs.span();
            let op = parser.advance().clone();
            let span = parser.join_span(start, parser.span_of(&op));
            let kind = if op.kind == TokenKind::PlusPlus {
                PostfixOp::Inc
            } else {
                PostfixOp::Dec
            };
            let new_expr = Expr::Postfix(kind, Box::new(lhs), span);
            Ok((new_expr, true))
        }
        _ => Ok((lhs, false)),
    }
}
