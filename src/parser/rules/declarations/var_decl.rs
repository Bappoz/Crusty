use crate::common::ast::stmt::Stmt;
use crate::common::errors::types::CompilerError;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::parser::Parser;
use crate::parser::rules::declarations::types::parse_type;

/// Parseia uma declaração de variável local: `tipo ident (= expr)? ;`
/// Pressupõe que o token atual já foi confirmado como keyword de tipo.
pub fn parse_var_decl(parser: &mut Parser) -> Result<Stmt, CompilerError> {
    let start = parser.peek().clone();

    let qty = parse_type(parser)?;

    // Nome da variável
    let name_token = parser.advance().clone();
    let TokenKind::Identifier(name) = name_token.kind.clone() else {
        return Err(parser.syntax_error(
            &name_token,
            "identificador",
            &format!("{:?}", name_token.kind),
        ));
    };

    // Inicializador Opcional: = expr
    let init = if parser.match_kind(&TokenKind::Equal) {
        Some(parser.parse_expr(0)?)
    } else {
        None
    };

    let semi = parser
        .expect(&TokenKind::Semicolon, "';' ao fim da declaração")?
        .clone();
    let span = parser.join_span(parser.span_of(&start), parser.span_of(&semi));

    Ok(Stmt::VarDecl(qty, name, init, span))
}
