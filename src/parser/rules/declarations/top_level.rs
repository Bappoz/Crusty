use crate::common::ast::ast::{Program, QualifierType};
use crate::common::ast::decl::Decl;
use crate::common::ast::stmt::Stmt;
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token::Token;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;
use crate::parser::rules::declarations::types::parse_type;

/// Parseia um programa C completo: sequência de declarações globais até EOF.
pub fn parse_program(parser: &mut Parser) -> Result<Program, CompilerError> {
    let mut decls = Vec::new();
    while !parser.is_at_end() {
        decls.push(parse_global_item(parser)?);
    }
    Ok(Program { decls })
}

/// Dispatcher do escopo global: tipo + lookahead para distinguir função de variável global.
fn parse_global_item(parser: &mut Parser) -> Result<Decl, CompilerError> {
    if parser.check(&TokenKind::Typedef) {
        return parse_typedef_decl(parser);
    }

    if is_enum_definition(parser) {
        return parse_enum_decl(parser);
    }

    let start = parser.peek().clone();
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

/// Continua o parsing de uma variável global após tipo e nome: `(= expr)? ;`.
pub(crate) fn parse_global_var_decl(
    parser: &mut Parser,
    qty: QualifierType,
    name: String,
    start: Token,
) -> Result<Decl, CompilerError> {
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

fn parse_typedef_decl(parser: &mut Parser) -> Result<Decl, CompilerError> {
    let start = parser.expect(&TokenKind::Typedef, "'typedef'")?.clone();
    let qty = parse_type(parser)?;

    let name_token = parser.advance().clone();
    let TokenKind::Identifier(alias) = name_token.kind else {
        return Err(parser.syntax_error(
            &name_token,
            "nome do typedef",
            &format!("{:?}", name_token.kind),
        ));
    };

    let semi = parser
        .expect(&TokenKind::Semicolon, "';' ao fim do typedef")?
        .clone();
    parser.register_type_name(alias.clone());
    let span = parser.join_span(parser.span_of(&start), parser.span_of(&semi));
    Ok(Decl::Typedef(qty, alias, span))
}

fn parse_enum_decl(parser: &mut Parser) -> Result<Decl, CompilerError> {
    let start = parser.expect(&TokenKind::Enum, "'enum'")?.clone();

    let name = if matches!(parser.peek_kind(), TokenKind::Identifier(_))
        && parser.peek_next().kind == TokenKind::LeftBrace
    {
        let name_token = parser.advance().clone();
        let TokenKind::Identifier(name) = name_token.kind else {
            unreachable!();
        };
        Some(name)
    } else {
        None
    };

    parser.expect(&TokenKind::LeftBrace, "'{' após enum")?;

    let mut variants = Vec::new();
    while !parser.check(&TokenKind::RightBrace) && !parser.is_at_end() {
        let variant_token = parser.advance().clone();
        let TokenKind::Identifier(variant_name) = variant_token.kind else {
            return Err(parser.syntax_error(
                &variant_token,
                "nome de variante de enum",
                &format!("{:?}", variant_token.kind),
            ));
        };

        let value = if parser.match_kind(&TokenKind::Equal) {
            Some(parser.parse_expr(0)?)
        } else {
            None
        };

        variants.push((variant_name, value));

        if parser.match_kind(&TokenKind::Comma) {
            continue;
        }
        break;
    }

    let rbrace = parser
        .expect(&TokenKind::RightBrace, "'}' ao fim do enum")?
        .clone();
    let semi = parser
        .expect(&TokenKind::Semicolon, "';' ao fim da declaração de enum")?
        .clone();

    let span = parser.join_span(parser.span_of(&start), parser.span_of(&semi));
    let _ = rbrace;
    Ok(Decl::EnumDecl(name, variants, span))
}

fn is_enum_definition(parser: &Parser) -> bool {
    if !parser.check(&TokenKind::Enum) {
        return false;
    }

    matches!(parser.peek_next().kind, TokenKind::LeftBrace)
        || (matches!(parser.peek_next().kind, TokenKind::Identifier(_))
            && parser.peek_n(2).kind == TokenKind::LeftBrace)
}
