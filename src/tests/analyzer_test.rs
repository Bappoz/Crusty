
use crate::analyser::analyzer::Analyser;
use crate::analyser::symbol_table::Symbol;
use crate::common::ast::ast::{QualifierType, Type};
use crate::common::ast::expr::{Expr, MemberAccess};
use crate::common::errors::error_data::Span;

#[cfg(test)]
mod test{
    use super::*; // chama todos os imports para dentro desse mod

    fn dummy_span () -> Span {
        Span{
            line: 1,
            end_line: 1,
            column_start: 1,
            column_end: 5,
        }
    }

    #[test]
    fn test_direct_access_struct(){

        // instacia o objeto que possui a estrutura do struct_table (vazia)
        let mut analyzer = Analyser::new();
        analyzer.symbols.enter_scope();

        // cria um campo da struct do tipo int
        let field_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        // cria tupla (QualifierType, Symbol)
        let fields_struct = vec![(field_type, "nome".to_string())];

        // insere na struct_table
        analyzer.symbols.register_struct("Pointer".to_string(), fields_struct);

        // chamada dentro de uma função - struct Pointer meu_ponto
        let struct_call = QualifierType {
            ty: Type::Struct("Pointer".to_string()),
            is_const: false,
            is_unsigned: false,
        }; // Define que há uma struct que utiliza a estrutura do Pointer

        analyzer.symbols.declare(Symbol {
            name: "meu_ponto".to_string(),
            ty: struct_call,
            mutable: true,
            decl_span: dummy_span(),
        }).unwrap();
        
        let span = dummy_span();

        let left_ast = Box::new(Expr::Ident("meu_ponto".to_string(), span.clone()));

        let expr_completa = Expr::Member(
            left_ast,
            MemberAccess::Direct,
            "nome".to_string(),
            span,
        );

        let result = analyzer.check_expr(&expr_completa);

        assert!(result.is_ok(), "O analisador deveria ter aceitado o acesso ao campo criado.");

        let detected_type = result.unwrap();

        assert_eq!(detected_type.ty, Type::Int, "O tipo retornado deveria ser Type::Int.");
    }

    #[test]
    fn test_field_not_found(){

        let mut analyzer = Analyser::new();
        analyzer.symbols.enter_scope();

        let field_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        let fields_struct  = vec![(field_type, "nome".to_string())];
        analyzer.symbols.register_struct("Pointer".to_string(), fields_struct);

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
        let left_ast = Box::new(Expr::Ident("meu_ponto".to_string(), span.clone()));

        let expr_completa = Expr::Member(
            left_ast,
            MemberAccess::Direct,
            "balacobaco".to_string(),
            span,
        );

        let result = analyzer.check_expr(&expr_completa);

        assert!(result.is_err(), "O analisador deveria ter rejeitado o acesso a um campo inexistente.");
    }

    #[test]
    fn  test_pointer_access_struct(){

        let mut analyzer = Analyser::new();
        analyzer.symbols.enter_scope();

        let field_type = QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        };

        let fields_struct = vec![(field_type, "nome".to_string())];
        analyzer.symbols.register_struct("Pointer".to_string(), fields_struct);

        let pointer_type = Type::Pointer(Box::new(Type::Struct("Pointer".to_string())));

        let struct_pointer_call = QualifierType{
            ty: pointer_type,
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
        let left_ast = Box::new(Expr::Ident("ponteiro_ponto".to_string(), span.clone()));

        let expr_completa = Expr::Member(
            left_ast,
            MemberAccess::Pointer,
            "nome".to_string(),
            span,
        );

        let result = analyzer.check_expr(&expr_completa);

        assert!(result.is_ok(), "O analisador deveria ter aceito o acesso via ponteiro '->'.");

        let detected_type = result.unwrap();
        assert_eq!(detected_type.ty, Type::Int, "O tipo retornado pelo ponteiro deveria ser Type::Int.");
    }
}