#[cfg(test)]
mod tests {
    use crate::common::ast::decl::Decl;
    use crate::common::ast::expr::{BinOp, Expr, Literal};
    use crate::common::ast::stmt::Stmt;
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
    fn parses_precedence_in_expression_statement() {
        let tokens = vec![
            tk(TokenKind::Int, 1),
            tk(TokenKind::Identifier("main".into()), 2),
            tk(TokenKind::LeftParen, 3),
            tk(TokenKind::RightParen, 4),
            tk(TokenKind::LeftBrace, 5),
            tk(TokenKind::IntLiteral(1), 6),
            tk(TokenKind::Plus, 7),
            tk(TokenKind::IntLiteral(2), 8),
            tk(TokenKind::Star, 9),
            tk(TokenKind::IntLiteral(3), 10),
            tk(TokenKind::Semicolon, 11),
            tk(TokenKind::RightBrace, 12),
            tk(TokenKind::Eof, 13),
        ];

        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();

        assert!(parser.diagnostics().is_empty());
        assert_eq!(program.decls.len(), 1);

        let Decl::Function(_, _, _, body, _) = &program.decls[0] else {
            panic!("esperava função");
        };

        let Stmt::ExprStmt(expr, _) = &body[0] else {
            panic!("esperava expression statement");
        };

        let Expr::Binary(left, BinOp::Add, right, _) = expr else {
            panic!("esperava soma no topo da árvore");
        };

        assert!(matches!(**left, Expr::Literal(Literal::Int(1), _)));
        assert!(matches!(**right, Expr::Binary(_, BinOp::Mul, _, _)));
    }

    // Garante parsing de declaração local com inicialização.
    #[test]
    fn parses_local_var_declaration() {
        let tokens = vec![
            tk(TokenKind::Int, 1),
            tk(TokenKind::Identifier("main".into()), 2),
            tk(TokenKind::LeftParen, 3),
            tk(TokenKind::RightParen, 4),
            tk(TokenKind::LeftBrace, 5),
            tk(TokenKind::Int, 6),
            tk(TokenKind::Identifier("x".into()), 7),
            tk(TokenKind::Equal, 8),
            tk(TokenKind::IntLiteral(42), 9),
            tk(TokenKind::Semicolon, 10),
            tk(TokenKind::RightBrace, 11),
            tk(TokenKind::Eof, 12),
        ];

        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();

        assert!(parser.diagnostics().is_empty());

        let Decl::Function(_, _, _, body, _) = &program.decls[0] else {
            panic!("esperava função");
        };

        let Stmt::VarDecl(_, name, init, _) = &body[0] else {
            panic!("esperava declaração local");
        };

        assert_eq!(name, "x");
        assert!(matches!(init, Some(Expr::Literal(Literal::Int(42), _))));
    }

    // Garante que erro sintático básico é reportado quando falta ';'.
    #[test]
    fn reports_missing_semicolon() {
        let tokens = vec![
            tk(TokenKind::Int, 1),
            tk(TokenKind::Identifier("main".into()), 2),
            tk(TokenKind::LeftParen, 3),
            tk(TokenKind::RightParen, 4),
            tk(TokenKind::LeftBrace, 5),
            tk(TokenKind::Return, 6),
            tk(TokenKind::IntLiteral(1), 7),
            tk(TokenKind::RightBrace, 8),
            tk(TokenKind::Eof, 9),
        ];

        let mut parser = Parser::new(tokens);
        let _ = parser.parse_program();

        assert!(!parser.diagnostics().is_empty());
    }
}
