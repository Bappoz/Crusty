use crate::lexer::scanner::Scanner;
use crate::lexer::tokens::TokenKind;

pub trait OperatorRules {
    fn lex_operator(&mut self, c: char, line: usize, col: usize);
}

impl OperatorRules for Scanner {
    /// Reconhece operadores simples e compostos com lookahead de 1 char, emitindo o token correto.
    fn lex_operator(&mut self, c: char, line: usize, col: usize) {
        match c {
            // -------------------------------------------------------
            // + : ++ | += | +
            '+' => match self.src.peek() {
                Some('+') => {
                    self.src.advance();
                    self.emit_at(TokenKind::PlusPlus, line, col);
                }
                Some('=') => {
                    self.src.advance();
                    self.emit_at(TokenKind::PlusEqual, line, col);
                }
                _ => self.emit_at(TokenKind::Plus, line, col),
            },

            // -------------------------------------------------------
            // - : -- | -= | -> | -
            '-' => match self.src.peek() {
                Some('-') => {
                    self.src.advance();
                    self.emit_at(TokenKind::MinusMinus, line, col);
                }
                Some('=') => {
                    self.src.advance();
                    self.emit_at(TokenKind::MinusEqual, line, col);
                }
                Some('>') => {
                    self.src.advance();
                    self.emit_at(TokenKind::Arrow, line, col);
                }
                _ => self.emit_at(TokenKind::Minus, line, col),
            },

            // -------------------------------------------------------
            // * : *= | *
            '*' => match self.src.peek() {
                Some('=') => {
                    self.src.advance();
                    self.emit_at(TokenKind::StarEqual, line, col);
                }
                _ => self.emit_at(TokenKind::Star, line, col),
            },

            // -------------------------------------------------------
            // / : /= | /
            // Nota: '//' e '/*' já são consumidos em skip_whitespaces_and_comments
            // então aqui só chegam '/' isolado ou '/='
            '/' => match self.src.peek() {
                Some('=') => {
                    self.src.advance();
                    self.emit_at(TokenKind::SlashEqual, line, col);
                }
                _ => self.emit_at(TokenKind::Slash, line, col),
            },

            // -------------------------------------------------------
            // = : == | =
            '=' => match self.src.peek() {
                Some('=') => {
                    self.src.advance();
                    self.emit_at(TokenKind::EqualEqual, line, col);
                }
                _ => self.emit_at(TokenKind::Equal, line, col),
            },

            // -------------------------------------------------------
            // ! : != | !
            '!' => match self.src.peek() {
                Some('=') => {
                    self.src.advance();
                    self.emit_at(TokenKind::BangEqual, line, col);
                }
                _ => self.emit_at(TokenKind::Bang, line, col),
            },

            // -------------------------------------------------------
            // < : <= | <<= | << | <
            '<' => match self.src.peek() {
                Some('=') => {
                    self.src.advance();
                    self.emit_at(TokenKind::LessEqual, line, col);
                }
                Some('<') => {
                    self.src.advance();
                    if self.src.peek() == Some('=') {
                        self.src.advance();
                        self.emit_at(TokenKind::LessLessEqual, line, col);
                    } else {
                        self.emit_at(TokenKind::LessLess, line, col);
                    }
                }
                _ => self.emit_at(TokenKind::Less, line, col),
            },

            // -------------------------------------------------------
            // > : >= | >>= | >> | >
            '>' => match self.src.peek() {
                Some('=') => {
                    self.src.advance();
                    self.emit_at(TokenKind::GreaterEqual, line, col);
                }
                Some('>') => {
                    self.src.advance();
                    if self.src.peek() == Some('=') {
                        self.src.advance();
                        self.emit_at(TokenKind::GreaterGreaterEqual, line, col);
                    } else {
                        self.emit_at(TokenKind::GreaterGreater, line, col);
                    }
                }
                _ => self.emit_at(TokenKind::Greater, line, col),
            },

            // -------------------------------------------------------
            // & : && | &
            '&' => match self.src.peek() {
                Some('&') => {
                    self.src.advance();
                    self.emit_at(TokenKind::AndAnd, line, col);
                }
                _ => self.emit_at(TokenKind::Ampersand, line, col),
            },

            // -------------------------------------------------------
            // | : || | |
            '|' => match self.src.peek() {
                Some('|') => {
                    self.src.advance();
                    self.emit_at(TokenKind::OrOr, line, col);
                }
                _ => self.emit_at(TokenKind::Pipe, line, col),
            },

            // Qualquer outro char cai aqui como desconhecido
            other => self.emit_unknown(other, line, col),
        }
    }
}
