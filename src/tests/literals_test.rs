#[cfg(test)]
mod tests {
    use crate::common::input::source::SourceFile;
    use crate::lexer::scanner::Scanner;

    fn scan_source(src: &str) -> Scanner {
        let source = SourceFile::from_string(src);
        let mut scanner = Scanner::new(source);
        scanner.scan();
        scanner
    }

    #[test]
    fn unterminated_string_literal() {
        let scanner = scan_source("\"unterminated");
        let found = scanner
            .diagnostics
            .iter()
            .any(|e| format!("{:?}", e).contains("UnterminatedLiteral"));
        assert!(found, "Deveria detectar string não terminada");
    }

    #[test]
    fn string_with_real_newline_should_error() {
        // Testa string com newline real (não \n)
        let scanner = scan_source("\"hello\nworld\"");
        assert!(
            scanner.diagnostics.len() >= 1,
            "Deveria detectar UnterminatedLiteral para string com newline real"
        );
        let found = scanner
            .diagnostics
            .iter()
            .any(|e| format!("{:?}", e).contains("UnterminatedLiteral"));
        assert!(found, "Diagnostico deve ser UnterminatedLiteral");
    }

    #[test]
    fn unterminated_char_literal() {
        let scanner = scan_source("'a");
        let found = scanner
            .diagnostics
            .iter()
            .any(|e| format!("{:?}", e).contains("UnterminatedLiteral"));
        assert!(found, "Deveria detectar char não terminado");
    }

    #[test]
    fn empty_char_literal() {
        let scanner = scan_source("''");
        assert!(
            scanner.diagnostics.len() >= 1,
            "Deveria detectar char literal vazio"
        );
        let found = scanner
            .diagnostics
            .iter()
            .any(|e| format!("{:?}", e).contains("UnterminatedLiteral"));
        assert!(found, "Diagnostico deve ser UnterminatedLiteral");
    }
}
