use crate::common::ast::stmt::Stmt;
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;

/// Dispatcher principal: decide qual parser de statement usar com base no token atual.
///
/// Nesta iteração (PARSER-01) reconhece apenas:
///   `{` → bloco, `return` → return, `break` → break, `continue` → continue,
///   qualquer outra coisa → statement de expressão.
/// Palavras-chave de controle de fluxo (`if`, `while`, `for`) e declarações de
/// variáveis serão adicionadas nas issues subsequentes.
pub fn parse_stmt(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    match parser.peek_kind().clone() {
        TokenKind::LeftBrace => parse_block(parser),
        TokenKind::Return => parse_return(parser),
        TokenKind::Break => parse_break(parser),
        TokenKind::Continue => parse_continue(parser),
        _ => parse_expr_stmt(parser),
    }
}

/// Parseia um bloco: `{ stmt* }`.
pub(super) fn parse_block(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let lbrace = parser
        .expect(&TokenKind::LeftBrace, "'{' para iniciar bloco")?
        .clone();

    let mut stmts = Vec::new();
    while !parser.check(&TokenKind::RightBrace) && !parser.is_at_end() {
        stmts.push(parse_stmt(parser)?);
    }

    let rbrace = parser
        .expect(&TokenKind::RightBrace, "'}' para fechar bloco")?
        .clone();

    let span = parser.join_span(parser.span_of(&lbrace), parser.span_of(&rbrace));
    Ok(Stmt::Block(stmts, span))
}

/// Parseia `return [expr];`.
fn parse_return(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::Return, "'return'")?.clone();

    let value = if parser.check(&TokenKind::Semicolon) {
        None
    } else {
        Some(parser.parse_expr(0)?)
    };

    let semi = parser
        .expect(&TokenKind::Semicolon, "';' após return")?
        .clone();
    let span = parser.join_span(parser.span_of(&kw), parser.span_of(&semi));
    Ok(Stmt::Return(value, span))
}

/// Parseia `break;`.
fn parse_break(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::Break, "'break'")?.clone();
    let semi = parser
        .expect(&TokenKind::Semicolon, "';' após break")?
        .clone();
    let span = parser.join_span(parser.span_of(&kw), parser.span_of(&semi));
    Ok(Stmt::Break(span))
}

/// Parseia `continue;`.
fn parse_continue(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::Continue, "'continue'")?.clone();
    let semi = parser
        .expect(&TokenKind::Semicolon, "';' após continue")?
        .clone();
    let span = parser.join_span(parser.span_of(&kw), parser.span_of(&semi));
    Ok(Stmt::Continue(span))
}

/// Parseia qualquer expressão seguida de `;`.
fn parse_expr_stmt(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let expr = parser.parse_expr(0)?;
    let expr_span = expr.span();
    let semi = parser
        .expect(&TokenKind::Semicolon, "';' após expressão")?
        .clone();
    let span = parser.join_span(expr_span, parser.span_of(&semi));
    Ok(Stmt::ExprStmt(expr, span))
}
