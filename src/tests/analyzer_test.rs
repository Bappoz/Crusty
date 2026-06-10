#[cfg(test)]
mod test {
    use crate::analyser::symbol_table::Symbol;
    use crate::analyser::SemanticAnalyser;
    use crate::common::ast::ast::{QualifierType, Type};
    use crate::common::ast::decl::Decl;
    use crate::common::ast::expr::{Expr, Literal, MemberAccess};
    use crate::common::ast::stmt::Stmt;
    use crate::common::errors::error_data::Span;
    use crate::common::errors::types::SemanticErrorKind;

    fn dummy_span() -> Span {
        Span {
            line: 1,
            end_line: 1,
            column_start: 1,
            column_end: 5,
        }
    }

    fn make_analyser_with_struct() -> SemanticAnalyser {
        let mut analyser = SemanticAnalyser::new();
        analyser.sym.enter_scope();
        analyser.sym.register_struct(
            "Pointer".to_string(),
            vec![(
                QualifierType {
                    ty: Type::Int,
                    is_const: false,
                    is_unsigned: false,
                },
                "nome".to_string(),
            )],
        );
        analyser
    }

    #[test]
    fn test_direct_access_struct() {
        let mut analyser = make_analyser_with_struct();
        analyser
            .sym
            .declare(Symbol {
                name: "meu_ponto".to_string(),
                ty: QualifierType {
                    ty: Type::Struct("Pointer".to_string()),
                    is_const: false,
                    is_unsigned: false,
                },
                mutable: true,
                params: None,
                decl_span: dummy_span(),
            })
            .unwrap();

        let span = dummy_span();
        let expr = Expr::Member(
            Box::new(Expr::Ident("meu_ponto".to_string(), span.clone())),
            MemberAccess::Direct,
            "nome".to_string(),
            span,
        );

        let ty = analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.is_empty(),
            "deveria aceitar acesso direto a campo existente"
        );
        assert_eq!(ty.ty, Type::Int);
    }

    #[test]
    fn test_field_not_found() {
        let mut analyser = make_analyser_with_struct();
        analyser
            .sym
            .declare(Symbol {
                name: "meu_ponto".to_string(),
                ty: QualifierType {
                    ty: Type::Struct("Pointer".to_string()),
                    is_const: false,
                    is_unsigned: false,
                },
                mutable: true,
                params: None,
                decl_span: dummy_span(),
            })
            .unwrap();

        let span = dummy_span();
        let expr = Expr::Member(
            Box::new(Expr::Ident("meu_ponto".to_string(), span.clone())),
            MemberAccess::Direct,
            "balacobaco".to_string(),
            span,
        );

        analyser.analyse_expr(&expr);
        assert!(
            !analyser.diagnostics.is_empty(),
            "deveria rejeitar campo inexistente"
        );
    }

    #[test]
    fn test_pointer_access_struct() {
        let mut analyser = make_analyser_with_struct();
        analyser
            .sym
            .declare(Symbol {
                name: "ponteiro_ponto".to_string(),
                ty: QualifierType {
                    ty: Type::Pointer(Box::new(Type::Struct("Pointer".to_string()))),
                    is_const: false,
                    is_unsigned: false,
                },
                mutable: true,
                params: None,
                decl_span: dummy_span(),
            })
            .unwrap();

        let span = dummy_span();
        let expr = Expr::Member(
            Box::new(Expr::Ident("ponteiro_ponto".to_string(), span.clone())),
            MemberAccess::Pointer,
            "nome".to_string(),
            span,
        );

        let ty = analyser.analyse_expr(&expr);
        assert!(
            analyser.diagnostics.is_empty(),
            "deveria aceitar acesso via ponteiro"
        );
        assert_eq!(ty.ty, Type::Int);
    }

    #[test]
    fn test_return_type_mismatch_error() {
        let mut analyser = SemanticAnalyser::new();
        let span = dummy_span();

        let ret_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        let expr_errada = Expr::Literal(Literal::Double(2.5), span.clone());
        let stmt_return = Stmt::Return(Some(expr_errada), span.clone());

        let funcao_ast = Decl::Function(
            ret_type,
            "minha_funcao".to_string(),
            vec![],
            vec![stmt_return],
            span,
        );

        analyser.sym.enter_scope();
        analyser.analyse_decl(&funcao_ast);
        analyser.sym.exit_scope();

        assert_eq!(
            analyser.diagnostics.len(),
            1,
            "Deveria ter detectado incopatibilidade: Int x Double."
        );
    }

    #[test]
    fn test_empty_return_in_non_void_error() {
        let mut analyser = SemanticAnalyser::new();
        let span = dummy_span();

        let ret_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        let stmt_return = Stmt::Return(None, span.clone());

        let funcao_ast = Decl::Function(
            ret_type,
            "minha_funcao".to_string(),
            vec![],
            vec![stmt_return],
            span,
        );

        analyser.sym.enter_scope();
        analyser.analyse_decl(&funcao_ast);
        analyser.sym.exit_scope();

        assert_eq!(
            analyser.diagnostics.len(),
            1,
            "Deveria ter detectado um retorno vazio em uma função int."
        );
    }

    #[test]
    fn test_valid_return_success() {
        let mut analyser = SemanticAnalyser::new();
        let span = dummy_span();

        let ret_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        let expr_correta = Expr::Literal(Literal::Int(10), span.clone());
        let stmt_return = Stmt::Return(Some(expr_correta), span.clone());

        let funcao_ast = Decl::Function(
            ret_type,
            "minha_funcao".to_string(),
            vec![],
            vec![stmt_return],
            span,
        );

        analyser.sym.enter_scope();
        analyser.analyse_decl(&funcao_ast);
        analyser.sym.exit_scope();

        assert_eq!(
            analyser.diagnostics.len(),
            0,
            "Não deveria ter detectado erro em um retorno válido"
        );
    }

    #[test]
    fn test_return_in_void_function_error() {
        let mut analyser = SemanticAnalyser::new();
        let span = dummy_span();

        let ret_type = QualifierType {
            ty: Type::Void,
            is_const: false,
            is_unsigned: false,
        };

        let expr = Expr::Literal(Literal::Int(10), span.clone());
        let stmt_return = Stmt::Return(Some(expr), span.clone());

        let funcao_ast = Decl::Function(
            ret_type,
            "minha_funcao_void".to_string(),
            vec![],
            vec![stmt_return],
            span,
        );

        analyser.sym.enter_scope();
        analyser.analyse_decl(&funcao_ast);
        analyser.sym.exit_scope();

        assert!(
            analyser.diagnostics.iter().any(|error| matches!(
                &error,
                crate::common::errors::types::CompilerError::Semantic(se)
                    if matches!(se.kind, SemanticErrorKind::ReturnInVoid)
            )),
            "deveria sinalizar retorno com valor em função void"
        );
    }
}
