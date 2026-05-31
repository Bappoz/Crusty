use crate::common::ast::ast::{Program, QualifierType};
use crate::common::ast::decl::Decl;
use crate::common::ast::stmt::Stmt;
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token::Token;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;
use crate::parser::rules::declarations::types::{parse_array_suffix, parse_type};

/// Parseia um programa C completo: sequência de declarações globais até EOF.
/// Ao encontrar erro, sincroniza no próximo `;` ou `}` e continua, acumulando todos os erros.
pub fn parse_program(parser: &mut Parser) -> Result<Program, Vec<CompilerError>> {
    let mut decls = Vec::new();
    while !parser.is_at_end() {
        match parse_global_item(parser) {
            Ok(decl) => decls.push(decl),
            Err(e) => {
                parser.diagnostics.push(e);
                parser.synchronize();
            }
        }
    }
    if parser.diagnostics.is_empty() {
        Ok(Program { decls })
    } else {
        Err(std::mem::take(&mut parser.diagnostics))
    }
}

/// Dispatcher do escopo global: tipo + lookahead para distinguir função de variável global.
fn parse_global_item(parser: &mut Parser) -> Result<Decl, CompilerError> {
    let start = parser.peek().clone();

    if parser.check(&TokenKind::Typedef) {
        return parse_typedef_decl(parser);
    }

    if parser.check(&TokenKind::Enum)
        && matches!(parser.peek_at(1).kind, TokenKind::Identifier(_))
        && parser.peek_at(2).kind == TokenKind::LeftBrace
    {
        return parse_enum_decl(parser);
    }

    if parser.check(&TokenKind::Struct)
        && matches!(parser.peek_at(1).kind, TokenKind::Identifier(_))
        && parser.peek_at(2).kind == TokenKind::LeftBrace
    {
        return parse_struct_decl(parser);
    }

    let qty = parse_type(parser)?;

    let name_token = parser.advance().clone();
    let TokenKind::Identifier(name) = name_token.kind else {
        return Err(parser.syntax_error(
            &name_token,
            "identificador",
            &format!("{:?}", name_token.kind),
        ));
    };

    if parser.check(&TokenKind::LeftParen) {
        parse_function_decl(parser, qty, name, start)
    } else {
        parse_global_var_decl(parser, qty, name, start)
    }
}

/// Continua o parsing de uma função após tipo e nome: `( params ) block`.
pub(crate) fn parse_function_decl(
    parser: &mut Parser,
    return_type: QualifierType,
    name: String,
    start: Token,
) -> Result<Decl, CompilerError> {
    parser.expect(&TokenKind::LeftParen, "'(' após nome da função")?;

    let params = parse_params(parser)?;

    parser.expect(&TokenKind::RightParen, "')' após parâmetros")?;

    let block = crate::parser::rules::statements::parse_block(parser)?;

    let span = parser.join_span(parser.span_of(&start), block.span());
    let Stmt::Block(stmts, _) = block else {
        unreachable!("parse_block sempre retorna Stmt::Block");
    };

    Ok(Decl::Function(return_type, name, params, stmts, span))
}

/// Parseia a lista de parâmetros: `void` | `(tipo ident (, tipo ident)*)?`.
fn parse_params(parser: &mut Parser) -> Result<Vec<(QualifierType, String)>, CompilerError> {
    if parser.check(&TokenKind::Void) && parser.peek_next().kind == TokenKind::RightParen {
        parser.advance();
        return Ok(Vec::new());
    }

    if parser.check(&TokenKind::RightParen) {
        return Ok(Vec::new());
    }

    let mut params = Vec::new();
    params.push(parse_param(parser)?);

    while parser.match_kind(&TokenKind::Comma) {
        params.push(parse_param(parser)?);
    }

    Ok(params)
}

/// Parseia um único parâmetro: `tipo ident`.
fn parse_param(parser: &mut Parser) -> Result<(QualifierType, String), CompilerError> {
    let qty = parse_type(parser)?;
    let name_token = parser.advance().clone();
    let TokenKind::Identifier(name) = name_token.kind else {
        return Err(parser.syntax_error(
            &name_token,
            "nome do parâmetro",
            &format!("{:?}", name_token.kind),
        ));
    };
    Ok((qty, name))
}

/// Parseia uma definição de struct: `struct Name { type field; ... };`
fn parse_struct_decl(parser: &mut Parser) -> Result<Decl, CompilerError> {
    let start = parser.advance().clone(); // consome 'struct'

    let name_token = parser.advance().clone();
    let TokenKind::Identifier(name) = name_token.kind else {
        return Err(parser.syntax_error(
            &name_token,
            "nome da struct",
            &format!("{:?}", name_token.kind),
        ));
    };

    parser.expect(&TokenKind::LeftBrace, "'{' após nome da struct")?;

    let mut fields = Vec::new();
    while !parser.check(&TokenKind::RightBrace) && !parser.is_at_end() {
        let field_type = parse_type(parser)?;
        let field_name_token = parser.advance().clone();
        let TokenKind::Identifier(field_name) = field_name_token.kind else {
            return Err(parser.syntax_error(
                &field_name_token,
                "nome do campo",
                &format!("{:?}", field_name_token.kind),
            ));
        };
        parser.expect(&TokenKind::Semicolon, "';' após campo da struct")?;
        fields.push((field_type, field_name));
    }

    let end = parser
        .expect(&TokenKind::RightBrace, "'}' ao fim da struct")?
        .clone();
    parser.expect(&TokenKind::Semicolon, "';' após definição de struct")?;

    let span = parser.join_span(parser.span_of(&start), parser.span_of(&end));
    Ok(Decl::StructDecl(name, fields, span))
}

/// Parseia uma definição de enum: `enum Name { VAR = expr, ... };`
fn parse_enum_decl(parser: &mut Parser) -> Result<Decl, CompilerError> {
    let start = parser.advance().clone();

    let name_token = parser.advance().clone();
    let TokenKind::Identifier(name) = name_token.kind else {
        return Err(parser.syntax_error(
            &name_token,
            "nome do enum",
            &format!("{:?}", name_token.kind),
        ));
    };

    parser.expect(&TokenKind::LeftBrace, "'{' após nome do enum")?;

    let mut variants = Vec::new();
    while !parser.check(&TokenKind::RightBrace) && !parser.is_at_end() {
        let variant_token = parser.advance().clone();
        let TokenKind::Identifier(variant_name) = variant_token.kind else {
            return Err(parser.syntax_error(
                &variant_token,
                "nome do variant",
                &format!("{:?}", variant_token.kind),
            ));
        };

        let value = if parser.match_kind(&TokenKind::Equal) {
            Some(parser.parse_expr(0)?)
        } else {
            None
        };

        variants.push((variant_name, value));

        if !parser.match_kind(&TokenKind::Comma) {
            break;
        }
    }

    let end = parser
        .expect(&TokenKind::RightBrace, "'}' ao fim do enum")?
        .clone();
    parser.expect(&TokenKind::Semicolon, "';' após definição de enum")?;

    let span = parser.join_span(parser.span_of(&start), parser.span_of(&end));
    Ok(Decl::EnumDecl(name, variants, span))
}

/// Parseia um typedef: `typedef <tipo> <alias>;`
fn parse_typedef_decl(parser: &mut Parser) -> Result<Decl, CompilerError> {
    let start = parser.expect(&TokenKind::Typedef, "'typedef'")?.clone();
    let qty = parse_type(parser)?;

    let alias_token = parser.advance().clone();
    let TokenKind::Identifier(alias) = alias_token.kind else {
        return Err(parser.syntax_error(
            &alias_token,
            "nome do alias",
            &format!("{:?}", alias_token.kind),
        ));
    };

    let end = parser
        .expect(&TokenKind::Semicolon, "';' após typedef")?
        .clone();

    parser.register_type_name(alias.clone());

    let span = parser.join_span(parser.span_of(&start), parser.span_of(&end));
    Ok(Decl::Typedef(qty, alias, span))
}

/// Continua o parsing de uma variável global após tipo e nome: `([N])? (= expr)? ;`.
pub(crate) fn parse_global_var_decl(
    parser: &mut Parser,
    qty: QualifierType,
    name: String,
    start: Token,
) -> Result<Decl, CompilerError> {
    let qty = parse_array_suffix(parser, qty)?;

    let init = if parser.match_kind(&TokenKind::Equal) {
        Some(parser.parse_expr(0)?)
    } else {
        None
    };

    let semi = parser
        .expect(&TokenKind::Semicolon, "';' ao fim da declaração global")?
        .clone();
    let span = parser.join_span(parser.span_of(&start), parser.span_of(&semi));
    Ok(Decl::GlobalVar(qty, name, init, span))
}
