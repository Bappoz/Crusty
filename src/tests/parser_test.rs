#[cfg(test)]
mod tests {
    use crate::common::ast::expr::{BinOp, Expr, Literal};
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
            tk(TokenKind::IntLiteral(1), 2),
            tk(TokenKind::Plus, 3),
            tk(TokenKind::IntLiteral(2), 4),
            tk(TokenKind::RightParen, 5),
            tk(TokenKind::Star, 6),
            tk(TokenKind::IntLiteral(3), 7),
            tk(TokenKind::Eof, 12),
        ];

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr(0).expect("expressão válida");

        let Expr::Binary(left, BinOp::Mul, right, _) = expr else {
            panic!("esperava multiplicação no topo da árvore");
        };

        assert!(matches!(*right, Expr::Literal(Literal::Int(3), _)));
        assert!(matches!(*left, Expr::Binary(_, BinOp::Add, _, _)));
    }

    // Garante erro sintático quando falta fechar parêntese.
    #[test]
    fn reports_missing_right_paren() {
        let tokens = vec![
            tk(TokenKind::LeftParen, 3),
            tk(TokenKind::IntLiteral(1), 7),
            tk(TokenKind::Plus, 8),
            tk(TokenKind::IntLiteral(2), 9),
            tk(TokenKind::Eof, 9),
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse_expr(0);
        assert!(result.is_err());
    }
}
