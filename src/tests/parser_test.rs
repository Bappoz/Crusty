#[cfg(test)]
mod tests {
    use crate::common::ast::ast::{QualifierType, Type};
    use crate::common::ast::expr::{BinOp, Expr, Literal, PostfixOp, PrefixOp};
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
                ("neg", Expr::Unary(_, _, _)) => {
                    println!("[parses_prefix_operators] '-x' é Unary  ✓")
                }
                ("not", Expr::Unary(_, _, _)) => {
                    println!("[parses_prefix_operators] '!x' é Unary  ✓")
                }
                ("inc", Expr::Prefix(PrefixOp::Inc, _, _)) => {
                    println!("[parses_prefix_operators] '++x' é Prefix::Inc  ✓")
                }
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
        println!(
            "[parses_postfix_operators] 'f(1,2)' é Call com {} args  ✓",
            args.len()
        );
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
        let stmt = parser.parse_stmt().expect("return válido");
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
        let stmt = parser.parse_stmt().expect("return vazio válido");
        println!("[parses_return_without_value] AST: {stmt:#?}");

        assert!(matches!(stmt, Stmt::Return(None, _)));
        println!("[parses_return_without_value] Stmt::Return(None)  ✓");
    }

    /// `break;` produz Stmt::Break.
    #[test]
    fn parses_break_statement() {
        let tokens = vec![tk(TokenKind::Break, 1), tk(TokenKind::Semicolon, 6), eof(7)];

        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("break válido");
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
        let stmt = parser.parse_stmt().expect("continue válido");
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
        let stmt = parser.parse_stmt().expect("expr stmt válido");
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
        let stmt = parser.parse_stmt().expect("bloco vazio válido");
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
        let stmt = parser.parse_stmt().expect("bloco com return válido");
        println!("[parses_block_with_return_from_full_code1] AST: {stmt:#?}");

        let Stmt::Block(stmts, _) = stmt else {
            panic!("esperava Block");
        };
        println!(
            "[parses_block_with_return_from_full_code1] Block com {} statements",
            stmts.len()
        );
        assert_eq!(stmts.len(), 1);
        println!(
            "[parses_block_with_return_from_full_code1] stmts[0] = {:?}",
            stmts[0]
        );
        assert!(matches!(stmts[0], Stmt::Return(Some(_), _)));
        println!("[parses_block_with_return_from_full_code1] Block {{ Return(x) }}  ✓");
    }

    /// break sem ponto-e-vírgula deve gerar erro sintático.
    #[test]
    fn rejects_break_without_semicolon() {
        let tokens = vec![tk(TokenKind::Break, 1), eof(6)];

        let mut parser = Parser::new(tokens);
        let result = parser.parse_stmt();
        println!("[rejects_break_without_semicolon] resultado: {result:?}");
        assert!(result.is_err());
        println!("[rejects_break_without_semicolon] erro esperado por ';' ausente após break  ✓");
    }

    /// continue sem ponto-e-vírgula deve gerar erro sintático.
    #[test]
    fn rejects_continue_without_semicolon() {
        let tokens = vec![tk(TokenKind::Continue, 1), eof(9)];

        let mut parser = Parser::new(tokens);
        let result = parser.parse_stmt();
        println!("[rejects_continue_without_semicolon] resultado: {result:?}");
        assert!(result.is_err());
        println!(
            "[rejects_continue_without_semicolon] erro esperado por ';' ausente após continue  ✓"
        );
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
        let result = parser.parse_stmt();
        println!("[rejects_unclosed_block] resultado: {result:?}");
        assert!(result.is_err());
        println!("[rejects_unclosed_block] erro esperado por '}}' ausente  ✓");
    }

    #[test]
    fn parses_var_decl_with_init() {
        // int x = 42;
        let tokens = vec![
            tk(TokenKind::Int, 1),
            ident("x", 5),
            tk(TokenKind::Equal, 7),
            int(42, 9),
            tk(TokenKind::Semicolon, 11),
            eof(12),
        ];
        let stmt = Parser::new(tokens).parse_stmt().expect("decl válida");
        let Stmt::VarDecl(qty, name, Some(init), _) = stmt else {
            panic!("esperava VarDecl com init");
        };
        assert!(matches!(qty.ty, Type::Int));
        assert_eq!(name, "x");
        assert!(matches!(init, Expr::Literal(Literal::Int(42), _)));
    }

    #[test]
    fn parses_var_decl_no_init() {
        // float y;
        let tokens = vec![
            tk(TokenKind::Float, 1),
            ident("y", 7),
            tk(TokenKind::Semicolon, 8),
            eof(9),
        ];
        let stmt = Parser::new(tokens).parse_stmt().expect("decl válida");
        let Stmt::VarDecl(qty, name, None, _) = stmt else {
            panic!("esperava VarDecl sem init");
        };
        assert!(matches!(qty.ty, Type::Float));
        assert_eq!(name, "y");
    }

    #[test]
    fn parses_expr_stmt() {
        // x = 1;
        let tokens = vec![
            ident("x", 1),
            tk(TokenKind::Equal, 3),
            int(1, 5),
            tk(TokenKind::Semicolon, 6),
            eof(7),
        ];
        let stmt = Parser::new(tokens).parse_stmt().expect("expr stmt válido");
        let Stmt::ExprStmt(expr, _) = stmt else {
            panic!("esperava ExprStmt");
        };
        let Expr::Assign(lhs, rhs, _) = expr else {
            panic!("esperava atribuição dentro do ExprStmt");
        };
        assert!(matches!(*lhs, Expr::Ident(name, _) if name == "x"));
        assert!(matches!(*rhs, Expr::Literal(Literal::Int(1), _)));
    }

    #[test]
    fn parses_const_pointer_var_decl() {
        // const int *p = 0;
        let tokens = vec![
            tk(TokenKind::Const, 1),
            tk(TokenKind::Int, 7),
            tk(TokenKind::Star, 11),
            ident("p", 12),
            tk(TokenKind::Equal, 14),
            int(0, 16),
            tk(TokenKind::Semicolon, 17),
            eof(18),
        ];
        let stmt = Parser::new(tokens).parse_stmt().expect("decl válida");
        let Stmt::VarDecl(qty, name, Some(_), _) = stmt else {
            panic!("esperava VarDecl");
        };
        assert!(qty.is_const);
        assert!(matches!(qty.ty, Type::Pointer(_)));
        assert_eq!(name, "p");
    }

    #[test]
    fn parses_if_without_else() {
        // if (x > 0) return x;
        let tokens = vec![
            tk(TokenKind::If, 1),
            tk(TokenKind::LeftParen, 4),
            ident("x", 5),
            tk(TokenKind::Greater, 7),
            int(0, 9),
            tk(TokenKind::RightParen, 10),
            tk(TokenKind::Return, 12),
            ident("x", 19),
            tk(TokenKind::Semicolon, 20),
            eof(21),
        ];
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("if válido");
        let Stmt::If(cond, then_branch, None, _) = stmt else {
            panic!("esperava Stmt::If sem else");
        };
        assert!(matches!(cond, Expr::Binary(_, BinOp::Greater, _, _)));
        let Stmt::Return(Some(ret_expr), _) = *then_branch else {
            panic!("esperava return no then-branch");
        };
        assert!(matches!(ret_expr, Expr::Ident(name, _) if name == "x"));
    }

    #[test]
    fn parses_if_with_else() {
        // if (x) return 1; else return 0;
        let tokens = vec![
            tk(TokenKind::If, 1),
            tk(TokenKind::LeftParen, 4),
            ident("x", 5),
            tk(TokenKind::RightParen, 6),
            tk(TokenKind::Return, 8),
            int(1, 15),
            tk(TokenKind::Semicolon, 16),
            tk(TokenKind::Else, 18),
            tk(TokenKind::Return, 23),
            int(0, 30),
            tk(TokenKind::Semicolon, 31),
            eof(32),
        ];
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("if-else válido");
        let Stmt::If(cond, _, Some(else_branch), _) = stmt else {
            panic!("esperava Stmt::If com else");
        };
        assert!(matches!(cond, Expr::Ident(name, _) if name == "x"));
        let Stmt::Return(Some(ret_expr), _) = *else_branch else {
            panic!("esperava return no else-branch");
        };
        assert!(matches!(ret_expr, Expr::Literal(Literal::Int(0), _)));
    }

    #[test]
    fn parses_if_else_if_chain() {
        // if (a) return 1; else if (b) return 2;
        let tokens = vec![
            tk(TokenKind::If, 1),
            tk(TokenKind::LeftParen, 4),
            ident("a", 5),
            tk(TokenKind::RightParen, 6),
            tk(TokenKind::Return, 8),
            int(1, 15),
            tk(TokenKind::Semicolon, 16),
            tk(TokenKind::Else, 18),
            tk(TokenKind::If, 23),
            tk(TokenKind::LeftParen, 26),
            ident("b", 27),
            tk(TokenKind::RightParen, 28),
            tk(TokenKind::Return, 30),
            int(2, 37),
            tk(TokenKind::Semicolon, 38),
            eof(39),
        ];
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("if-else-if válido");
        let Stmt::If(_, _, Some(else_branch), _) = stmt else {
            panic!("esperava Stmt::If com else");
        };
        let Stmt::If(inner_cond, _, None, _) = *else_branch else {
            panic!("esperava Stmt::If aninhado no else");
        };
        assert!(matches!(inner_cond, Expr::Ident(name, _) if name == "b"));
    }

    #[test]
    fn parses_if_with_block() {
        // if (x > 0) { return x; }
        let tokens = vec![
            tk(TokenKind::If, 1),
            tk(TokenKind::LeftParen, 4),
            ident("x", 5),
            tk(TokenKind::Greater, 7),
            int(0, 9),
            tk(TokenKind::RightParen, 10),
            tk(TokenKind::LeftBrace, 12),
            tk(TokenKind::Return, 14),
            ident("x", 21),
            tk(TokenKind::Semicolon, 22),
            tk(TokenKind::RightBrace, 24),
            eof(25),
        ];
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("if com bloco válido");
        let Stmt::If(cond, then_branch, None, _) = stmt else {
            panic!("esperava Stmt::If sem else");
        };
        assert!(matches!(cond, Expr::Binary(_, BinOp::Greater, _, _)));
        let Stmt::Block(stmts, _) = *then_branch else {
            panic!("esperava bloco no then-branch");
        };
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parses_while_stmt() {
        // while (x > 0) x = x - 1;
        let tokens = vec![
            tk(TokenKind::While, 1),
            tk(TokenKind::LeftParen, 7),
            ident("x", 8),
            tk(TokenKind::Greater, 10),
            int(0, 12),
            tk(TokenKind::RightParen, 13),
            ident("x", 15),
            tk(TokenKind::Equal, 17),
            ident("x", 19),
            tk(TokenKind::Minus, 21),
            int(1, 23),
            tk(TokenKind::Semicolon, 24),
            eof(25),
        ];
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("while válido");
        let Stmt::While(cond, body, _) = stmt else {
            panic!("esperava Stmt::While");
        };
        assert!(matches!(cond, Expr::Binary(_, BinOp::Greater, _, _)));
        let Stmt::ExprStmt(Expr::Assign(_, _, _), _) = *body else {
            panic!("esperava ExprStmt com atribuição no corpo do while");
        };
    }

    #[test]
    fn parses_while_with_block() {
        // while (1) { break; }
        let tokens = vec![
            tk(TokenKind::While, 1),
            tk(TokenKind::LeftParen, 7),
            int(1, 8),
            tk(TokenKind::RightParen, 9),
            tk(TokenKind::LeftBrace, 11),
            tk(TokenKind::Break, 13),
            tk(TokenKind::Semicolon, 18),
            tk(TokenKind::RightBrace, 20),
            eof(21),
        ];
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("while com bloco válido");
        let Stmt::While(cond, body, _) = stmt else {
            panic!("esperava Stmt::While");
        };
        assert!(matches!(cond, Expr::Literal(Literal::Int(1), _)));
        let Stmt::Block(stmts, _) = *body else {
            panic!("esperava bloco no corpo do while");
        };
        assert!(matches!(&stmts[0], Stmt::Break(_)));
    }

    #[test]
    fn parses_do_while_stmt() {
        // do x = x + 1; while (x < 10);
        let tokens = vec![
            tk(TokenKind::Do, 1),
            ident("x", 4),
            tk(TokenKind::Equal, 6),
            ident("x", 8),
            tk(TokenKind::Plus, 10),
            int(1, 12),
            tk(TokenKind::Semicolon, 13),
            tk(TokenKind::While, 15),
            tk(TokenKind::LeftParen, 21),
            ident("x", 22),
            tk(TokenKind::Less, 24),
            int(10, 26),
            tk(TokenKind::RightParen, 28),
            tk(TokenKind::Semicolon, 29),
            eof(30),
        ];
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("do-while válido");
        let Stmt::DoWhile(body, cond, _) = stmt else {
            panic!("esperava Stmt::DoWhile");
        };
        let Stmt::ExprStmt(Expr::Assign(_, _, _), _) = *body else {
            panic!("esperava ExprStmt com atribuição no corpo do do-while");
        };
        assert!(matches!(cond, Expr::Binary(_, BinOp::Less, _, _)));
    }

    #[test]
    fn parses_do_while_with_block() {
        // do { break; } while (1);
        let tokens = vec![
            tk(TokenKind::Do, 1),
            tk(TokenKind::LeftBrace, 4),
            tk(TokenKind::Break, 6),
            tk(TokenKind::Semicolon, 11),
            tk(TokenKind::RightBrace, 13),
            tk(TokenKind::While, 15),
            tk(TokenKind::LeftParen, 21),
            int(1, 22),
            tk(TokenKind::RightParen, 23),
            tk(TokenKind::Semicolon, 24),
            eof(25),
        ];
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().expect("do-while com bloco válido");
        let Stmt::DoWhile(body, cond, _) = stmt else {
            panic!("esperava Stmt::DoWhile");
        };
        let Stmt::Block(stmts, _) = *body else {
            panic!("esperava bloco no corpo do do-while");
        };
        assert!(matches!(&stmts[0], Stmt::Break(_)));
        assert!(matches!(cond, Expr::Literal(Literal::Int(1), _)));
    }
}
