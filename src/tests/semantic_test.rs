#[cfg(test)]
mod tests {
    use crate::analyser::SemanticAnalyser;
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

    fn analyse(prog: &Program) -> Vec<crate::common::errors::types::CompilerError> {
        crate::analyser::analyse(prog)
    }

    // ── VarDecl ───────────────────────────────────────────────────────────────

    #[test]
    fn var_decl_registers_symbol() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), Some(int_lit(0)), span()),
            Stmt::ExprStmt(ident("x"), span()),
        ]);
        assert!(analyse(&prog).is_empty());
    }

    #[test]
    fn undeclared_variable_emits_error() {
        let prog = program(vec![Stmt::ExprStmt(ident("x"), span())]);
        let errors = analyse(&prog);
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
        let errors = analyse(&prog);
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
        assert!(analyse(&prog).is_empty());
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
        let errors = analyse(&prog);
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
        assert!(analyse(&prog).is_empty());
    }

    // ── múltiplos erros acumulados ────────────────────────────────────────────

    #[test]
    fn accumulates_multiple_errors() {
        let prog = program(vec![
            Stmt::ExprStmt(ident("a"), span()),
            Stmt::ExprStmt(ident("b"), span()),
        ]);
        let errors = analyse(&prog);
        assert_eq!(errors.len(), 2);
    }

    // ── inferência de tipo básica ─────────────────────────────────────────────

    #[test]
    fn analyse_expr_infers_int_literal_type() {
        let _prog = program(vec![]);
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        let ty = analyser.analyse_expr(&int_lit(42));
        assert!(matches!(ty.ty, crate::common::ast::ast::Type::Int));
    }

    #[test]
    fn analyse_expr_infers_ident_type_from_scope() {
        let prog = program(vec![Stmt::VarDecl(
            qty(Type::Float),
            "f".into(),
            None,
            span(),
        )]);
        let mut analyser = SemanticAnalyser::new();
        analyser.analyse_program(&prog);

        // após analyse_program o escopo global foi fechado;
        // abre escopo manual para simular lookup dentro de função
        analyser.sym.enter_scope();
        analyser
            .sym
            .declare(crate::analyser::symbol_table::Symbol {
                name: "f".into(),
                ty: qty(Type::Float),
                mutable: true,
                decl_span: span(),
            })
            .unwrap();
        let ty = analyser.analyse_expr(&ident("f"));
        assert!(matches!(ty.ty, crate::common::ast::ast::Type::Float));
    }
}
