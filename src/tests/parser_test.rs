#[cfg(test)]
mod tests {
    use crate::common::ast::ast::{QualifierType, Type};
    use crate::common::ast::expr::{BinOp, Expr, Literal, PostfixOp, PrefixOp};
    use crate::common::input::span::ByteSpan;
    use crate::lexer::tokens::token::Token;
    use crate::lexer::tokens::token_kind::TokenKind;
    use crate::parser::Parser;

    // Helper para criar tokens compactos nos testes sem depender do scanner.
    fn tk(kind: TokenKind, col: usize) -> Token {
        Token {
            kind,
            span: ByteSpan::new(col, col + 1),
            line: 1,
            col,
        }
    }

    fn ident(name: &str, col: usize) -> Token {
        tk(TokenKind::Identifier(name.to_string()), col)
    }

    fn int(value: i64, col: usize) -> Token {
        tk(TokenKind::IntLiteral(value), col)
    }

    fn eof(col: usize) -> Token {
        tk(TokenKind::Eof, col)
    }

    // Garante que precedência de multiplicação vence soma: 1 + 2 * 3.
    #[test]
    fn parses_precedence_in_expression() {
        let tokens = vec![
            tk(TokenKind::IntLiteral(1), 6),
            tk(TokenKind::Plus, 7),
            tk(TokenKind::IntLiteral(2), 8),
            tk(TokenKind::Star, 9),
            tk(TokenKind::IntLiteral(3), 10),
            tk(TokenKind::Eof, 13),
        ];

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr(0).expect("expressão válida");

        let Expr::Binary(left, BinOp::Add, right, _) = expr else {
            panic!("esperava soma no topo da árvore");
        };

        assert!(matches!(*left, Expr::Literal(Literal::Int(1), _)));
        assert!(matches!(*right, Expr::Binary(_, BinOp::Mul, _, _)));
    }

    // Garante respeito ao agrupamento com parênteses: (1 + 2) * 3.
    #[test]
    fn parses_grouped_expression() {
        let tokens = vec![
            tk(TokenKind::LeftParen, 1),
            int(1, 2),
            tk(TokenKind::Plus, 3),
            int(2, 4),
            tk(TokenKind::RightParen, 5),
            tk(TokenKind::Star, 6),
            int(3, 7),
            eof(12),
        ];

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr(0).expect("expressão válida");

        let Expr::Binary(left, BinOp::Mul, right, _) = expr else {
            panic!("esperava multiplicação no topo da árvore");
        };

        assert!(matches!(*right, Expr::Literal(Literal::Int(3), _)));
        assert!(matches!(*left, Expr::Binary(_, BinOp::Add, _, _)));
    }

    #[test]
    fn parses_prefix_operators() {
        let cases = vec![
            (vec![tk(TokenKind::Minus, 1), ident("x", 2), eof(3)], "neg"),
            (vec![tk(TokenKind::Bang, 1), ident("x", 2), eof(3)], "not"),
            (vec![tk(TokenKind::PlusPlus, 1), ident("x", 3), eof(4)], "inc"),
        ];

        for (tokens, kind) in cases {
            let mut parser = Parser::new(tokens);
            let expr = parser.parse_expr(0).expect("expressão válida");
            match (kind, expr) {
                ("neg", Expr::Unary(_, _, _)) => {}
                ("not", Expr::Unary(_, _, _)) => {}
                ("inc", Expr::Prefix(PrefixOp::Inc, _, _)) => {}
                _ => panic!("nó prefixo inesperado"),
            }
        }
    }

    #[test]
    fn parses_postfix_operators() {
        let cases = vec![
            vec![ident("x", 1), tk(TokenKind::PlusPlus, 2), eof(4)],
            vec![ident("x", 1), tk(TokenKind::LeftBracket, 2), int(0, 3), tk(TokenKind::RightBracket, 4), eof(5)],
            vec![
                ident("f", 1),
                tk(TokenKind::LeftParen, 2),
                int(1, 3),
                tk(TokenKind::Comma, 4),
                int(2, 5),
                tk(TokenKind::RightParen, 6),
                eof(7),
            ],
        ];

        let first = Parser::new(cases[0].clone()).parse_expr(0).expect("postfix válido");
        assert!(matches!(first, Expr::Postfix(PostfixOp::Inc, _, _)));

        let second = Parser::new(cases[1].clone()).parse_expr(0).expect("indexação válida");
        assert!(matches!(second, Expr::Index(_, _, _)));

        let third = Parser::new(cases[2].clone()).parse_expr(0).expect("chamada válida");
        let Expr::Call(_, args, _) = third else {
            panic!("esperava chamada de função");
        };
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn parses_cast_expression() {
        let tokens = vec![
            tk(TokenKind::LeftParen, 1),
            tk(TokenKind::Int, 2),
            tk(TokenKind::RightParen, 3),
            ident("x", 4),
            eof(5),
        ];

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr(0).expect("cast válido");

        let Expr::Cast(QualifierType { ty, is_const, is_unsigned }, inner, _) = expr else {
            panic!("esperava cast no topo da árvore");
        };

        assert!(matches!(ty, Type::Int));
        assert!(!is_const);
        assert!(!is_unsigned);
        assert!(matches!(*inner, Expr::Ident(_, _)));
    }

    #[test]
    fn parses_assignment_expression() {
        let tokens = vec![ident("x", 1), tk(TokenKind::Equal, 2), ident("y", 3), eof(4)];

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr(0).expect("atribuição válida");

        let Expr::Assign(lhs, rhs, _) = expr else {
            panic!("esperava atribuição no topo da árvore");
        };

        assert!(matches!(*lhs, Expr::Ident(_, _)));
        assert!(matches!(*rhs, Expr::Ident(_, _)));
    }

    #[test]
    fn rejects_invalid_operator_tokens() {
        let tokens = vec![int(1, 1), tk(TokenKind::Unknown('?'), 2), int(2, 3), eof(4)];

        let mut parser = Parser::new(tokens);
        assert!(parser.parse_expr(0).is_err());
    }

    // Garante erro sintático quando falta fechar parêntese.
    #[test]
    fn reports_missing_right_paren() {
        let tokens = vec![
            tk(TokenKind::LeftParen, 3),
            int(1, 7),
            tk(TokenKind::Plus, 8),
            int(2, 9),
            eof(9),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse_expr(0);
        assert!(result.is_err());
    }
}
