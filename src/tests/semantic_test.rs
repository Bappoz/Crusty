#[cfg(test)]
mod tests {
    use crate::analyser::SemanticAnalyser;
    use crate::common::ast::ast::{Program, QualifierType, Type};
    use crate::common::ast::decl::Decl;
    use crate::common::ast::expr::{Expr, Literal, MemberAccess};
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

    // ── Binary: inferência de tipo ────────────────────────────────────────────

    #[test]
    fn binary_int_plus_int_returns_int() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        let expr = Expr::Binary(
            Box::new(int_lit(1)),
            crate::common::ast::expr::BinOp::Add,
            Box::new(int_lit(2)),
            span(),
        );
        let ty = analyser.analyse_expr(&expr);
        assert!(analyser.diagnostics.is_empty(), "sem erros esperados");
        assert!(matches!(ty.ty, Type::Int));
    }

    #[test]
    fn binary_int_plus_double_promotes_to_double() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        let expr = Expr::Binary(
            Box::new(int_lit(1)),
            crate::common::ast::expr::BinOp::Add,
            Box::new(Expr::Literal(Literal::Double(3.14), span())),
            span(),
        );
        let ty = analyser.analyse_expr(&expr);
        assert!(analyser.diagnostics.is_empty(), "sem erros esperados");
        assert!(matches!(ty.ty, Type::Double));
    }

    #[test]
    fn binary_int_plus_float_promotes_to_float() {
        let mut analyser = SemanticAnalyser::new();
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
        let expr = Expr::Binary(
            Box::new(int_lit(1)),
            crate::common::ast::expr::BinOp::Add,
            Box::new(ident("f")),
            span(),
        );
        let ty = analyser.analyse_expr(&expr);
        assert!(analyser.diagnostics.is_empty(), "sem erros esperados");
        assert!(matches!(ty.ty, Type::Float));
    }

    #[test]
    fn binary_int_plus_pointer_is_valid_pointer_arithmetic() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        analyser
            .sym
            .declare(crate::analyser::symbol_table::Symbol {
                name: "p".into(),
                ty: qty(Type::Pointer(Box::new(Type::Int))),
                mutable: true,
                decl_span: span(),
            })
            .unwrap();
        let expr = Expr::Binary(
            Box::new(int_lit(1)),
            crate::common::ast::expr::BinOp::Add,
            Box::new(ident("p")),
            span(),
        );
        let ty = analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.is_empty(),
            "aritmetica de ponteiro deve ser valida"
        );
        assert!(matches!(ty.ty, Type::Pointer(_)));
    }

    #[test]
    fn binary_int_mul_string_emits_type_mismatch() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        // string literal tem tipo Pointer(Char) — não é numérico
        let expr = Expr::Binary(
            Box::new(int_lit(1)),
            crate::common::ast::expr::BinOp::Mul,
            Box::new(Expr::Literal(Literal::String("hello".into()), span())),
            span(),
        );
        analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::TypeMismatch { .. })
            )),
            "int * string deve emitir TypeMismatch"
        );
    }

    #[test]
    fn binary_float_mod_float_emits_type_mismatch() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        let expr = Expr::Binary(
            Box::new(Expr::Literal(Literal::Double(1.0), span())),
            crate::common::ast::expr::BinOp::Mod,
            Box::new(Expr::Literal(Literal::Double(2.0), span())),
            span(),
        );
        analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::TypeMismatch { .. })
            )),
            "double % double deve emitir TypeMismatch (% exige inteiros)"
        );
    }

    #[test]
    fn binary_relational_int_vs_int_returns_int() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        let expr = Expr::Binary(
            Box::new(int_lit(1)),
            crate::common::ast::expr::BinOp::Less,
            Box::new(int_lit(2)),
            span(),
        );
        let ty = analyser.analyse_expr(&expr);
        assert!(analyser.diagnostics.is_empty());
        assert!(matches!(ty.ty, Type::Int));
    }

    // ── Assign: verificação de compatibilidade de tipo ────────────────────────

    #[test]
    fn assign_int_to_int_is_ok() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
            Stmt::ExprStmt(
                Expr::Assign(Box::new(ident("x")), Box::new(int_lit(42)), span()),
                span(),
            ),
        ]);
        assert!(analyse(&prog).is_empty(), "int = int deve ser valido");
    }

    #[test]
    fn assign_float_to_int_implicit_coercion_is_ok() {
        // Em C, int = double é válido com coerção implícita
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
            Stmt::ExprStmt(
                Expr::Assign(
                    Box::new(ident("x")),
                    Box::new(Expr::Literal(Literal::Double(3.14), span())),
                    span(),
                ),
                span(),
            ),
        ]);
        assert!(
            analyse(&prog).is_empty(),
            "int = double deve ser valido (coercao implicita)"
        );
    }

    #[test]
    fn assign_string_to_int_emits_type_mismatch() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
            Stmt::ExprStmt(
                Expr::Assign(
                    Box::new(ident("x")),
                    Box::new(Expr::Literal(Literal::String("hello".into()), span())),
                    span(),
                ),
                span(),
            ),
        ]);
        let errors = analyse(&prog);
        assert!(
            errors.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::TypeMismatch { .. })
            )),
            "int = string deve emitir TypeMismatch"
        );
    }

    #[test]
    fn assign_int_to_pointer_emits_type_mismatch() {
        let prog = program(vec![
            Stmt::VarDecl(
                qty(Type::Pointer(Box::new(Type::Int))),
                "p".into(),
                None,
                span(),
            ),
            Stmt::ExprStmt(
                Expr::Assign(Box::new(ident("p")), Box::new(int_lit(42)), span()),
                span(),
            ),
        ]);
        let errors = analyse(&prog);
        assert!(
            errors.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::TypeMismatch { .. })
            )),
            "int* = int deve emitir TypeMismatch"
        );
    }

    // ── typedef + member access (teste existente) ─────────────────────────────

    #[test]
    fn typedef_alias_resolves_before_member_access() {

        let prog = Program {
            decls: vec![
                Decl::StructDecl("Point".into(), vec![(qty(Type::Int), "x".into())], span()),
                Decl::Typedef(
                    qty(Type::Struct("Point".into())),
                    "PointAlias".into(),
                    span(),
                ),
                Decl::Function(
                    qty(Type::Void),
                    "main".into(),
                    vec![],
                    vec![
                        Stmt::VarDecl(
                            qty(Type::Alias("PointAlias".into())),
                            "p".into(),
                            None,
                            span(),
                        ),
                        Stmt::ExprStmt(
                            Expr::Member(
                                Box::new(ident("p")),
                                MemberAccess::Direct,
                                "x".into(),
                                span(),
                            ),
                            span(),
                        ),
                    ],
                    span(),
                ),
            ],
        };

        assert!(analyse(&prog).is_empty());
    }
}
