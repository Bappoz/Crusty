#[cfg(test)]
mod test {
    use crate::analyser::symbol_table::Symbol;
    use crate::analyser::SemanticAnalyser;
    use crate::common::ast::ast::{QualifierType, Type};
    use crate::common::ast::expr::{Expr, MemberAccess};
    use crate::common::errors::error_data::Span;

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
}
