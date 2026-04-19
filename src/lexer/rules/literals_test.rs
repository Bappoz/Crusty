#[cfg(test)]
mod tests {
    use crate::lexer::scanner::Scanner;
    use crate::common::input::source::SourceFile;
    use crate::lexer::rules::LiteralsRules;
    use crate::lexer::tokens::TokenKind;

    fn scan_source(src: &str) -> Scanner {
        let source = SourceFile::new("<test>", src);
        let mut scanner = Scanner::new(source);
        scanner.scan();
        scanner
    }

    #[test]
    fn unterminated_string_literal() {
        let mut scanner = scan_source("\"unterminated");
        let found = scanner.diagnostics.iter().any(|e| format!("{:?}", e).contains("unterminated literal"));
        assert!(found, "Deveria detectar string não terminada");
    }

    #[test]
    fn unterminated_char_literal() {
        let mut scanner = scan_source("'a");
        let found = scanner.diagnostics.iter().any(|e| format!("{:?}", e).contains("unterminated literal"));
        assert!(found, "Deveria detectar char não terminado");
    }
}
