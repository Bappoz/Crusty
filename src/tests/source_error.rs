use crate::common::input::source::SourceFile;
use std::{io::Write, path::PathBuf};

fn src(input: &str) -> SourceFile {
    SourceFile::from_string(input)
}

#[test]
fn peek_does_not_consume() {
    let sf = src("abc");

    assert_eq!(sf.peek(), Some('a'));
    assert_eq!(sf.peek(), Some('a'));
    assert_eq!(sf.peek(), Some('a'));
    assert_eq!(sf.pos, 0); // pos é pub(crate), visível nos testes
}

#[test]
fn advance_consumes_char() {
    let mut sf = src("abc");

    assert_eq!(sf.advance(), Some('a'));
    assert_eq!(sf.advance(), Some('b'));
    assert_eq!(sf.advance(), Some('c'));
}

#[test]
fn peek_after_advance_returns_next() {
    let mut sf = src("ab");

    sf.advance();
    assert_eq!(sf.peek(), Some('b'));
}

#[test]
fn eof_returns_none() {
    let mut sf = src("");

    assert_eq!(sf.peek(), None);
    assert_eq!(sf.advance(), None);
    assert_eq!(sf.advance(), None); // não panics em chamadas repetidas
}

#[test]
fn advances_line_on_newline() {
    let mut sf = src("a\nb");

    sf.advance();
    assert_eq!(sf.current_pos(), (1, 2));

    sf.advance(); // '\n'
    assert_eq!(sf.current_pos(), (2, 1));

    sf.advance();
    assert_eq!(sf.current_pos(), (2, 2));
}

#[test]
fn col_increments_on_same_line() {
    let mut sf = src("hello");

    for expected_col in 2..=6 {
        sf.advance();
        let (_, col) = sf.current_pos();
        assert_eq!(col, expected_col);
    }
}

#[test]
fn multiple_newlines_tracked_correctly() {
    let mut sf = src("a\n\n\nb");

    sf.advance(); // 'a'
    sf.advance(); // '\n' → linha 2
    sf.advance(); // '\n' → linha 3
    sf.advance(); // '\n' → linha 4
    sf.advance(); // 'b'

    assert_eq!(sf.current_pos(), (4, 2));
}

#[test]
fn initial_position_is_line1_col1() {
    let sf = src("anything");

    assert_eq!(sf.current_pos(), (1, 1));
}

#[test]
fn line_and_col_getters_match_current_pos() {
    let mut sf = src("a\nb");

    sf.advance(); // 'a'
    sf.advance(); // '\n'
    sf.advance(); // 'b'

    let (line, col) = sf.current_pos();
    assert_eq!(sf.line(), line);
    assert_eq!(sf.col(), col);
}

#[test]
fn set_line_and_col_update_position() {
    let mut sf = src("abc");

    sf.set_line(10);
    sf.set_col(5);

    assert_eq!(sf.current_pos(), (10, 5));
}

// Teste de integração — único que toca disco
#[test]
fn from_path_reads_file_correctly() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "int main() {{}}").unwrap();

    let sf = SourceFile::from_path(file.path().to_path_buf()).unwrap();

    assert_eq!(sf.source, "int main() {}");
    assert_eq!(sf.current_pos(), (1, 1));
}

#[test]
fn from_path_fails_on_nonexistent_file() {
    let result = SourceFile::from_path(PathBuf::from("/nonexistent/path/file.c"));

    assert!(result.is_err());
}
