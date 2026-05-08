#[cfg(test)]
mod tests {
    use crate::common::ast::ast::{QualifierType, Type};
    use crate::common::ast::expr::{BinOp, Expr, Literal, PostfixOp, PrefixOp};
    use crate::common::ast::stmt::Stmt;
    use crate::common::input::span::ByteSpan;
    use crate::lexer::tokens::token::Token;
    use crate::lexer::tokens::token_kind::TokenKind;
    use crate::parser::rules::statements::parse_stmt;
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

    // ── testes de expressão ──────────────────────────────────────────────────

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
        println!("[parses_precedence_in_expression] AST: {expr:#?}");

        let Expr::Binary(left, BinOp::Add, right, _) = expr else {
            panic!("esperava soma no topo da árvore");
        };
        println!("[parses_precedence_in_expression] nó raiz = Add  ✓");
        println!("[parses_precedence_in_expression] left  = {left:?}");
        println!("[parses_precedence_in_expression] right = {right:?}");

        assert!(matches!(*left, Expr::Literal(Literal::Int(1), _)));
        println!("[parses_precedence_in_expression] left é Literal(1)  ✓");
        assert!(matches!(*right, Expr::Binary(_, BinOp::Mul, _, _)));
        println!("[parses_precedence_in_expression] right é Mul  ✓");
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
        println!("[parses_grouped_expression] AST: {expr:#?}");

        let Expr::Binary(left, BinOp::Mul, right, _) = expr else {
            panic!("esperava multiplicação no topo da árvore");
        };
        println!("[parses_grouped_expression] nó raiz = Mul  ✓");
        println!("[parses_grouped_expression] left  = {left:?}");
        println!("[parses_grouped_expression] right = {right:?}");

        assert!(matches!(*right, Expr::Literal(Literal::Int(3), _)));
        println!("[parses_grouped_expression] right é Literal(3)  ✓");
        assert!(matches!(*left, Expr::Binary(_, BinOp::Add, _, _)));
        println!("[parses_grouped_expression] left é Add (grupo parênteses)  ✓");
    }

    #[test]
    fn parses_prefix_operators() {
        let cases = vec![
            (vec![tk(TokenKind::Minus, 1), ident("x", 2), eof(3)], "neg"),
            (vec![tk(TokenKind::Bang, 1), ident("x", 2), eof(3)], "not"),
            (
                vec![tk(TokenKind::PlusPlus, 1), ident("x", 3), eof(4)],
                "inc",
            ),
        ];

        for (tokens, kind) in cases {
            let mut parser = Parser::new(tokens);
            let expr = parser.parse_expr(0).expect("expressão válida");
            println!("[parses_prefix_operators] caso '{kind}' => AST: {expr:?}");
            match (kind, expr) {
                ("neg", Expr::Unary(_, _, _)) => println!("[parses_prefix_operators] '-x' é Unary  ✓"),
                ("not", Expr::Unary(_, _, _)) => println!("[parses_prefix_operators] '!x' é Unary  ✓"),
                ("inc", Expr::Prefix(PrefixOp::Inc, _, _)) => println!("[parses_prefix_operators] '++x' é Prefix::Inc  ✓"),
                _ => panic!("nó prefixo inesperado"),
            }
        }
    }

    #[test]
    fn parses_postfix_operators() {
        let cases = vec![
            vec![ident("x", 1), tk(TokenKind::PlusPlus, 2), eof(4)],
            vec![
                ident("x", 1),
                tk(TokenKind::LeftBracket, 2),
                int(0, 3),
                tk(TokenKind::RightBracket, 4),
                eof(5),
            ],
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

        let first = Parser::new(cases[0].clone())
            .parse_expr(0)
            .expect("postfix válido");
        println!("[parses_postfix_operators] 'x++' => {first:?}");
        assert!(matches!(first, Expr::Postfix(PostfixOp::Inc, _, _)));
        println!("[parses_postfix_operators] 'x++' é Postfix::Inc  ✓");

        let second = Parser::new(cases[1].clone())
            .parse_expr(0)
            .expect("indexação válida");
        println!("[parses_postfix_operators] 'x[0]' => {second:?}");
        assert!(matches!(second, Expr::Index(_, _, _)));
        println!("[parses_postfix_operators] 'x[0]' é Index  ✓");

        let third = Parser::new(cases[2].clone())
            .parse_expr(0)
            .expect("chamada válida");
        println!("[parses_postfix_operators] 'f(1,2)' => {third:?}");
        let Expr::Call(_, args, _) = third else {
            panic!("esperava chamada de função");
        };
        println!("[parses_postfix_operators] 'f(1,2)' é Call com {} args  ✓", args.len());
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
        println!("[parses_cast_expression] AST: {expr:#?}");

        let Expr::Cast(
            QualifierType {
                ty,
                is_const,
                is_unsigned,
            },
            inner,
            _,
        ) = expr
        else {
            panic!("esperava cast no topo da árvore");
        };
        println!("[parses_cast_expression] Cast para ty={ty:?}  is_const={is_const}  is_unsigned={is_unsigned}");

        assert!(matches!(ty, Type::Int));
        println!("[parses_cast_expression] ty é Int  ✓");
        assert!(!is_const);
        println!("[parses_cast_expression] is_const=false  ✓");
        assert!(!is_unsigned);
        println!("[parses_cast_expression] is_unsigned=false  ✓");
        println!("[parses_cast_expression] inner = {inner:?}");
        assert!(matches!(*inner, Expr::Ident(_, _)));
        println!("[parses_cast_expression] inner é Ident  ✓");
    }

    #[test]
    fn parses_assignment_expression() {
        let tokens = vec![
            ident("x", 1),
            tk(TokenKind::Equal, 2),
            ident("y", 3),
            eof(4),
        ];

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr(0).expect("atribuição válida");
        println!("[parses_assignment_expression] AST: {expr:#?}");

        let Expr::Assign(lhs, rhs, _) = expr else {
            panic!("esperava atribuição no topo da árvore");
        };
        println!("[parses_assignment_expression] lhs = {lhs:?}");
        println!("[parses_assignment_expression] rhs = {rhs:?}");

        assert!(matches!(*lhs, Expr::Ident(_, _)));
        println!("[parses_assignment_expression] lhs é Ident  ✓");
        assert!(matches!(*rhs, Expr::Ident(_, _)));
        println!("[parses_assignment_expression] rhs é Ident  ✓");
    }

    #[test]
    fn rejects_invalid_operator_tokens() {
        let tokens = vec![int(1, 1), tk(TokenKind::Unknown('?'), 2), int(2, 3), eof(4)];

        let mut parser = Parser::new(tokens);
        let result = parser.parse_expr(0);
        println!("[rejects_invalid_operator_tokens] resultado: {result:?}");
        assert!(result.is_err());
        println!("[rejects_invalid_operator_tokens] erro esperado ao encontrar '?'  ✓");
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
        println!("[reports_missing_right_paren] resultado: {result:?}");
        assert!(result.is_err());
        println!("[reports_missing_right_paren] erro esperado por ')' ausente  ✓");
    }

    // ── testes de statements (PARSER-01) ────────────────────────────────────

    /// `return x;` produz Stmt::Return com expressão Ident("x").
    #[test]
    fn parses_return_with_value() {
        // return x ;  EOF
        let tokens = vec![
            tk(TokenKind::Return, 1),
            ident("x", 8),
            tk(TokenKind::Semicolon, 9),
            eof(10),
        ];

        let mut parser = Parser::new(tokens);
        let stmt = parse_stmt(&mut parser).expect("return válido");
        println!("[parses_return_with_value] AST: {stmt:#?}");

        let Stmt::Return(Some(expr), _) = stmt else {
            panic!("esperava Stmt::Return com valor");
        };
        println!("[parses_return_with_value] expr = {expr:?}");
        assert!(matches!(expr, Expr::Ident(ref s, _) if s == "x"));
        println!("[parses_return_with_value] Stmt::Return(Ident(\"x\"))  ✓");
    }

    /// `return;` produz Stmt::Return sem valor.
    #[test]
    fn parses_return_without_value() {
        let tokens = vec![
            tk(TokenKind::Return, 1),
            tk(TokenKind::Semicolon, 7),
            eof(8),
        ];

        let mut parser = Parser::new(tokens);
        let stmt = parse_stmt(&mut parser).expect("return vazio válido");
        println!("[parses_return_without_value] AST: {stmt:#?}");

        assert!(matches!(stmt, Stmt::Return(None, _)));
        println!("[parses_return_without_value] Stmt::Return(None)  ✓");
    }

    /// `break;` produz Stmt::Break.
    #[test]
    fn parses_break_statement() {
        let tokens = vec![tk(TokenKind::Break, 1), tk(TokenKind::Semicolon, 6), eof(7)];

        let mut parser = Parser::new(tokens);
        let stmt = parse_stmt(&mut parser).expect("break válido");
        println!("[parses_break_statement] AST: {stmt:#?}");

        assert!(matches!(stmt, Stmt::Break(_)));
        println!("[parses_break_statement] Stmt::Break  ✓");
    }

    /// `continue;` produz Stmt::Continue.
    #[test]
    fn parses_continue_statement() {
        let tokens = vec![
            tk(TokenKind::Continue, 1),
            tk(TokenKind::Semicolon, 9),
            eof(10),
        ];

        let mut parser = Parser::new(tokens);
        let stmt = parse_stmt(&mut parser).expect("continue válido");
        println!("[parses_continue_statement] AST: {stmt:#?}");

        assert!(matches!(stmt, Stmt::Continue(_)));
        println!("[parses_continue_statement] Stmt::Continue  ✓");
    }

    /// `x++;` produz Stmt::ExprStmt com Expr::Postfix.
    #[test]
    fn parses_expr_stmt_postfix_inc() {
        let tokens = vec![
            ident("x", 1),
            tk(TokenKind::PlusPlus, 2),
            tk(TokenKind::Semicolon, 4),
            eof(5),
        ];

        let mut parser = Parser::new(tokens);
        let stmt = parse_stmt(&mut parser).expect("expr stmt válido");
        println!("[parses_expr_stmt_postfix_inc] AST: {stmt:#?}");

        let Stmt::ExprStmt(expr, _) = stmt else {
            panic!("esperava ExprStmt");
        };
        println!("[parses_expr_stmt_postfix_inc] expr = {expr:?}");
        assert!(matches!(expr, Expr::Postfix(PostfixOp::Inc, _, _)));
        println!("[parses_expr_stmt_postfix_inc] ExprStmt(Postfix::Inc)  ✓");
    }

    /// Bloco vazio `{}` produz Stmt::Block com zero statements.
    #[test]
    fn parses_empty_block() {
        let tokens = vec![
            tk(TokenKind::LeftBrace, 1),
            tk(TokenKind::RightBrace, 2),
            eof(3),
        ];

        let mut parser = Parser::new(tokens);
        let stmt = parse_stmt(&mut parser).expect("bloco vazio válido");
        println!("[parses_empty_block] AST: {stmt:#?}");

        let Stmt::Block(stmts, _) = stmt else {
            panic!("esperava Block");
        };
        println!("[parses_empty_block] Block com {} statements", stmts.len());
        assert!(stmts.is_empty());
        println!("[parses_empty_block] Block vazio  ✓");
    }

    /// Bloco `{ return x; }` — reproduz o trecho de full_code1.c coberto por PARSER-01.
    /// full_code1.c: `if (x > 0) { return x; }` — o corpo do if é um bloco com return.
    #[test]
    fn parses_block_with_return_from_full_code1() {
        // { return x ; }  EOF
        let tokens = vec![
            tk(TokenKind::LeftBrace, 1),
            tk(TokenKind::Return, 5),
            ident("x", 12),
            tk(TokenKind::Semicolon, 13),
            tk(TokenKind::RightBrace, 5),
            eof(6),
        ];

        let mut parser = Parser::new(tokens);
        let stmt = parse_stmt(&mut parser).expect("bloco com return válido");
        println!("[parses_block_with_return_from_full_code1] AST: {stmt:#?}");

        let Stmt::Block(stmts, _) = stmt else {
            panic!("esperava Block");
        };
        println!("[parses_block_with_return_from_full_code1] Block com {} statements", stmts.len());
        assert_eq!(stmts.len(), 1);
        println!("[parses_block_with_return_from_full_code1] stmts[0] = {:?}", stmts[0]);
        assert!(matches!(stmts[0], Stmt::Return(Some(_), _)));
        println!("[parses_block_with_return_from_full_code1] Block {{ Return(x) }}  ✓");
    }

    /// break sem ponto-e-vírgula deve gerar erro sintático.
    #[test]
    fn rejects_break_without_semicolon() {
        let tokens = vec![tk(TokenKind::Break, 1), eof(6)];

        let mut parser = Parser::new(tokens);
        let result = parse_stmt(&mut parser);
        println!("[rejects_break_without_semicolon] resultado: {result:?}");
        assert!(result.is_err());
        println!("[rejects_break_without_semicolon] erro esperado por ';' ausente após break  ✓");
    }

    /// continue sem ponto-e-vírgula deve gerar erro sintático.
    #[test]
    fn rejects_continue_without_semicolon() {
        let tokens = vec![tk(TokenKind::Continue, 1), eof(9)];

        let mut parser = Parser::new(tokens);
        let result = parse_stmt(&mut parser);
        println!("[rejects_continue_without_semicolon] resultado: {result:?}");
        assert!(result.is_err());
        println!("[rejects_continue_without_semicolon] erro esperado por ';' ausente após continue  ✓");
    }

    /// Bloco sem fechar `}` deve gerar erro sintático.
    #[test]
    fn rejects_unclosed_block() {
        let tokens = vec![
            tk(TokenKind::LeftBrace, 1),
            tk(TokenKind::Return, 3),
            tk(TokenKind::Semicolon, 9),
            eof(10),
        ];

        let mut parser = Parser::new(tokens);
        let result = parse_stmt(&mut parser);
        println!("[rejects_unclosed_block] resultado: {result:?}");
        assert!(result.is_err());
        println!("[rejects_unclosed_block] erro esperado por '}}' ausente  ✓");
    }
}
