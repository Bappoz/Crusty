#[cfg(test)]
mod test {
    use crate::analyser::analyzer::Analyser;
    use crate::analyser::symbol_table::Symbol;
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

    #[test]
    fn test_direct_access_struct() {
        let mut analyzer = Analyser::new();
        analyzer.symbols.enter_scope();

        let field_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        analyzer.symbols.register_struct("Pointer".to_string(), vec![(field_type, "nome".to_string())]);

        let struct_call = QualifierType {
            ty: Type::Struct("Pointer".to_string()),
            is_const: false,
            is_unsigned: false,
        };

        analyzer.symbols.declare(Symbol {
            name: "meu_ponto".to_string(),
            ty: struct_call,
            mutable: true,
            decl_span: dummy_span(),
        }).unwrap();

        let span = dummy_span();
        let expr = Expr::Member(
            Box::new(Expr::Ident("meu_ponto".to_string(), span.clone())),
            MemberAccess::Direct,
            "nome".to_string(),
            span,
        );

        let result = analyzer.check_expr(&expr);
        assert!(result.is_ok(), "deveria aceitar acesso direto a campo existente");
        assert_eq!(result.unwrap().ty, Type::Int);
    }

    #[test]
    fn test_field_not_found() {
        let mut analyzer = Analyser::new();
        analyzer.symbols.enter_scope();

        let field_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        analyzer.symbols.register_struct("Pointer".to_string(), vec![(field_type, "nome".to_string())]);

        let struct_call = QualifierType {
            ty: Type::Struct("Pointer".to_string()),
            is_const: false,
            is_unsigned: false,
        };

        analyzer.symbols.declare(Symbol {
            name: "meu_ponto".to_string(),
            ty: struct_call,
            mutable: true,
            decl_span: dummy_span(),
        }).unwrap();

        let span = dummy_span();
        let expr = Expr::Member(
            Box::new(Expr::Ident("meu_ponto".to_string(), span.clone())),
            MemberAccess::Direct,
            "balacobaco".to_string(),
            span,
        );

        let result = analyzer.check_expr(&expr);
        assert!(result.is_err(), "deveria rejeitar campo inexistente");
    }

    #[test]
    fn test_pointer_access_struct() {
        let mut analyzer = Analyser::new();
        analyzer.symbols.enter_scope();

        let field_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        analyzer.symbols.register_struct("Pointer".to_string(), vec![(field_type, "nome".to_string())]);

        let struct_pointer_call = QualifierType {
            ty: Type::Pointer(Box::new(Type::Struct("Pointer".to_string()))),
            is_const: false,
            is_unsigned: false,
        };

        analyzer.symbols.declare(Symbol {
            name: "ponteiro_ponto".to_string(),
            ty: struct_pointer_call,
            mutable: true,
            decl_span: dummy_span(),
        }).unwrap();

        let span = dummy_span();
        let expr = Expr::Member(
            Box::new(Expr::Ident("ponteiro_ponto".to_string(), span.clone())),
            MemberAccess::Pointer,
            "nome".to_string(),
            span,
        );

        let result = analyzer.check_expr(&expr);
        assert!(result.is_ok(), "deveria aceitar acesso via ponteiro");
        assert_eq!(result.unwrap().ty, Type::Int);
    }
}
