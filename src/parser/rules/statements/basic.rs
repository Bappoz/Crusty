use crate::common::ast::stmt::Stmt;
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;
use crate::parser::rules::declarations;

/// Dispatcher principal: decide qual parser de statement usar com base no token atual.
///
/// Reconhece:
///   `{` → bloco, `return` → return, `break` → break, `continue` → continue,
///   `if` → if, `while` → while, `do` → do-while,
///   keywords de tipo → declaração de variável,
///   qualquer outra coisa → statement de expressão.
pub fn parse_stmt(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    match parser.peek_kind() {
        TokenKind::LeftBrace => parse_block(parser),
        TokenKind::Return => parse_return(parser),
        TokenKind::Break => parse_break(parser),
        TokenKind::Continue => parse_continue(parser),
        TokenKind::If => parse_if(parser),
        TokenKind::While => parse_while(parser),
        TokenKind::Do => parse_do_while(parser),
        TokenKind::For => {
            let tok = parser.peek().clone();
            Err(parser.syntax_error(&tok, "statement", "'for' ainda não implementado"))
        }
        _ if declarations::starts_type(parser.peek_kind()) => declarations::parse_var_decl(parser),
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

/// Parseia `if (expr) stmt [else stmt]`.
/// Suporta `else if` via recursão natural (o `else` chama `parse_stmt` novamente).
fn parse_if(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::If, "'if'")?.clone();

    parser.expect(&TokenKind::LeftParen, "'(' após if")?;
    let cond = parser.parse_expr(0)?;
    parser.expect(&TokenKind::RightParen, "')' após condição do if")?;

    let then_branch = parse_stmt(parser)?;

    let else_branch = if parser.match_kind(&TokenKind::Else) {
        Some(Box::new(parse_stmt(parser)?))
    } else {
        None
    };

    let end = else_branch
        .as_ref()
        .map(|e| e.span())
        .unwrap_or_else(|| then_branch.span());
    let span = parser.join_span(parser.span_of(&kw), end);
    Ok(Stmt::If(cond, Box::new(then_branch), else_branch, span))
}

/// Parseia `while (expr) stmt`.
fn parse_while(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::While, "'while'")?.clone();

    parser.expect(&TokenKind::LeftParen, "'(' após while")?;
    let cond = parser.parse_expr(0)?;
    parser.expect(&TokenKind::RightParen, "')' após condição do while")?;

    let body = parse_stmt(parser)?;

    let span = parser.join_span(parser.span_of(&kw), body.span());
    Ok(Stmt::While(cond, Box::new(body), span))
}

/// Parseia `do stmt while (expr);`.
fn parse_do_while(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let kw = parser.expect(&TokenKind::Do, "'do'")?.clone();

    let body = parse_stmt(parser)?;

    parser.expect(&TokenKind::While, "'while' após corpo do do")?;
    parser.expect(&TokenKind::LeftParen, "'(' após while do do-while")?;
    let cond = parser.parse_expr(0)?;
    parser.expect(&TokenKind::RightParen, "')' após condição do do-while")?;
    let semi = parser
        .expect(&TokenKind::Semicolon, "';' após do-while")?
        .clone();

    let span = parser.join_span(parser.span_of(&kw), parser.span_of(&semi));
    Ok(Stmt::DoWhile(cond, Box::new(body), span))
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
