#[cfg(test)]
mod tests {
    use crate::lexer::tokens::{token::Token, token_kind::TokenKind};

    #[test]
    fn token_display_shows_lexeme() {
        let token = Token {
            kind: TokenKind::Identifier("foo".to_string()),
            lexeme: "foo".to_string(),
            line: 1,
            col: 1,
        };

        assert_eq!(token.to_string(), "foo");
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
