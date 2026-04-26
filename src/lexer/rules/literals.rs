use crate::common::utils::char_utils::*;
use crate::lexer::scanner::Scanner;
use crate::lexer::tokens::TokenKind;

pub trait LiteralsRules {
    fn lex_number(&mut self, first: char, line: usize, col: usize);
    fn lex_string(&mut self, line: usize, col: usize);
    fn lex_char(&mut self, line: usize, col: usize);
}

impl LiteralsRules for Scanner {
    fn lex_number(&mut self, first: char, line: usize, col: usize) {
        let mut buf = String::from(first);

        // HEXADECIMAL: 0xf...
        // If responsavel para resolver Hexadecimais
        if first == '0' && matches!(self.src.peek(), Some('x') | Some('X')) {
            buf.push(self.src.advance().unwrap());

            // Consome todos os digitos do hex
            while let Some(c) = self.src.peek() {
                if is_hex_digit(c) {
                    buf.push(c);
                    self.src.advance();
                } else {
                    break;
                }
            }
            // Converte "FF" para i64 na base 16
            let value = i64::from_str_radix(&buf[2..], 16).unwrap_or(0);
            while matches!(self.src.peek(), Some('u' | 'U' | 'l' | 'L')) {
                buf.push(self.src.advance().unwrap());
            }
            return self.emit_at(TokenKind::IntLiteral(value), &buf, line, col);
        }

        // -------------------------------------------------

        // OCTAL: 0755, ...
        // If Responsavel para resolver os Hexadecimais
        if first == '0' && matches!(self.src.peek(), Some('0'..='7')) {
            // Consome todos os digitos do OCTAL
            while let Some(c) = self.src.peek() {
                if is_octal_digit(c) {
                    buf.push(c);
                    self.src.advance();
                } else {
                    break;
                }
            }
            // Converte na base 8 pulando o 0 inicial
            let value = i64::from_str_radix(&buf[1..], 8).unwrap_or(0);
            while matches!(self.src.peek(), Some('u' | 'U' | 'l' | 'L')) {
                buf.push(self.src.advance().unwrap());
            }
            return self.emit_at(TokenKind::IntLiteral(value), &buf, line, col);
        }

        // -------------------------------------------------

        // Decimal: 42, 3.14, 1e10 e etc...
        // Consome todos os digitos decimais restantes
        while let Some(c) = self.src.peek() {
            if is_decimal_digit(c) {
                buf.push(c);
                self.src.advance();
            } else {
                break;
            }
        }

        // Decide se é Int ou Float
        if matches!(self.src.peek(), Some('.') | Some('e') | Some('E')) {
            // Parte fracionaria
            if self.src.peek() == Some('.') {
                buf.push('.');
                self.src.advance();
                while let Some(c) = self.src.peek() {
                    if is_decimal_digit(c) {
                        buf.push(c);
                        self.src.advance();
                    } else {
                        break;
                    }
                }
            }

            // Checando a parte de expoente
            if matches!(self.src.peek(), Some('e') | Some('E')) {
                buf.push(self.src.advance().unwrap());

                if matches!(self.src.peek(), Some('-') | Some('+')) {
                    buf.push(self.src.advance().unwrap());
                }

                while let Some(c) = self.src.peek() {
                    if is_decimal_digit(c) {
                        buf.push(c);
                        self.src.advance();
                    } else {
                        break;
                    }
                }
            }

            // Parse da string completa para f64
            let value: f64 = buf.parse().unwrap_or(0.0);
            while matches!(self.src.peek(), Some('f' | 'F' | 'l' | 'L')) {
                buf.push(self.src.advance().unwrap());
            }
            self.emit_at(TokenKind::FloatLiteral(value), &buf, line, col);
        } else {
            // Chechou tudo e é Inteiro
            let value: i64 = buf.parse().unwrap_or(0);
            while matches!(self.src.peek(), Some('u' | 'U' | 'l' | 'L')) {
                buf.push(self.src.advance().unwrap());
            }
            self.emit_at(TokenKind::IntLiteral(value), &buf, line, col);
        }
    }

    fn lex_string(&mut self, line: usize, col: usize) {
        let mut value = String::new();
        let mut lexeme = String::from('"');
        let mut col_end = col + 1;

        loop {
            match self.src.advance() {
                Some('"') => {
                    lexeme.push('"');
                    break;
                }
                Some('\\') => {
                    lexeme.push('\\');
                    col_end += 1;
                    if let Some(e) = self.src.advance() {
                        lexeme.push(e);
                        value.push(resolve_escape(e).unwrap_or(e));
                        col_end += 1;
                    }
                }
                None => {
                    self.emit_unterminated_literal("string", line, col, col_end);
                    break;
                }
                Some(c) => {
                    lexeme.push(c);
                    value.push(c);
                    col_end += 1;
                }
            }
        }
        self.emit_at(TokenKind::StringLiteral(value), &lexeme, line, col);
    }

    fn lex_char(&mut self, line: usize, col: usize) {
        let mut lexeme = String::from('\'');
        let mut col_end = col + 1;

        let c = match self.src.advance() {
            Some('\\') => {
                lexeme.push('\\');
                col_end += 1;
                match self.src.advance() {
                    Some(e) => {
                        lexeme.push(e);
                        col_end += 1;
                        resolve_escape(e).unwrap_or(e)
                    }
                    None => {
                        self.emit_unterminated_literal("char", line, col, col_end);
                        '\0'
                    }
                }
            }
            Some(c) => {
                lexeme.push(c);
                col_end += 1;
                c
            }
            None => {
                self.emit_unterminated_literal("char", line, col, col_end);
                '\0'
            }
        };

        // Fecha aspas simples
        match self.src.advance() {
            Some('\'') => {
                lexeme.push('\'');
                self.emit_at(TokenKind::CharLiteral(c), &lexeme, line, col);
            }
            Some(err) => self.emit_unknown(err, line, col),
            None => self.emit_unterminated_literal("char", line, col, col_end),
        }
    }
}
