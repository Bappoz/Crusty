use crate::common::errors::{
    error_data::Span,
    types::{CompilerError, LexicalError, LexicalErrorKind},
};
use crate::common::input::source::SourceFile;
use crate::common::utils::char_utils::is_ident_start;
use crate::lexer::rules::{
    identifiers::IdentifierRules,
    literals::LiteralsRules,
    operators::OperatorRules,
};
use crate::lexer::tokens::{token::Token, token_kind::TokenKind};

pub struct Scanner {
    pub src: SourceFile,
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<CompilerError>,
    /// Pilha de delimitadores abertos ainda não fechados: (char, linha, coluna)
    delimiter_stack: Vec<(char, usize, usize)>,
}

impl Scanner {
    // Construtor
    pub fn new(src: SourceFile) -> Self {
        Self {
            src,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
            delimiter_stack: Vec::new(),
        }
    }

    // Roda o scanner ate o fim do arquivo e retorna os tokens produzidos
    pub fn scan(&mut self) -> &[Token] {
        while !self.src.is_at_end() {
            self.skip_whitespaces_and_comments();
            if self.src.is_at_end() {
                break;
            }
            self.next_token();
        }

        // Delimitadores abertos sem fechamento → um diagnóstico por abertura
        let unclosed: Vec<(char, usize, usize)> = self.delimiter_stack.drain(..).collect();
        for (c, line, col) in unclosed {
            self.diagnostics.push(CompilerError::Lexical(LexicalError {
                span: Span { line, column_start: col, column_end: col + 1 },
                kind: LexicalErrorKind::UnclosedDelimiter(c),
            }));
        }

        // Sempre termina com EOF para o parser saber que acabou
        self.emit_at(TokenKind::Eof, "", self.src.line(), self.src.col());
        &self.tokens
    }

    // lê o próximo char e despacha para o método correto
    fn next_token(&mut self) {
        let line = self.src.line();
        let col = self.src.col();

        let c = match self.src.advance() {
            Some(c) => c,
            None => return,
        };

        match c {
            '0'..='9' => self.lex_number(c, line, col),
            '"'  => self.lex_string(line, col),
            '\'' => self.lex_char(line, col),
            c if is_ident_start(c) => self.lex_identifier(c, line, col),

            // Delimitadores de abertura — empilha para rastrear fechamento
            '(' => { self.delimiter_stack.push(('(', line, col)); self.emit_at(TokenKind::LeftParen,    "(", line, col); }
            '[' => { self.delimiter_stack.push(('[', line, col)); self.emit_at(TokenKind::LeftBracket,  "[", line, col); }
            '{' => { self.delimiter_stack.push(('{', line, col)); self.emit_at(TokenKind::LeftBrace,    "{", line, col); }

            // Delimitadores de fechamento — desempilha o par ou reporta mismatch/inesperado
            ')' => {
                if matches!(self.delimiter_stack.last(), Some(('(', _, _))) {
                    self.delimiter_stack.pop();
                } else {
                    self.emit_unexpected_delimiter(')', line, col);
                }
                self.emit_at(TokenKind::RightParen, ")", line, col);
            }
            ']' => {
                if matches!(self.delimiter_stack.last(), Some(('[', _, _))) {
                    self.delimiter_stack.pop();
                } else {
                    self.emit_unexpected_delimiter(']', line, col);
                }
                self.emit_at(TokenKind::RightBracket, "]", line, col);
            }
            '}' => {
                if matches!(self.delimiter_stack.last(), Some(('{', _, _))) {
                    self.delimiter_stack.pop();
                } else {
                    self.emit_unexpected_delimiter('}', line, col);
                }
                self.emit_at(TokenKind::RightBrace, "}", line, col);
            }

            // Pontuação simples sem lookahead
            '%' => self.emit_at(TokenKind::Percent,   "%", line, col),
            '^' => self.emit_at(TokenKind::Caret,     "^", line, col),
            '~' => self.emit_at(TokenKind::Tilde,     "~", line, col),
            '.' => self.emit_at(TokenKind::Dot,       ".", line, col),
            ';' => self.emit_at(TokenKind::Semicolon, ";", line, col),
            ',' => self.emit_at(TokenKind::Comma,     ",", line, col),
            ':' => self.emit_at(TokenKind::Colon,     ":", line, col),

            // Operadores (simples e compostos) — delega para operators.rs
            '+' | '-' | '*' | '/' | '=' | '!' | '<' | '>' | '&' | '|'
                => self.lex_operator(c, line, col),

            c => self.emit_unknown(c, line, col),
        }
    }

    // Emite um token com posição explícita.
    pub fn emit_at(&mut self, kind: TokenKind, lexeme: &str, line: usize, col: usize) {
        use crate::common::input::span::ByteSpan;
        let start = self.src.pos.saturating_sub(lexeme.len());
        let end = self.src.pos;
        self.tokens.push(Token {
            kind,
            span: ByteSpan { start, end },
            line,
            col,
        });
    }

    // Ignora espaços em branco, comentários de linha (//) e de bloco (/* */)
    fn skip_whitespaces_and_comments(&mut self) {
        loop {
            // Pula whitespace
            while matches!(
                self.src.peek(),
                Some(' ') | Some('\t') | Some('\r') | Some('\n')
            ) {
                self.src.advance();
            }

            // Comentário de linha: //
            if self.src.peek() == Some('/') && self.src.peek_ahead() == Some('/') {
                while !matches!(self.src.peek(), Some('\n') | None) {
                    self.src.advance();
                }
                continue;
            }

            // Comentário de bloco: /* ... */
            if self.src.peek() == Some('/') && self.src.peek_ahead() == Some('*') {
                let comment_line = self.src.line();
                let comment_col  = self.src.col();
                self.src.advance(); // '/'
                self.src.advance(); // '*'

                let mut closed = false;
                loop {
                    match self.src.advance() {
                        None => {
                            self.diagnostics.push(CompilerError::Lexical(LexicalError {
                                span: Span {
                                    line: comment_line,
                                    column_start: comment_col,
                                    column_end: comment_col + 2,
                                },
                                kind: LexicalErrorKind::UnclosedBlockComment,
                            }));
                            break;
                        }
                        Some('*') if self.src.peek() == Some('/') => {
                            self.src.advance(); // '/'
                            closed = true;
                            break;
                        }
                        _ => {}
                    }
                }
                if closed { continue; } else { break; }
            }

            // Diretivas de pré-processador: #include, #define, etc.
            if self.src.peek() == Some('#') {
                while !matches!(self.src.peek(), Some('\n') | None) {
                    self.src.advance();
                }
                continue;
            }

            break;
        }
    }

    // Emite um token Unknown e registra o diagnostico de error
    pub fn emit_unknown(&mut self, c: char, line: usize, col: usize) {
        self.diagnostics.push(CompilerError::Lexical(LexicalError {
            span: Span {
                line,
                column_start: col,
                column_end: col + 1,
            },
            kind: LexicalErrorKind::InvalidChar(c),
        }));
        self.emit_at(TokenKind::Unknown(c), &c.to_string(), line, col);
    }

    fn emit_unexpected_delimiter(&mut self, c: char, line: usize, col: usize) {
        self.diagnostics.push(CompilerError::Lexical(LexicalError {
            span: Span { line, column_start: col, column_end: col + 1 },
            kind: LexicalErrorKind::UnexpectedClosingDelimiter(c),
        }));
    }

    // Emite um diagnóstico de literal não terminada (string ou char sem fechamento)
    pub fn emit_unterminated_literal(&mut self, lit: &str, line: usize, col_start: usize, col_end: usize) {
        self.diagnostics.push(CompilerError::Lexical(LexicalError {
            span: Span {
                line,
                column_start: col_start,
                column_end: col_end,
            },
            kind: LexicalErrorKind::UnterminatedLiteral(lit.to_string()),
        }));
    }
}
