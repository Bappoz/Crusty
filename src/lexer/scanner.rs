use crate::common::token::{Token, TokenKind};

pub struct Scanner {
    source: Vec<char>,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
    col: usize,
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        Scanner {
            source: source.chars().collect(),
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn scan_tokens(mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            line: self.line,
            col: self.col,
        });

        self.tokens
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.current];
        self.current += 1;
        self.col += 1;
        ch
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.current + 1]
        }
    }

    fn add_token(&mut self, kind: TokenKind) {
        let lexeme: String = self.source[self.start..self.current].iter().collect();
        self.tokens.push(Token {
            kind,
            lexeme,
            line: self.line,
            col: self.col, // simplificado
        });
    }

    fn scan_token(&mut self) {
        let c = self.advance();

        match c {
            // Espaços em branco
            ' ' | '\r' | '\t' => { /* ignora */ }
            '\n' => {
                self.line += 1;
                self.col = 1;
            }

            // Delimitadores
            '(' => self.add_token(TokenKind::LeftParen),
            ')' => self.add_token(TokenKind::RightParen),
            '{' => self.add_token(TokenKind::LeftBrace),
            '}' => self.add_token(TokenKind::RightBrace),
            ';' => self.add_token(TokenKind::Semicolon),
            ',' => self.add_token(TokenKind::Comma),

            // Operadores simples e duplos
            '+' => self.add_token(TokenKind::Plus),
            '-' => self.add_token(TokenKind::Minus),
            '*' => self.add_token(TokenKind::Star),
            '/' => self.add_token(TokenKind::Slash),

            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::EqualEqual);
                } else {
                    self.add_token(TokenKind::Equal);
                }
            }
            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::BangEqual);
                } else {
                    // se quiser ter TokenKind::Bang, adicione lá no enum
                    self.add_token(TokenKind::Unknown);
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::LessEqual);
                } else {
                    self.add_token(TokenKind::Less);
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenKind::GreaterEqual);
                } else {
                    self.add_token(TokenKind::Greater);
                }
            }

            // Literais string
            '"' => self.string_literal(),

            // Dígitos -> número
            c if c.is_ascii_digit() => self.number(),

            // Identificadores / palavras-chave
            c if is_identifier_start(c) => self.identifier(),

            // Qualquer outra coisa
            _ => {
                self.add_token(TokenKind::Unknown);
            }
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.source[self.current] != expected {
            return false;
        }
        self.current += 1;
        self.col += 1;
        true
    }

    fn string_literal(&mut self) {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
                self.col = 1;
            }
            self.advance();
        }

        // Fecha aspas
        if self.is_at_end() {
            // ideal: registrar erro de string não terminada
            return;
        }

        // consumir o '"'
        self.advance();

        self.add_token(TokenKind::StringLiteral);
    }

    fn number(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        // parte fracionária opcional
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance(); // ponto
            while self.peek().is_ascii_digit() {
                self.advance();
            }
            self.add_token(TokenKind::FloatLiteral);
        } else {
            self.add_token(TokenKind::IntLiteral);
        }
    }

    fn identifier(&mut self) {
        while is_identifier_continue(self.peek()) {
            self.advance();
        }

        let text: String = self.source[self.start..self.current].iter().collect();

        let kind = match text.as_str() {
            "if" => TokenKind::If,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "loop" => TokenKind::Loop,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "impl" => TokenKind::Impl,
            "trait" => TokenKind::Trait,
            "pub" => TokenKind::Pub,
            "mod" => TokenKind::Mod,
            "use" => TokenKind::Use,
            "const" => TokenKind::Const,
            "static" => TokenKind::Static,
            "int" => TokenKind::Int,
            "return" => TokenKind::Return,
            _ => TokenKind::Identifier,
        };

        self.add_token(kind);
    }
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_continue(c: char) -> bool {
    is_identifier_start(c) || c.is_ascii_digit()
}