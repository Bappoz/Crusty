use crate::common::ast::ast::{QualifierType, Type};
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;

/// Consome sufixos `[expr?]` após o nome de uma variável e envolve o tipo em `Type::Array`.
/// Suporta múltiplas dimensões: `int arr[3][4]` → `Array(Array(Int, Some(4)), Some(3))`.
pub fn parse_array_suffix(
    parser: &mut Parser,
    mut qty: QualifierType,
) -> Result<QualifierType, CompilerError> {
    let mut dimensions = Vec::new();

    while parser.check(&TokenKind::LeftBracket) {
        parser.advance();

        let size = if parser.check(&TokenKind::RightBracket) {
            None
        } else if let TokenKind::IntLiteral(value) = parser.peek_kind() {
            let value = *value;
            parser.parse_expr(0)?;
            usize::try_from(value).ok()
        } else {
            parser.parse_expr(0)?;
            None
        };

        parser.expect(&TokenKind::RightBracket, "']' ao fim do tamanho do array")?;
        dimensions.push(size);
    }

    for size in dimensions.into_iter().rev() {
        qty.ty = Type::Array(Box::new(qty.ty), size);
    }

    Ok(qty)
}

// Retorna `true` se o token inicia uma declaração de tipo
pub fn starts_type(parser: &Parser) -> bool {
    starts_type_kind(parser, parser.peek_kind())
}

pub fn starts_type_kind(parser: &Parser, kind: &TokenKind) -> bool {
    match kind {
        TokenKind::Const
        | TokenKind::Unsigned
        | TokenKind::Int
        | TokenKind::Long
        | TokenKind::Short
        | TokenKind::Float
        | TokenKind::Double
        | TokenKind::Struct
        | TokenKind::Void
        | TokenKind::Char
        | TokenKind::Enum => true,
        TokenKind::Identifier(name) => parser.is_type_name(name),
        _ => false,
    }
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
            Type::Long
        }
        TokenKind::Short => {
            parser.advance();
            Type::Short
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
        TokenKind::Enum => {
            parser.advance();
            let t = parser.advance().clone();
            let TokenKind::Identifier(name) = t.kind else {
                return Err(parser.syntax_error(&t, "nome de enum", &format!("{:?}", t.kind)));
            };
            Type::Enum(name)
        }
        TokenKind::Identifier(name) if parser.is_type_name(name) => {
            let name = name.clone();
            parser.advance();
            Type::Alias(name)
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
