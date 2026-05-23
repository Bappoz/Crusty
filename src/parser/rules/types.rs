use crate::common::ast::ast::QualifierType;
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;

/// Parseia o tipo dentro de um cast, incluindo qualificadores `const`/`unsigned` e ponteiros `*`.
pub fn parse_cast_type(parser: &mut Parser) -> Result<QualifierType, CompilerError> {
    let mut is_const = false;
    let mut is_unsigned = false;

    if parser.match_kind(&TokenKind::Const) {
        is_const = true;
    }

    if parser.match_kind(&TokenKind::Unsigned) {
        is_unsigned = true;
    }

    let base = match parser.peek_kind() {
        TokenKind::Int => {
            parser.advance();
            crate::common::ast::ast::Type::Int
        }
        TokenKind::Char => {
            parser.advance();
            crate::common::ast::ast::Type::Char
        }
        TokenKind::Float => {
            parser.advance();
            crate::common::ast::ast::Type::Float
        }
        TokenKind::Double => {
            parser.advance();
            crate::common::ast::ast::Type::Double
        }
        TokenKind::Void => {
            parser.advance();
            crate::common::ast::ast::Type::Void
        }
        TokenKind::Struct => {
            parser.advance();
            let t = parser.advance().clone();
            let TokenKind::Identifier(name) = t.kind else {
                return Err(parser.syntax_error(&t, "nome de struct", &format!("{:?}", t.kind)));
            };
            crate::common::ast::ast::Type::Struct(name)
        }
        _ => {
            let found = parser.peek().clone();
            return Err(parser.syntax_error(
                &found,
                "tipo para cast",
                &format!("{:?}", found.kind),
            ));
        }
    };

    let mut ty = base;
    while parser.match_kind(&TokenKind::Star) {
        ty = crate::common::ast::ast::Type::Pointer(Box::new(ty));
    }

    Ok(QualifierType {
        ty,
        is_const,
        is_unsigned,
    })
}
