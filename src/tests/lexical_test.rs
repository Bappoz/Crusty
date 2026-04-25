#[cfg(test)]
mod tests {
    use crate::common::input::source::SourceFile;
    use crate::lexer::scanner::Scanner;
    use crate::lexer::tokens::token_kind::TokenKind;

    // Helper que tokeniza uma string e devolve só os kinds, sem o Eof.
    // Evita repetição nos testes — cada teste foca só no que importa.
    fn scan(input: &str) -> Vec<TokenKind> {
        let src = SourceFile::from_string(input);
        let mut scanner = Scanner::new(src);
        scanner
            .scan()
            .iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| t.kind.clone())
            .collect()
    }

    // --- Inteiros ---

    #[test]
    fn lex_integer_literals() {
        assert_eq!(scan("42"), vec![TokenKind::IntLiteral(42)]);
        assert_eq!(scan("0"), vec![TokenKind::IntLiteral(0)]);
        assert_eq!(scan("0xFF"), vec![TokenKind::IntLiteral(255)]);
        assert_eq!(scan("0755"), vec![TokenKind::IntLiteral(493)]);
    }

    // --- Floats ---

    #[test]
    fn lex_float_literals() {
        assert_eq!(scan("3.14"), vec![TokenKind::FloatLiteral(3.14)]);
        assert_eq!(scan("1e10"), vec![TokenKind::FloatLiteral(1e10)]);
        assert_eq!(scan("2.5e-3"), vec![TokenKind::FloatLiteral(2.5e-3)]);
    }

    // --- Strings ---

    #[test]
    fn lex_string_with_escapes() {
        assert_eq!(
            scan(r#""hello""#),
            vec![TokenKind::StringLiteral("hello".into())]
        );
        assert_eq!(
            scan(r#""hello\nworld""#),
            vec![TokenKind::StringLiteral("hello\nworld".into())]
        );
        assert_eq!(
            scan(r#""say \"hi\"""#),
            vec![TokenKind::StringLiteral("say \"hi\"".into())]
        );
    }

    // --- Chars ---

    #[test]
    fn lex_char_literals() {
        assert_eq!(scan("'a'"), vec![TokenKind::CharLiteral('a')]);
        assert_eq!(scan("'\\n'"), vec![TokenKind::CharLiteral('\n')]);
        assert_eq!(scan("'\\t'"), vec![TokenKind::CharLiteral('\t')]);
    }

    // --- Keywords vs identificadores ---

    #[test]
    fn keywords_not_treated_as_idents() {
        assert_eq!(scan("if"), vec![TokenKind::If]);
        assert_eq!(scan("while"), vec![TokenKind::While]);
        assert_eq!(scan("int"), vec![TokenKind::Int]);
        assert_eq!(scan("void"), vec![TokenKind::Void]);
    }

    #[test]
    fn keyword_prefix_is_identifier() {
        // "iffy" começa com "if" mas é identificador
        assert_eq!(scan("iffy"), vec![TokenKind::Identifier("iffy".into())]);
        assert_eq!(scan("forte"), vec![TokenKind::Identifier("forte".into())]);
        assert_eq!(scan("_count"), vec![TokenKind::Identifier("_count".into())]);
    }

    // --- Char inválido ---

    #[test]
    fn unknown_char_emits_diagnostic() {
        let src = SourceFile::from_string("@");
        let mut scanner = Scanner::new(src);
        scanner.scan();

        // Deve ter exatamente um diagnóstico
        assert_eq!(scanner.diagnostics.len(), 1);

        // O token emitido deve ser Unknown com o char problemático
        assert_eq!(scanner.tokens[0].kind, TokenKind::Unknown('@'));
    }

    #[test]
    fn multiple_unknowns_keep_scanning() {
        // O scanner não para no primeiro erro
        let src = SourceFile::from_string("@ ? $");
        let mut scanner = Scanner::new(src);
        scanner.scan();
        assert_eq!(scanner.diagnostics.len(), 3);
    }

    // --- Comentários e Diretivas são ignorados ---

    #[test]
    fn line_comments_are_skipped() {
        let kinds = scan("42 // isso é um comentário\n99");
        assert_eq!(
            kinds,
            vec![TokenKind::IntLiteral(42), TokenKind::IntLiteral(99),]
        );
    }

    #[test]
    fn preprocessor_directives_are_skipped() {
        let kinds = scan("#include <stdio.h>\n#define MAX 10\n42");
        assert_eq!(kinds, vec![TokenKind::IntLiteral(42)]);
    }
}
