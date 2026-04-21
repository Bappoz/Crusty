//core do parser
//controle de fluxo, orquestrador
// parse_expression(precedence)
//guardar tokens
//controlar índice atual
//consumir tokens
//iniciar parsing
//chamar rules

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    diagnostics: Vec<CompilerError>,
}