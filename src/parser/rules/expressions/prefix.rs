use crate::common::ast::expr::Expr;
use crate::parser::parser::Parser;

// Encapsula a regra prefixa no parser de expressões isolado.
// TODO(parser): quando houver API pública dedicada para nud, redirecionar para ela.
pub fn nud(parser: &mut Parser) -> Option<Expr> {
	parser.parse_expr(0).ok()
}
