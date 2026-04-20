use crate::common::input::source::SourceFile;
use std::io::Write;
use std::path::PathBuf;

fn src(input: &str) -> SourceFile {
    SourceFile::from_string(input)
}

// helper para desempacotar o Result sem precisar de Debug no erro
fn unwrap_sf(
    result: Result<SourceFile, Box<dyn crate::common::errors::report::ToReport>>,
) -> SourceFile {
    match result {
        Ok(sf) => sf,
        Err(e) => panic!("SourceFile::from_path falhou: {:?}", e.to_report()),
    }
}

#[test]
fn peek_does_not_consume() {
    let sf = src("abc");
    assert_eq!(sf.peek(), Some('a'));
    assert_eq!(sf.peek(), Some('a'));
    assert_eq!(sf.peek(), Some('a'));
    assert_eq!(sf.pos, 0);
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
    assert_eq!(sf.advance(), None);
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
    sf.advance();
    sf.advance();
    sf.advance();
    sf.advance();
    sf.advance();
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
    sf.advance();
    sf.advance();
    sf.advance();
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

#[test]
fn from_path_reads_file_correctly() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "int main() {{}}").unwrap();

    // usa o helper em vez de .unwrap() — evita exigir Debug no erro
    let sf = unwrap_sf(SourceFile::from_path(file.path().to_path_buf()));

    assert_eq!(sf.source, "int main() {}");
    assert_eq!(sf.current_pos(), (1, 1));
}

#[test]
fn from_path_fails_on_nonexistent_file() {
    let result = SourceFile::from_path(PathBuf::from("/nonexistent/path/file.c"));

    // is_err() não precisa de Debug — só checa se falhou
    assert!(result.is_err());
}
