use crate::common::input::source::SourceFile;

#[test]
fn test_utf8_char_boundaries() {
let mut sf = SourceFile::from_string("á b");

assert_eq!(sf.peek(), Some('á'));

let ch = sf.advance();
assert_eq!(ch,Some('á'));
assert_eq!(sf.pos, 2);

assert_eq!(sf.peek(), Some(' '));
sf.advance();

assert_eq!(sf.peek(), Some('b'));
assert_eq!(sf.pos, 3);
}
