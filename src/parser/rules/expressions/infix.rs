use crate::common::ast::expr::Expr;
use crate::parser::parser::Parser;
use crate::parser::precedence::Precedence;

// TODO(parser): reativar integração de led quando o parser completo (nud/led separados) voltar.
// No modo atual (expression-only), o loop infixo está dentro de parse_expr.
pub fn led(parser: &mut Parser, left: Expr, infix_precedence: Precedence) -> Option<Expr> {
	let _ = (parser, left, infix_precedence);
	None
}