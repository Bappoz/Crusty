use crate::common::ast::stmt::Stmt;
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;
use crate::parser::rules::expressions::prefix::parse_cast_type;

/// Dispatcher principal: decide qual parser de statement usar com base no token atual.
pub fn parse_stmt(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    match parser.peek_kind().clone() {
        TokenKind::LeftBrace => parse_block(parser),
        TokenKind::If => parse_if(parser),
        TokenKind::While => parse_while(parser),
        TokenKind::For => parse_for(parser),
        TokenKind::Return => parse_return(parser),
        TokenKind::Break => parse_break(parser),
        TokenKind::Continue => parse_continue(parser),
        TokenKind::Int
        | TokenKind::Char
        | TokenKind::Float
        | TokenKind::Double
        | TokenKind::Void
        | TokenKind::Struct
        | TokenKind::Const
        | TokenKind::Unsigned => parse_var_decl(parser),
        _ => parse_expr_stmt(parser),
    }
}

/// Parseia um bloco: `{ stmt* }`.
fn parse_block(parser: &mut Parser) -> Result<Stmt, CompilerError> {
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

/// Parseia `if (cond) then [else else_branch]`.
fn parse_if(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::If, "'if'")?.clone();

    parser.expect(&TokenKind::LeftParen, "'(' após 'if'")?;
    let cond = parser.parse_expr(0)?;
    parser.expect(&TokenKind::RightParen, "')' após condição do if")?;

    let then_branch = Box::new(parse_stmt(parser)?);

    let else_branch = if parser.match_kind(&TokenKind::Else) {
        Some(Box::new(parse_stmt(parser)?))
    } else {
        None
    };

    let end_span = else_branch
        .as_ref()
        .map(|s| s.span())
        .unwrap_or_else(|| then_branch.span());

    let span = parser.join_span(parser.span_of(&kw), end_span);
    Ok(Stmt::If(cond, then_branch, else_branch, span))
}

/// Parseia `while (cond) body`.
fn parse_while(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::While, "'while'")?.clone();

    parser.expect(&TokenKind::LeftParen, "'(' após 'while'")?;
    let cond = parser.parse_expr(0)?;
    parser.expect(&TokenKind::RightParen, "')' após condição do while")?;

    let body = Box::new(parse_stmt(parser)?);
    let span = parser.join_span(parser.span_of(&kw), body.span());
    Ok(Stmt::While(cond, body, span))
}

/// Parseia `for (init?; cond?; inc?) body`.
fn parse_for(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::For, "'for'")?.clone();
    parser.expect(&TokenKind::LeftParen, "'(' após 'for'")?;

    // Cláusula init: declaração, expressão ou vazia
    let init: Option<Box<Stmt>> = if parser.check(&TokenKind::Semicolon) {
        parser.advance();
        None
    } else if is_type_start(parser) {
        Some(Box::new(parse_var_decl(parser)?))
    } else {
        let expr = parser.parse_expr(0)?;
        let span = expr.span();
        parser.expect(&TokenKind::Semicolon, "';' após inicialização do for")?;
        Some(Box::new(Stmt::ExprStmt(expr, span)))
    };

    // Cláusula cond: expressão ou vazia
    let cond = if parser.check(&TokenKind::Semicolon) {
        parser.advance();
        None
    } else {
        let e = parser.parse_expr(0)?;
        parser.expect(&TokenKind::Semicolon, "';' após condição do for")?;
        Some(e)
    };

    // Cláusula inc: expressão ou vazia
    let inc = if parser.check(&TokenKind::RightParen) {
        None
    } else {
        Some(parser.parse_expr(0)?)
    };

    parser.expect(&TokenKind::RightParen, "')' para fechar cabeçalho do for")?;

    let body = Box::new(parse_stmt(parser)?);
    let span = parser.join_span(parser.span_of(&kw), body.span());
    Ok(Stmt::For(init, cond, inc, body, span))
}

/// Parseia `return [expr];`.
fn parse_return(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::Return, "'return'")?.clone();

    let value = if parser.check(&TokenKind::Semicolon) {
        None
    } else {
        Some(parser.parse_expr(0)?)
    };

    let semi = parser.expect(&TokenKind::Semicolon, "';' após return")?.clone();
    let span = parser.join_span(parser.span_of(&kw), parser.span_of(&semi));
    Ok(Stmt::Return(value, span))
}

/// Parseia `break;`.
fn parse_break(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::Break, "'break'")?.clone();
    let semi = parser.expect(&TokenKind::Semicolon, "';' após break")?.clone();
    let span = parser.join_span(parser.span_of(&kw), parser.span_of(&semi));
    Ok(Stmt::Break(span))
}

/// Parseia `continue;`.
fn parse_continue(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::Continue, "'continue'")?.clone();
    let semi = parser.expect(&TokenKind::Semicolon, "';' após continue")?.clone();
    let span = parser.join_span(parser.span_of(&kw), parser.span_of(&semi));
    Ok(Stmt::Continue(span))
}

/// Parseia `[const] [unsigned] tipo[*] nome [= expr];`.
fn parse_var_decl(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let start_tok = parser.peek().clone();
    let qualifier = parse_cast_type(parser)?;

    let name_tok = parser.advance().clone();
    let TokenKind::Identifier(name) = name_tok.kind else {
        return Err(parser.syntax_error(
            &name_tok,
            "nome da variável",
            &format!("{:?}", name_tok.kind),
        ));
    };

    let init = if parser.match_kind(&TokenKind::Equal) {
        Some(parser.parse_expr(0)?)
    } else {
        None
    };

    let semi = parser
        .expect(&TokenKind::Semicolon, "';' após declaração de variável")?
        .clone();

    let span = parser.join_span(parser.span_of(&start_tok), parser.span_of(&semi));
    Ok(Stmt::VarDecl(qualifier, name, init, span))
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

/// Retorna `true` se o token atual pode iniciar um especificador de tipo.
fn is_type_start(parser: &Parser) -> bool {
    matches!(
        parser.peek_kind(),
        TokenKind::Int
            | TokenKind::Char
            | TokenKind::Float
            | TokenKind::Double
            | TokenKind::Void
            | TokenKind::Struct
            | TokenKind::Const
            | TokenKind::Unsigned
    )
}
