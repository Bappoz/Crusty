#[cfg(test)]
mod tests {
    use crate::lexer::tokens::{token::Token, token_kind::TokenKind};

    #[test]
    fn token_display_shows_span() {
        use crate::common::input::span::ByteSpan;
        let token = Token {
            kind: TokenKind::Identifier("foo".to_string()),
            span: ByteSpan { start: 0, end: 3 },
            line: 1,
            col: 1,
        };

        assert_eq!(token.to_string(), "[0..3]");
    }

    #[test]
    fn token_kind_eq_works() {
        let t1 = TokenKind::If;
        let t2 = TokenKind::If;
        let t3 = TokenKind::While;

        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
    }
}
