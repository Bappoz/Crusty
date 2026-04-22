use crate::common::ast::stmt::Stmt;
use crate::parser::parser::Parser;

// TODO(parser): statements não fazem parte desta entrega expression-only.
// Este stub mantém o módulo compilando até a implementação da fase de statements.
pub fn parse_statement(parser: &mut Parser) -> Option<Stmt> {
	let _ = parser;
	None
}
