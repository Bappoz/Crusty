#[cfg(test)]
mod tests {
    use crate::analyser::SemanticAnalyser;
    use crate::common::ast::ast::{Program, QualifierType, Type};
    use crate::common::ast::decl::Decl;
    use crate::common::ast::expr::{Expr, Literal, MemberAccess};
    use crate::common::ast::stmt::Stmt;
    use crate::common::builtins::BuiltinsLibs;
    use crate::common::errors::error_data::Span;
    use crate::common::errors::types::{
        CompilerError, Diagnostic, SemanticErrorKind, SemanticWarningKind,
    };

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

    fn call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::Call(Box::new(ident(name)), args, span())
    }

    fn program(stmts: Vec<Stmt>) -> Program {
        Program {
            decls: vec![Decl::Function(
                qty(Type::Int),
                "main".into(),
                vec![],
                stmts,
                span(),
            )],
        }
    }

    fn analyse(prog: &Program) -> Vec<Diagnostic> {
        crate::analyser::analyse(prog)
    }

    /// Extrai apenas os erros (ignora avisos) de um conjunto de diagnósticos.
    fn errors(diags: &[Diagnostic]) -> Vec<&CompilerError> {
        diags
            .iter()
            .filter_map(|d| match d {
                Diagnostic::Error(e) => Some(e),
                _ => None,
            })
            .collect()
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
        let diags = analyse(&prog);
        assert_eq!(diags.len(), 1);
        assert!(matches!(
            &diags[0],
            Diagnostic::Error(CompilerError::Semantic(e))
                if matches!(e.kind, SemanticErrorKind::UndefinedVariable(_))
        ));
    }

    #[test]
    fn redeclaration_same_scope_emits_error() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
        ]);
        let diags = analyse(&prog);
        let errs = errors(&diags);
        assert!(errs.iter().any(|e| matches!(
            e,
            CompilerError::Semantic(se)
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
        // sem erros (apenas warnings de unused-variable, que são esperados)
        assert!(errors(&analyse(&prog)).is_empty());
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
        let diags = analyse(&prog);
        let errs = errors(&diags);
        assert!(errs.iter().any(|e| matches!(
            e,
            CompilerError::Semantic(se)
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

        let diags = analyse(&prog);
        assert!(diags.iter().any(|e| matches!(
            e,
            Diagnostic::Error(crate::common::errors::types::CompilerError::Semantic(se))
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
        let diags = analyse(&prog);
        assert_eq!(diags.len(), 2);
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
                prototype_only: false,
                used: false,
                initialized: true,
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
            Box::new(Expr::Literal(Literal::Double(1.5), span())),
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
                prototype_only: false,
                used: false,
                initialized: true,
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
                prototype_only: false,
                used: false,
                initialized: true,
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

    #[test]
    fn printf_without_stdio_header_emits_missing_header_error() {
        let prog = program(vec![Stmt::ExprStmt(
            call(
                "printf",
                vec![
                    Expr::Literal(Literal::String("%d".into()), span()),
                    int_lit(1),
                ],
            ),
            span(),
        )]);

        let mut analyser = SemanticAnalyser::with_builtins(BuiltinsLibs::default());
        analyser.analyse_program(&prog);

        assert!(analyser.diagnostics.iter().any(|e| matches!(
            e,
            CompilerError::Semantic(se)
                if matches!(&se.kind, SemanticErrorKind::MissingLibraryHeader { header, symbol }
                    if header == "stdio.h" && symbol == "printf")
        )));
    }

    #[test]
    fn printf_with_stdio_header_accepts_basic_call() {
        let prog = program(vec![Stmt::ExprStmt(
            call(
                "printf",
                vec![
                    Expr::Literal(Literal::String("%d".into()), span()),
                    int_lit(1),
                ],
            ),
            span(),
        )]);

        let mut analyser = SemanticAnalyser::with_builtins(BuiltinsLibs {
            stdbool: false,
            stdio: true,
        });
        analyser.analyse_program(&prog);

        assert!(
            analyser.diagnostics.is_empty(),
            "printf com stdio.h deve ser aceito"
        );
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
                    Box::new(Expr::Literal(Literal::Double(1.5), span())),
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
        let diags = analyse(&prog);
        let errs = errors(&diags);
        assert!(
            errs.iter().any(|e| matches!(
                e,
                CompilerError::Semantic(se)
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
        let diags = analyse(&prog);
        let errs = errors(&diags);
        assert!(
            errs.iter().any(|e| matches!(
                e,
                CompilerError::Semantic(se)
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

        // sem erros (p é lido via member access; apenas pode haver warnings)
        assert!(errors(&analyse(&prog)).is_empty());
    }

    // ── Warnings: unused-variable ─────────────────────────────────────────────

    #[test]
    fn unused_variable_emits_warning() {
        // int x = 5; return 0;   -> x declarada mas nunca lida
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), Some(int_lit(5)), span()),
            Stmt::Return(Some(int_lit(0)), span()),
        ]);
        let diags = analyse(&prog);
        assert!(errors(&diags).is_empty(), "não deve haver erros");
        assert!(
            diags.iter().any(|d| matches!(
                d,
                Diagnostic::Warning(crate::common::errors::types::CompilerWarning::Semantic(w))
                    if matches!(&w.kind, SemanticWarningKind::UnusedVariable(n) if n == "x")
            )),
            "deve emitir warning de unused-variable para 'x'"
        );
    }

    #[test]
    fn used_variable_emits_no_warning() {
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), Some(int_lit(5)), span()),
            Stmt::Return(Some(ident("x")), span()),
        ]);
        let diags = analyse(&prog);
        assert!(
            diags.is_empty(),
            "variável usada não deve gerar nenhum diagnóstico"
        );
    }

    #[test]
    fn assignment_counts_as_use_no_unused_warning() {
        // int x = 0; x = 1;  -> x é referenciada (atribuição), sem warning de unused
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), Some(int_lit(0)), span()),
            Stmt::ExprStmt(
                Expr::Assign(Box::new(ident("x")), Box::new(int_lit(1)), span()),
                span(),
            ),
        ]);
        let diags = analyse(&prog);
        assert!(
            !diags.iter().any(|d| matches!(
                d,
                Diagnostic::Warning(crate::common::errors::types::CompilerWarning::Semantic(w))
                    if matches!(w.kind, SemanticWarningKind::UnusedVariable(_))
            )),
            "atribuir à variável conta como uso"
        );
    }

    // ── Warnings: uninitialized ───────────────────────────────────────────────

    #[test]
    fn uninitialized_use_emits_warning() {
        // int x; return x;  -> x lida sem inicialização
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
            Stmt::Return(Some(ident("x")), span()),
        ]);
        let diags = analyse(&prog);
        assert!(errors(&diags).is_empty(), "não deve haver erros");
        assert!(
            diags.iter().any(|d| matches!(
                d,
                Diagnostic::Warning(crate::common::errors::types::CompilerWarning::Semantic(w))
                    if matches!(&w.kind, SemanticWarningKind::MayBeUninitialized(n) if n == "x")
            )),
            "deve emitir warning de uninitialized para 'x'"
        );
    }

    #[test]
    fn initialized_then_used_emits_no_warning() {
        // int x = 0; return x;  -> x inicializada, nenhum warning
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), Some(int_lit(0)), span()),
            Stmt::Return(Some(ident("x")), span()),
        ]);
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
        let diags = analyse(&prog);
        assert!(diags.iter().any(|e| matches!(
            e,
            Diagnostic::Error(crate::common::errors::types::CompilerError::Semantic(se))
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
        let diags = analyse(&prog);
        assert!(diags.iter().any(|e| matches!(
            e,
            Diagnostic::Error(crate::common::errors::types::CompilerError::Semantic(se))
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
                prototype_only: false,
                used: true,
                initialized: true,
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
                prototype_only: false,
                used: true,
                initialized: true,
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
                prototype_only: false,
                used: true,
                initialized: true,
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
        let diags = analyse(&prog);
        assert!(diags.iter().any(|e| matches!(
            e,
            Diagnostic::Error(crate::common::errors::types::CompilerError::Semantic(se))
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
                prototype_only: false,
                used: false,
                initialized: true,
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
    fn ternary_enum_condition_is_valid() {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        analyser
            .sym
            .declare(crate::analyser::symbol_table::Symbol {
                name: "e".into(),
                ty: qty(Type::Enum("Color".into())),
                mutable: true,
                params: None,
                decl_span: span(),
                prototype_only: false,
                used: false,
                initialized: true,
            })
            .unwrap();
        // e ? 1 : 2  → enum é escalar, deve ser válido
        let expr = Expr::Ternary(
            Box::new(ident("e")),
            Box::new(int_lit(1)),
            Box::new(int_lit(2)),
            span(),
        );
        let ty = analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.is_empty(),
            "condição enum no ternário deve ser válida (enum é tipo inteiro em C)"
        );
        assert!(matches!(ty.ty, Type::Int));
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

    // ── Protótipos de função (forward declarations) ───────────────────────────

    /// Caso 1: protótipo declarado antes da definição → chamada válida sem erros.
    #[test]
    fn prototype_valid_forward_call_is_ok() {
        // int soma(int a, int b);
        // int main() { return soma(1, 2); }
        // int soma(int a, int b) { return a + b; }
        let prog = Program {
            decls: vec![
                Decl::Prototype(
                    qty(Type::Int),
                    "soma".into(),
                    vec![(qty(Type::Int), "a".into()), (qty(Type::Int), "b".into())],
                    span(),
                ),
                Decl::Function(
                    qty(Type::Int),
                    "main".into(),
                    vec![],
                    vec![Stmt::Return(
                        Some(Expr::Call(
                            Box::new(ident("soma")),
                            vec![int_lit(1), int_lit(2)],
                            span(),
                        )),
                        span(),
                    )],
                    span(),
                ),
                Decl::Function(
                    qty(Type::Int),
                    "soma".into(),
                    vec![(qty(Type::Int), "a".into()), (qty(Type::Int), "b".into())],
                    vec![],
                    span(),
                ),
            ],
        };
        assert!(
            analyse(&prog).is_empty(),
            "chamada com protótipo declarado antes deve ser válida"
        );
    }

    /// Caso 2: chamada sem protótipo → `UndefinedVariable`.
    #[test]
    fn prototype_call_without_declaration_emits_undefined() {
        // int main() { return soma(1, 2); }  // sem protótipo
        let prog = Program {
            decls: vec![Decl::Function(
                qty(Type::Int),
                "main".into(),
                vec![],
                vec![Stmt::Return(
                    Some(Expr::Call(
                        Box::new(ident("soma")),
                        vec![int_lit(1), int_lit(2)],
                        span(),
                    )),
                    span(),
                )],
                span(),
            )],
        };
        let diags = analyse(&prog);
        assert!(
            diags.iter().any(|e| matches!(
                e,
                Diagnostic::Error(crate::common::errors::types::CompilerError::Semantic(se))
                    if matches!(&se.kind, SemanticErrorKind::UndefinedVariable(n) if n == "soma")
            )),
            "chamada sem protótipo deve emitir UndefinedVariable"
        );
    }

    /// Caso 3: protótipo com tipo de retorno diferente da definição → `PrototypeMismatch`.
    #[test]
    fn prototype_mismatch_return_type_emits_error() {
        // float soma(int a, int b);  // protótipo diz float
        // int soma(int a, int b) { ... }  // definição diz int
        let prog = Program {
            decls: vec![
                Decl::Prototype(
                    qty(Type::Float), // retorno float no protótipo
                    "soma".into(),
                    vec![(qty(Type::Int), "a".into()), (qty(Type::Int), "b".into())],
                    span(),
                ),
                Decl::Function(
                    qty(Type::Int), // retorno int na definição
                    "soma".into(),
                    vec![(qty(Type::Int), "a".into()), (qty(Type::Int), "b".into())],
                    vec![],
                    span(),
                ),
            ],
        };
        let diags = analyse(&prog);
        assert!(
            diags.iter().any(|e| matches!(
                e,
                Diagnostic::Error(crate::common::errors::types::CompilerError::Semantic(se))
                    if matches!(&se.kind, SemanticErrorKind::PrototypeMismatch { name, .. } if name == "soma")
            )),
            "retorno diferente entre protótipo e definição deve emitir PrototypeMismatch"
        );
    }

    /// Caso 4: protótipo com parâmetros diferentes da definição → `PrototypeMismatch`.
    #[test]
    fn prototype_mismatch_param_types_emits_error() {
        // int soma(float a, int b);  // protótipo diz float no 1º param
        // int soma(int a, int b) { ... }  // definição diz int
        let prog = Program {
            decls: vec![
                Decl::Prototype(
                    qty(Type::Int),
                    "soma".into(),
                    vec![(qty(Type::Float), "a".into()), (qty(Type::Int), "b".into())],
                    span(),
                ),
                Decl::Function(
                    qty(Type::Int),
                    "soma".into(),
                    vec![(qty(Type::Int), "a".into()), (qty(Type::Int), "b".into())],
                    vec![],
                    span(),
                ),
            ],
        };
        let diags = analyse(&prog);
        assert!(
            diags.iter().any(|e| matches!(
                e,
                Diagnostic::Error(crate::common::errors::types::CompilerError::Semantic(se))
                    if matches!(&se.kind, SemanticErrorKind::PrototypeMismatch { name, .. } if name == "soma")
            )),
            "parâmetros diferentes entre protótipo e definição deve emitir PrototypeMismatch"
        );
    }

    /// Caso 5: protótipo sem implementação correspondente → `PrototypeMissingBody`.
    #[test]
    fn prototype_without_body_emits_error() {
        // int soma(int a, int b);  // nunca implementada
        let prog = Program {
            decls: vec![Decl::Prototype(
                qty(Type::Int),
                "soma".into(),
                vec![(qty(Type::Int), "a".into()), (qty(Type::Int), "b".into())],
                span(),
            )],
        };
        let diags = analyse(&prog);
        assert!(
            diags.iter().any(|e| matches!(
                e,
                Diagnostic::Error(crate::common::errors::types::CompilerError::Semantic(se))
                    if matches!(&se.kind, SemanticErrorKind::PrototypeMissingBody(n) if n == "soma")
            )),
            "protótipo sem implementação deve emitir PrototypeMissingBody"
        );
    }

    #[test]
    fn assignment_before_use_initializes_variable() {
        // int x; x = 5; return x;  -> atribuição inicializa antes do uso
        let prog = program(vec![
            Stmt::VarDecl(qty(Type::Int), "x".into(), None, span()),
            Stmt::ExprStmt(
                Expr::Assign(Box::new(ident("x")), Box::new(int_lit(5)), span()),
                span(),
            ),
            Stmt::Return(Some(ident("x")), span()),
        ]);
        let diags = analyse(&prog);
        assert!(
            !diags.iter().any(|d| matches!(
                d,
                Diagnostic::Warning(crate::common::errors::types::CompilerWarning::Semantic(w))
                    if matches!(w.kind, SemanticWarningKind::MayBeUninitialized(_))
            )),
            "atribuição antes do uso inicializa a variável"
        );
    }
}
