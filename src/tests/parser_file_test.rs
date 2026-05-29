#[cfg(test)]
mod tests {
    use crate::common::ast::ast::Program;
    use crate::common::ast::decl::Decl;
    use crate::common::ast::expr::{Expr, Literal};
    use crate::common::ast::stmt::Stmt;
    use crate::common::input::source::SourceFile;
    use crate::lexer::scanner::Scanner;
    use crate::parser::Parser;
    use std::fs;
    use std::panic;
    use std::path::PathBuf;
    use tempfile::tempdir;

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
            "esperado zero erros lexicais em {name}, obtido {}",
            scanner.diagnostics.len()
        );

        let mut parser = Parser::new(scanner.tokens);
        parser
            .parse_program()
            .unwrap_or_else(|e| panic!("falha ao parsear '{}': {:?}", name, e))
    }

    #[test]
    fn full_code1_has_one_main_function_decl() {
        let program = parse_file("full_code1.c");

        assert_eq!(program.decls.len(), 1, "esperado 1 declaração global");
        assert!(matches!(
            &program.decls[0],
            Decl::Function(_, name, _, _, _) if name == "main"
        ));
    }

    #[test]
    fn declarations_has_five_global_vars() {
        let program = parse_file("declarations.c");

        assert_eq!(program.decls.len(), 5, "esperado 5 declarações globais");
        assert!(program
            .decls
            .iter()
            .all(|d| matches!(d, Decl::GlobalVar(_, _, _, _))));
    }

    #[test]
    fn hello_world_has_printf_call_and_return() {
        let program = parse_file("hello_world.c");

        assert_eq!(program.decls.len(), 1, "esperado programa com 1 função");
        let Decl::Function(_, name, _, body, _) = &program.decls[0] else {
            panic!("esperava Decl::Function");
        };
        assert_eq!(name, "main");

        let has_printf_call = body.iter().any(|stmt| {
            matches!(
                stmt,
                Stmt::ExprStmt(
                    Expr::Call(callee, args, _),
                    _
                ) if matches!(callee.as_ref(), Expr::Ident(id, _) if id == "printf")
                    && matches!(args.as_slice(), [Expr::Literal(Literal::String(_), _)])
            )
        });
        assert!(has_printf_call, "esperava chamada printf no corpo de main");

        let has_return = body.iter().any(|stmt| matches!(stmt, Stmt::Return(Some(_), _)));
        assert!(has_return, "esperava return com valor no corpo de main");
    }

    #[test]
    fn syntax_error_file_reports_diagnostic_without_panic() {
        let result = panic::catch_unwind(|| {
            let tmp = tempdir().expect("falha ao criar diretório temporário");
            let file_path = tmp.path().join("syntax_error.c");

            // Falta '}' de fechamento para forçar erro sintático no parser.
            fs::write(&file_path, "int main() {\n    return 0;\n")
                .expect("falha ao gravar arquivo temporário");

            let src = SourceFile::from_path(file_path)
                .expect("falha ao abrir arquivo temporário via SourceFile");
            let mut scanner = Scanner::new(src);
            scanner.scan();

            let mut parser = Parser::new(scanner.tokens);
            let parse_result = parser.parse_program();

            let diagnostics_count = scanner.diagnostics.len() + usize::from(parse_result.is_err());
            assert!(
                diagnostics_count >= 1,
                "esperado ao menos 1 diagnóstico, obtido {diagnostics_count}"
            );
        });

        assert!(result.is_ok(), "pipeline não deve panicar em erro sintático");
    }
}