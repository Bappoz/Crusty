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

    #[test]
    fn global_initializer_type_mismatch_emits_error() {
        let prog = Program {
            decls: vec![Decl::GlobalVar(
                qty(Type::Int),
                "x".into(),
                Some(Expr::Literal(Literal::String("hello".into()), span())),
                span(),
            )],
        };

        let errors = analyse(&prog);
        assert!(errors.iter().any(|e| matches!(
            e,
            crate::common::errors::types::CompilerError::Semantic(se)
                if matches!(&se.kind, SemanticErrorKind::TypeMismatch { .. })
        )));
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
                params: None,
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
                params: None,
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
                params: None,
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

    #[test]
    fn function_call_uses_registered_return_type() {
        let prog = Program {
            decls: vec![Decl::Function(
                qty(Type::Int),
                "main".into(),
                vec![],
                vec![Stmt::Return(
                    Some(Expr::Call(Box::new(ident("main")), vec![], span())),
                    span(),
                )],
                span(),
            )],
        };

        assert!(analyse(&prog).is_empty());
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

    // ── Call checks ───────────────────────────────────────────────────────

    #[test]
    fn call_correct_is_ok() {
        let prog = Program {
            decls: vec![
                Decl::Function(
                    qty(Type::Int),
                    "f".into(),
                    vec![(qty(Type::Int), "a".into())],
                    vec![],
                    span(),
                ),
                Decl::Function(
                    qty(Type::Void),
                    "main".into(),
                    vec![],
                    vec![Stmt::ExprStmt(
                        Expr::Call(
                            Box::new(Expr::Ident("f".into(), span())),
                            vec![int_lit(1)],
                            span(),
                        ),
                        span(),
                    )],
                    span(),
                ),
            ],
        };
        assert!(analyse(&prog).is_empty());
    }

    #[test]
    fn call_arity_mismatch_emits_error() {
        let prog = Program {
            decls: vec![
                Decl::Function(
                    qty(Type::Int),
                    "f".into(),
                    vec![(qty(Type::Int), "a".into())],
                    vec![],
                    span(),
                ),
                Decl::Function(
                    qty(Type::Void),
                    "main".into(),
                    vec![],
                    vec![Stmt::ExprStmt(
                        Expr::Call(Box::new(Expr::Ident("f".into(), span())), vec![], span()),
                        span(),
                    )],
                    span(),
                ),
            ],
        };
        let errors = analyse(&prog);
        assert!(errors.iter().any(|e| matches!(
            e,
            crate::common::errors::types::CompilerError::Semantic(se)
                if matches!(&se.kind, crate::common::errors::types::SemanticErrorKind::ArityMismatch { .. })
        )));
    }

    #[test]
    fn call_arg_type_mismatch_emits_error() {
        let prog = Program {
            decls: vec![
                Decl::Function(
                    qty(Type::Int),
                    "f".into(),
                    vec![(qty(Type::Int), "a".into())],
                    vec![],
                    span(),
                ),
                Decl::Function(
                    qty(Type::Void),
                    "main".into(),
                    vec![],
                    vec![Stmt::ExprStmt(
                        Expr::Call(
                            Box::new(Expr::Ident("f".into(), span())),
                            vec![Expr::Literal(Literal::String("hi".into()), span())],
                            span(),
                        ),
                        span(),
                    )],
                    span(),
                ),
            ],
        };
        let errors = analyse(&prog);
        assert!(errors.iter().any(|e| matches!(
            e,
            crate::common::errors::types::CompilerError::Semantic(se)
                if matches!(&se.kind, crate::common::errors::types::SemanticErrorKind::TypeMismatch { .. })
        )));
    }

    // ── Expr::Index ───────────────────────────────────────────────────────────

    fn declare_array(analyser: &mut SemanticAnalyser, name: &str, inner: Type) {
        analyser
            .sym
            .declare(crate::analyser::symbol_table::Symbol {
                name: name.into(),
                ty: qty(Type::Array(Box::new(inner))),
                mutable: true,
                params: None,
                decl_span: span(),
            })
            .unwrap();
    }

    fn declare_var(analyser: &mut SemanticAnalyser, name: &str, ty: Type) {
        analyser
            .sym
            .declare(crate::analyser::symbol_table::Symbol {
                name: name.into(),
                ty: qty(ty),
                mutable: true,
                params: None,
                decl_span: span(),
            })
            .unwrap();
    }

    #[test]
    fn index_array_with_int_is_ok() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        declare_array(&mut analyser, "arr", Type::Int);
        let expr = Expr::Index(Box::new(ident("arr")), Box::new(int_lit(0)), span());
        let ty = analyser.analyse_expr(&expr);
        assert!(analyser.diagnostics.is_empty(), "arr[0] deve ser válido");
        assert!(matches!(ty.ty, Type::Int));
    }

    #[test]
    fn index_array_with_float_emits_invalid_index_type() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        declare_array(&mut analyser, "arr", Type::Int);
        let expr = Expr::Index(
            Box::new(ident("arr")),
            Box::new(Expr::Literal(Literal::Double(1.5), span())),
            span(),
        );
        analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::InvalidIndexType { .. })
            )),
            "arr[float] deve emitir InvalidIndexType"
        );
    }

    #[test]
    fn index_non_array_emits_not_indexable() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        declare_var(&mut analyser, "x", Type::Int);
        let expr = Expr::Index(Box::new(ident("x")), Box::new(int_lit(0)), span());
        analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::NotIndexable { .. })
            )),
            "x[0] onde x é int deve emitir NotIndexable"
        );
    }

    #[test]
    fn index_char_pointer_returns_char() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        analyser
            .sym
            .declare(crate::analyser::symbol_table::Symbol {
                name: "s".into(),
                ty: qty(Type::Pointer(Box::new(Type::Char))),
                mutable: true,
                params: None,
                decl_span: span(),
            })
            .unwrap();
        let expr = Expr::Index(Box::new(ident("s")), Box::new(int_lit(0)), span());
        let ty = analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.is_empty(),
            "s[0] onde s é char* deve ser válido"
        );
        assert!(matches!(ty.ty, Type::Char));
    }

    #[test]
    fn calling_variable_as_function_emits_error() {
        let prog = Program {
            decls: vec![Decl::Function(
                qty(Type::Void),
                "main".into(),
                vec![],
                vec![
                    Stmt::VarDecl(qty(Type::Int), "x".into(), Some(int_lit(1)), span()),
                    Stmt::ExprStmt(
                        Expr::Call(Box::new(Expr::Ident("x".into(), span())), vec![], span()),
                        span(),
                    ),
                ],
                span(),
            )],
        };
        let errors = analyse(&prog);
        assert!(errors.iter().any(|e| matches!(
            e,
            crate::common::errors::types::CompilerError::Semantic(se)
                if matches!(&se.kind, crate::common::errors::types::SemanticErrorKind::CallNonFunction(_))
        )));
    }

    // ── Expr::Ternary ─────────────────────────────────────────────────────────

    #[test]
    fn ternary_same_type_returns_int() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        // 1 ? 10 : 20  → int, sem erros
        let expr = Expr::Ternary(
            Box::new(int_lit(1)),
            Box::new(int_lit(10)),
            Box::new(int_lit(20)),
            span(),
        );
        let ty = analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.is_empty(),
            "ternário int:int deve ser válido"
        );
        assert!(matches!(ty.ty, Type::Int));
    }

    #[test]
    fn ternary_numeric_promotion_returns_double() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        // 1 ? 1.0 : 2  → double, sem erros
        let expr = Expr::Ternary(
            Box::new(int_lit(1)),
            Box::new(Expr::Literal(Literal::Double(1.0), span())),
            Box::new(int_lit(2)),
            span(),
        );
        let ty = analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.is_empty(),
            "ternário double:int deve ser válido (promoção numérica)"
        );
        assert!(matches!(ty.ty, Type::Double));
    }

    #[test]
    fn ternary_incompatible_branches_emits_type_mismatch() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        // 1 ? 10 : "str"  → TypeMismatch (int vs char*)
        let expr = Expr::Ternary(
            Box::new(int_lit(1)),
            Box::new(int_lit(10)),
            Box::new(Expr::Literal(Literal::String("str".into()), span())),
            span(),
        );
        analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::TypeMismatch { .. })
            )),
            "ternário int:char* deve emitir TypeMismatch"
        );
    }

    #[test]
    fn ternary_non_scalar_condition_emits_error() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        // Declara variável do tipo struct
        analyser
            .sym
            .declare(crate::analyser::symbol_table::Symbol {
                name: "s".into(),
                ty: qty(Type::Struct("S".into())),
                mutable: true,
                params: None,
                decl_span: span(),
            })
            .unwrap();
        // s ? 1 : 2  → erro: struct não é escalar
        let expr = Expr::Ternary(
            Box::new(ident("s")),
            Box::new(int_lit(1)),
            Box::new(int_lit(2)),
            span(),
        );
        analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::TypeMismatch { .. })
            )),
            "condição struct no ternário deve emitir TypeMismatch"
        );
    }

    #[test]
    fn ternary_pointer_int_without_constant_folding_emits_error() {
        // Sem análise de valor constante (constant folding), o analisador
        // não distingue `0` (null pointer) de `10` (inteiro arbitrário).
        // Por isso, `char* : int` é sempre rejeitado neste nível semântico.
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        let expr = Expr::Ternary(
            Box::new(int_lit(1)),
            Box::new(Expr::Literal(Literal::String("a".into()), span())),
            Box::new(int_lit(0)),
            span(),
        );
        analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.iter().any(|e| matches!(
                e,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(&se.kind, SemanticErrorKind::TypeMismatch { .. })
            )),
            "char*:int deve emitir TypeMismatch sem constant folding"
        );
    }
}
