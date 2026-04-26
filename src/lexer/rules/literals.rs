use crate::common::errors::error_data::Span;
use crate::common::errors::types::{CompilerError, LexicalError, LexicalErrorKind};
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
            return self.emit_at(TokenKind::IntLiteral(value), &buf, line, col);
        }

        // -------------------------------------------------

        // OCTAL: 0755, ...
        // If Responsavel para resolver os Hexadecimais
        if first == '0' {
            // Consome todos os digitos do OCTAL
            while let Some(c) = self.src.peek() {
                if is_octal_digit(c) {
                    buf.push(c);
                    self.src.advance();
                } else {
                    break;
                }
            }

            // Se o proximo carctere é 8 ou 9, armazene em 'c' sem consumi-lo
            if let Some(c @ ('8' | '9')) = self.src.peek() {
                let error_col = self.src.col(); // Pego a coluna que está o 8/9
                self.src.advance(); // consumo o 8/9, para não passar novamente pelo scanner

                self.diagnostics.push(CompilerError::Lexical(LexicalError {
                    // relatório de erro: posição ee tipo
                    span: Span {
                        line,
                        column_start: error_col,
                        column_end: error_col + 1,
                    },
                    kind: LexicalErrorKind::InvalidOctalDigit(c),
                }));

                return self.emit_at(TokenKind::Unknown(c), &c.to_string(), line, error_col);
                // classifica como erro para não passar pra próxima fase:
            } // passando o digito inválido com string e sua posição

            if buf.len() == 1 {
                return self.emit_at(TokenKind::IntLiteral(0), &buf, line, col);
            }
            // Converte na base 8 pulando o 0 inicial
            let value = i64::from_str_radix(&buf[1..], 8).unwrap_or(0);
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
            self.emit_at(TokenKind::FloatLiteral(value), &buf, line, col);
        } else {
            // Chechou tudo e é Inteiro
            let value: i64 = buf.parse().unwrap_or(0);
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
                // FIX: Issue #58 - Rejeita newline real dentro de string
                Some('\n') => {
                    self.emit_unterminated_literal("string", line, col, col_end);
                    break;
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

        // '' é inválido em C — char literal vazio
        if self.src.peek() == Some('\'') {
            self.src.advance(); // consome o fechamento imediato
            self.emit_unterminated_literal("char", line, col, col_end);
            return;
        }

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
            Some('\n') => {
                self.emit_unterminated_literal("char", line, col, col_end);
                return; // Early return para não tentar fechar as aspas
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
