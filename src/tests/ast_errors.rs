use crate::common::ast::expr::{Expr, Literal, BinOp, UnOp};
use crate::common::ast::ast::{QualifierType, Type};
use crate::common::errors::error_data::Span;

#[cfg(test)]
mod test{
    use super::*; //sobe onível de hierarquia

    #[test]
    fn ast_nodes_are_debug_printable() {
        let span = Span { line: 1, end_line: 1, column_start: 1, column_end: 5 };
        // Criando um literal "10"
        let expr = Expr::Literal(Literal::Int(10), span.clone());
        
        println!("{:?}", expr);
        assert_eq!(expr.span().line, 1);
    }

    #[test]
    fn binary_expr_span_covers_both_operands() {
        let span1 = Span { line: 1, end_line: 1, column_start: 1, column_end: 2 };
        let span2 = Span { line: 1, end_line: 1, column_start: 5, column_end: 6 };
        let span_total = Span { line: 1, end_line: 1, column_start: 1, column_end: 6 };

        let left = Box::new(Expr::Literal(Literal::Int(1), span1));
        let right = Box::new(Expr::Literal(Literal::Int(2), span2));
        
        // 1 + 2
        let binary = Expr::Binary(left, BinOp::Add, right, span_total.clone());
        
        assert_eq!(binary.span(), span_total);
    }

    #[test]
    fn test_nested_pointer_types(){
        let int_ptr = Type::Pointer(Box::new(Type::Int));
        let double_ptr = Type::Pointer(Box::new(int_ptr));

        let q_type = QualifierType{
            ty: double_ptr,
            is_const: false,
            is_unsigned: false,
        };

      
        if let Type::Pointer(inner) = q_type.ty{ // Verificação se q_type.ty é enum Pointer e depois passa para inner
            if let Type::Pointer(base) = *inner{ // Verifica se o q_type.ty está apontando para outro Pointer
                assert_eq! (*base, Type::Int); // base -> inner -> Int?
            }
            else{
                panic!("Deveria ser outro ponteiro aqui!");
            }
        }
    }

    #[test]
    fn test_unary_operations(){
        let span = Span { line: 1, end_line: 1, column_start: 1, column_end: 2 };
        let expr = Box::new(Expr::Ident("IdentificadoVariável".to_string(), span.clone()));

        let deref_expr = Expr::Unary(UnOp::Deref, expr, span);

        println!("{:?}", deref_expr);
    }
}