#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use crate::common::ast::ast::{Program, QualifierType, Type};
    use crate::common::ast::decl::Decl;
    use crate::common::ast::expr::{Expr, Literal};
    use crate::common::ast::stmt::Stmt;
    use crate::common::errors::error_data::Span;
    use crate::common::errors::types::SemanticErrorKind;

    fn span() -> Span {
        Span {
            line: 1,
            end_line: 1,
            column_start: 1,
            column_end: 2,
        }
    }

    fn qty(ty: Type) -> QualifierType {
        QualifierType {
            ty,
            is_const: false,
            is_unsigned: false,
        }
    }

    fn qty_const(ty: Type) -> QualifierType {
        QualifierType {
            ty,
            is_const: true,
            is_unsigned: false,
        }
    }

    fn int_lit(v: i64) -> Expr {
        Expr::Literal(Literal::Int(v), span())
    }

    fn ident(name: &str) -> Expr {
        Expr::Ident(name.to_string(), span())
    }

    fn program(stmts: Vec<Stmt>) -> Program {
        Program {
            decls: vec![Decl::Function(
                qty(Type::Void),
                "main".into(),
                vec![],
                stmts,
                span(),
            )],
        }
    }

    // ── VarDecl ───────────────────────────────────────────────────────────────

    #[test]
    fn var_decl_registers_symbol() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), Some(int_lit(0)), span()),
            Stmt::ExprStmt(ident("x"), span()),
        ]);
        assert!(Analyser::new().analyse_program(&prog).is_ok());
    }

    #[test]
    fn undeclared_variable_emits_error() {
        let prog = program(vec![Stmt::ExprStmt(ident("x"), span())]);
        let errors = Analyser::new().analyse_program(&prog).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            crate::common::errors::types::CompilerError::Semantic(e)
                if matches!(e.kind, SemanticErrorKind::UndefinedVariable(_))
        ));
    }

    #[test]
    fn redeclaration_same_scope_emits_error() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
        ]);
        let errors = Analyser::new().analyse_program(&prog).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            crate::common::errors::types::CompilerError::Semantic(se)
                if matches!(se.kind, SemanticErrorKind::Redeclaration(_))
        )));
    }

    #[test]
    fn redeclaration_in_inner_scope_is_ok() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
            Stmt::Block(
                vec![Stmt::VarDecl(qty(Type::Float), "x".into(), None, span())],
                span(),
            ),
        ]);
        assert!(Analyser::new().analyse_program(&prog).is_ok());
    }

    // ── AssignToConst ─────────────────────────────────────────────────────────

    #[test]
    fn assign_to_const_emits_error() {
        let prog = program(vec![
            Stmt::VarDecl(qty_const(Type::Int), "PI".into(), Some(int_lit(3)), span()),
            Stmt::ExprStmt(
                Expr::Assign(Box::new(ident("PI")), Box::new(int_lit(4)), span()),
                span(),
            ),
        ]);
        let errors = Analyser::new().analyse_program(&prog).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e,
            crate::common::errors::types::CompilerError::Semantic(se)
                if matches!(&se.kind, SemanticErrorKind::AssignToConst(n) if n == "PI")
        )));
    }

    #[test]
    fn assign_to_mutable_is_ok() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), Some(int_lit(0)), span()),
            Stmt::ExprStmt(
                Expr::Assign(Box::new(ident("x")), Box::new(int_lit(1)), span()),
                span(),
            ),
        ]);
        assert!(Analyser::new().analyse_program(&prog).is_ok());
    }

    // ── múltiplos erros acumulados ────────────────────────────────────────────

    #[test]
    fn accumulates_multiple_errors() {
        // dois identificadores não declarados no mesmo programa
        let prog = program(vec![
            Stmt::ExprStmt(ident("a"), span()),
            Stmt::ExprStmt(ident("b"), span()),
        ]);
        let errors = Analyser::new().analyse_program(&prog).unwrap_err();
        assert_eq!(errors.len(), 2);
    }
}
