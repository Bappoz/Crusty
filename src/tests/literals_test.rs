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
    fn unterminated_char_literal() {
        let scanner = scan_source("'a");
        let found = scanner
            .diagnostics
            .iter()
            .any(|e| format!("{:?}", e).contains("UnterminatedLiteral"));
        assert!(found, "Deveria detectar char não terminado");
    }
}
