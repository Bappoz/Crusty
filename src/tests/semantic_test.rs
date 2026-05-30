#[cfg(test)]
mod tests {
    use crate::analyser::semantic::analyse;
    use crate::common::ast::ast::Program;
    use crate::common::input::source::SourceFile;
    use crate::lexer::scanner::Scanner;
    use crate::parser::Parser;
    use std::path::PathBuf;

    fn parse_file(name: &str) -> Program {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/examples")
            .join(name);
        let src = SourceFile::from_path(path)
            .unwrap_or_else(|e| panic!("failed to read '{}': {:?}", name, e.to_report()));

        let mut scanner = Scanner::new(src);
        scanner.scan();
        assert!(
            scanner.diagnostics.is_empty(),
            "esperado zero erros lexicais em {name}"
        );

        let mut parser = Parser::new(scanner.tokens);
        parser
            .parse_program()
            .unwrap_or_else(|e| panic!("falha ao parsear '{}': {:?}", name, e))
    }

    #[test]
    fn full_code1_has_no_semantic_diagnostics() {
        let program = parse_file("full_code1.c");
        let diagnostics = analyse(&program);
        assert!(
            diagnostics.is_empty(),
            "esperava zero diagnósticos semânticos"
        );
    }

    #[test]
    fn semantic_error_file_reports_at_least_one_diagnostic() {
        let program = parse_file("semantic_error.c");
        let diagnostics = analyse(&program);
        assert!(
            !diagnostics.is_empty(),
            "esperava ao menos 1 diagnóstico semântico"
        );
    }
}
