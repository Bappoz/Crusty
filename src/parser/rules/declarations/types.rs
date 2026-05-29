use crate::common::ast::ast::{QualifierType, Type};
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;

// Retorna `true` se o token inicia uma declaração de tipo
pub fn starts_type(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Const
            | TokenKind::Unsigned
            | TokenKind::Int
            | TokenKind::Long
            | TokenKind::Float
            | TokenKind::Double
            | TokenKind::Struct
            | TokenKind::Void
            | TokenKind::Char
    )
}

/// Parseia um tipo C completo: `const? Unsigned? base *...`
/// Aceita qualificadores em qualquer ordem
pub fn parse_type(parser: &mut Parser) -> Result<QualifierType, CompilerError> {
    let mut is_const = false;
    let mut is_unsigned = false;

    loop {
        if parser.match_kind(&TokenKind::Const) {
            is_const = true;
        } else if parser.match_kind(&TokenKind::Unsigned) {
            is_unsigned = true;
        } else {
            break;
        }
    }

    let base = match parser.peek_kind() {
        TokenKind::Int => {
            parser.advance();
            Type::Int
        }
        TokenKind::Long => {
            parser.advance();
            // A AST ainda não possui Type::Long; tratamos como inteiro por ora.
            Type::Int
        }
        TokenKind::Char => {
            parser.advance();
            Type::Char
        }
        TokenKind::Float => {
            parser.advance();
            Type::Float
        }
        TokenKind::Double => {
            parser.advance();
            Type::Double
        }
        TokenKind::Void => {
            parser.advance();
            Type::Void
        }
        TokenKind::Struct => {
            parser.advance();
            let t = parser.advance().clone();
            let TokenKind::Identifier(name) = t.kind else {
                return Err(parser.syntax_error(&t, "nome de struct", &format!("{:?}", t.kind)));
            };
            Type::Struct(name)
        }
        _ => {
            let found = parser.peek().clone();
            return Err(parser.syntax_error(&found, "tipo", &format!("{:?}", found.kind)));
        }
    };

    // Ponteiros
    let mut ty = base;
    while parser.match_kind(&TokenKind::Star) {
        ty = Type::Pointer(Box::new(ty));
    }

    Ok(QualifierType {
        ty,
        is_const,
        is_unsigned,
    })
}
