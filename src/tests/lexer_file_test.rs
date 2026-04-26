#[cfg(test)]
mod tests {
    use crate::common::input::source::SourceFile;
    use crate::lexer::scanner::Scanner;
    use crate::lexer::tokens::token_kind::TokenKind;
    use std::path::PathBuf;

    fn lex_file(name: &str) -> Scanner {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/examples")
            .join(name);
        let src = SourceFile::from_path(path)
            .unwrap_or_else(|e| panic!("failed to read '{}': {:?}", name, e.to_report()));
        let mut scanner = Scanner::new(src);
        scanner.scan();
        scanner
    }

    fn kinds(scanner: &Scanner) -> Vec<TokenKind> {
        scanner
            .tokens
            .iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| t.kind.clone())
            .collect()
    }

    // --- hello_world.c ---

    #[test]
    fn hello_world_token_sequence_matches_snapshot() {
        let scanner = lex_file("hello_world.c");
        assert_eq!(scanner.diagnostics.len(), 0, "esperado zero erros lexicais");

        #[rustfmt::skip]
        let expected = vec![
            TokenKind::Int,
            TokenKind::Identifier("main".into()),
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::Identifier("printf".into()),
            TokenKind::LeftParen,
            TokenKind::StringLiteral("Hello, World!\n".into()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::Return,
            TokenKind::IntLiteral(0),
            TokenKind::Semicolon,
            TokenKind::RightBrace,
        ];
        assert_eq!(kinds(&scanner), expected);
    }

    #[test]
    fn hello_world_int_keyword_at_line3_col1() {
        let scanner = lex_file("hello_world.c");
        // linha 1: #include (diretiva ignorada), linha 2: vazia, linha 3: int main()
        let first = scanner
            .tokens
            .iter()
            .find(|t| t.kind != TokenKind::Eof)
            .unwrap();
        assert_eq!(first.kind, TokenKind::Int);
        assert_eq!(first.line, 3, "esperado 'int' na linha 3");
        assert_eq!(first.col, 1, "esperado 'int' na coluna 1");
    }

    // --- declarations.c ---

    #[test]
    fn declarations_token_sequence_matches_snapshot() {
        let scanner = lex_file("declarations.c");
        assert_eq!(scanner.diagnostics.len(), 0, "esperado zero erros lexicais");

        #[rustfmt::skip]
        let expected = vec![
            // int x = 42;
            TokenKind::Int, TokenKind::Identifier("x".into()), TokenKind::Equal, TokenKind::IntLiteral(42), TokenKind::Semicolon,
            // float y = 3.14f;
            TokenKind::Float, TokenKind::Identifier("y".into()), TokenKind::Equal, TokenKind::FloatLiteral(3.14), TokenKind::Semicolon,
            // char c = 'A';
            TokenKind::Char, TokenKind::Identifier("c".into()), TokenKind::Equal, TokenKind::CharLiteral('A'), TokenKind::Semicolon,
            // unsigned long ul = 0xFFUL;
            TokenKind::Unsigned, TokenKind::Long, TokenKind::Identifier("ul".into()), TokenKind::Equal, TokenKind::IntLiteral(255), TokenKind::Semicolon,
            // const int MAX = 100;
            TokenKind::Const, TokenKind::Int, TokenKind::Identifier("MAX".into()), TokenKind::Equal, TokenKind::IntLiteral(100), TokenKind::Semicolon,
        ];
        assert_eq!(kinds(&scanner), expected);
    }

    #[test]
    fn declarations_literal_spans_cover_full_lexeme() {
        let scanner = lex_file("declarations.c");
        let src = scanner.src.source.as_str().to_owned();

        // 3.14f deve ter span cobrindo os 5 chars "3.14f"
        let float_tok = scanner
            .tokens
            .iter()
            .find(|t| matches!(t.kind, TokenKind::FloatLiteral(_)))
            .expect("FloatLiteral não encontrado");
        let lexeme = &src[float_tok.span.start..float_tok.span.end];
        assert_eq!(lexeme, "3.14f");

        // 0xFFUL deve ter span cobrindo "0xFFUL"
        let int_tok = scanner
            .tokens
            .iter()
            .find(|t| matches!(&t.kind, TokenKind::IntLiteral(255)))
            .expect("IntLiteral(255) não encontrado");
        let lexeme = &src[int_tok.span.start..int_tok.span.end];
        assert_eq!(lexeme, "0xFFUL");
    }

    // --- operators.c ---

    #[test]
    fn operators_token_sequence_matches_snapshot() {
        let scanner = lex_file("operators.c");
        assert_eq!(scanner.diagnostics.len(), 0, "esperado zero erros lexicais");

        #[rustfmt::skip]
        let expected = vec![
            // int a = 1 + 2;
            TokenKind::Int, TokenKind::Identifier("a".into()), TokenKind::Equal, TokenKind::IntLiteral(1), TokenKind::Plus, TokenKind::IntLiteral(2), TokenKind::Semicolon,
            // int b = a - 1;
            TokenKind::Int, TokenKind::Identifier("b".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::Minus, TokenKind::IntLiteral(1), TokenKind::Semicolon,
            // a += 1;
            TokenKind::Identifier("a".into()), TokenKind::PlusEqual, TokenKind::IntLiteral(1), TokenKind::Semicolon,
            // b -= 1;
            TokenKind::Identifier("b".into()), TokenKind::MinusEqual, TokenKind::IntLiteral(1), TokenKind::Semicolon,
            // a++;
            TokenKind::Identifier("a".into()), TokenKind::PlusPlus, TokenKind::Semicolon,
            // b--;
            TokenKind::Identifier("b".into()), TokenKind::MinusMinus, TokenKind::Semicolon,
            // int c = a == b;
            TokenKind::Int, TokenKind::Identifier("c".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::EqualEqual, TokenKind::Identifier("b".into()), TokenKind::Semicolon,
            // int d = a != b;
            TokenKind::Int, TokenKind::Identifier("d".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::BangEqual, TokenKind::Identifier("b".into()), TokenKind::Semicolon,
            // int e = a <= b;
            TokenKind::Int, TokenKind::Identifier("e".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::LessEqual, TokenKind::Identifier("b".into()), TokenKind::Semicolon,
            // int f = a >= b;
            TokenKind::Int, TokenKind::Identifier("f".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::GreaterEqual, TokenKind::Identifier("b".into()), TokenKind::Semicolon,
            // int g = a && b;
            TokenKind::Int, TokenKind::Identifier("g".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::AndAnd, TokenKind::Identifier("b".into()), TokenKind::Semicolon,
            // int h = a || b;
            TokenKind::Int, TokenKind::Identifier("h".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::OrOr, TokenKind::Identifier("b".into()), TokenKind::Semicolon,
            // int i = a << 1;
            TokenKind::Int, TokenKind::Identifier("i".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::LessLess, TokenKind::IntLiteral(1), TokenKind::Semicolon,
            // int j = a >> 1;
            TokenKind::Int, TokenKind::Identifier("j".into()), TokenKind::Equal, TokenKind::Identifier("a".into()), TokenKind::GreaterGreater, TokenKind::IntLiteral(1), TokenKind::Semicolon,
            // i <<= 2;
            TokenKind::Identifier("i".into()), TokenKind::LessLessEqual, TokenKind::IntLiteral(2), TokenKind::Semicolon,
            // j >>= 2;
            TokenKind::Identifier("j".into()), TokenKind::GreaterGreaterEqual, TokenKind::IntLiteral(2), TokenKind::Semicolon,
        ];
        assert_eq!(kinds(&scanner), expected);
    }

    // --- invalid_char.c ---

    #[test]
    fn invalid_char_produces_diagnostic_not_panic() {
        let scanner = lex_file("invalid_char.c");
        assert_eq!(scanner.diagnostics.len(), 2, "esperado 2 erros: '@' e '$'");
    }

    #[test]
    fn invalid_char_scanner_recovers_and_emits_valid_tokens() {
        let scanner = lex_file("invalid_char.c");
        let ks = kinds(&scanner);

        // tokens válidos ainda aparecem apesar dos erros
        assert!(ks.contains(&TokenKind::IntLiteral(42)));
        assert!(ks.contains(&TokenKind::Unknown('@')));
        assert!(ks.contains(&TokenKind::Unknown('$')));

        // identificadores ao redor dos erros também foram emitidos
        assert!(ks.contains(&TokenKind::Identifier("x".into())));
        assert!(ks.contains(&TokenKind::Identifier("y".into())));
        assert!(ks.contains(&TokenKind::Identifier("z".into())));
    }
}
