#[cfg(test)]
mod tests {

    use crate::common::errors::error_data::Span;
    use crate::common::errors::report::ToReport;
    use crate::common::errors::types::{
        CodegenError, CompilerError, IntermediateError, LexicalError, OptimizationError,
        SemanticError, SemanticErrorKind, SyntaxError,
    };

    // =======================
    // LEXICAL ERROR
    // =======================
    #[test]
    fn test_lexical_error() {
        let error = LexicalError {
            span: Span {
                line: 1,
                column_start: 5,
                column_end: 6,
            },
            invalid_char: '@',
        };

        let report = error.to_report();

        assert_eq!(report.message, "invalid character");
        assert!(report.help.is_some());
        assert_eq!(report.labels.len(), 1);
        assert!(report.labels[0].message.contains("@"));
    }

    // =======================
    // SYNTAX ERROR
    // =======================
    #[test]
    fn test_syntax_error() {
        let error = SyntaxError {
            span: Span {
                line: 2,
                column_start: 3,
                column_end: 4,
            },
            expected: ";".into(),
            found: "}".into(),
        };

        let report = error.to_report();

        assert_eq!(report.message, "syntax error");
        assert!(report.labels[0].message.contains("esperado ';'"));
        assert!(report.help.unwrap().contains(";"));
    }

    // =======================
    // SEMANTIC ERROR
    // =======================
    #[test]
    fn test_semantic_undefined_variable() {
        let error = SemanticError {
            span: Span {
                line: 3,
                column_start: 1,
                column_end: 2,
            },
            kind: SemanticErrorKind::UndefinedVariable("x".into()),
        };

        let report = error.to_report();

        assert_eq!(report.message, "variable not defined");
        assert!(report.labels[0].message.contains("x"));
        assert!(report.help.unwrap().contains("declare"));
    }

    #[test]
    fn test_semantic_type_mismatch() {
        let error = SemanticError {
            span: Span {
                line: 4,
                column_start: 2,
                column_end: 5,
            },
            kind: SemanticErrorKind::TypeMismatch {
                expected: "int".into(),
                found: "string".into(),
            },
        };

        let report = error.to_report();

        assert_eq!(report.message, "type error");
        assert!(report.labels[0].message.contains("int"));
        assert!(report.labels[0].message.contains("string"));
    }

    // =======================
    // INTERMEDIATE ERROR
    // =======================
    #[test]
    fn test_intermediate_error() {
        let error = IntermediateError {
            message: "operando ausente".into(),
            instruction: None,
        };

        let report = error.to_report();

        assert!(report.message.contains("IR error"));
    }

    // =======================
    // OPTIMIZATION ERROR
    // =======================
    #[test]
    fn test_optimization_error() {
        let error = OptimizationError {
            pass: "Constant Folding".into(),
            message: "divisão por zero".into(),
        };

        let report = error.to_report();

        assert!(report.message.contains("Constant Folding"));
        assert!(report.help.unwrap().contains("divisão"));
    }

    // =======================
    // CODEGEN ERROR
    // =======================
    #[test]
    fn test_codegen_error() {
        let error = CodegenError {
            message: "invalid register".into(),
            instruction: Some("MOV R1, R2".into()),
        };

        let report = error.to_report();

        assert!(report.message.contains("code generation"));
        assert!(report.help.unwrap().contains("register"));
    }

    // =======================
    // COMPILER ERROR (enum)
    // =======================
    #[test]
    fn test_compiler_error_enum() {
        let error = CompilerError::Syntax(SyntaxError {
            span: Span {
                line: 1,
                column_start: 0,
                column_end: 1,
            },
            expected: ";".into(),
            found: "EOF".into(),
        });

        let report = error.to_report();

        assert_eq!(report.message, "syntax error");
    }
}
