use crate::common::errors::{
    error_data::Span,
    types::{CompilerError, LexicalError},
};
use crate::common::input::source::SourceFile;
use crate::common::utils::char_utils::is_ident_start;
use crate::lexer::rules::{identifiers::IdentifierRules, literals::LiteralsRules};
use crate::lexer::tokens::{token::Token, token_kind::TokenKind};

pub struct Scanner {
    pub src: SourceFile,
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<CompilerError>,
}

impl Scanner {
    // Construtor
    pub fn new(src: SourceFile) -> Self {
        Self {
            src,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
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

        // Sempre termina com EOF para o parser saber que acabou
        self.emit_at(TokenKind::Eof, "", self.src.line(), self.src.col());
        &self.tokens
    }

    // lê o próximo char e despacha para o método correto
    fn next_token(&mut self) {
        // Deve capturar a posicao antes de consumir o char
        let line = self.src.line();
        let col = self.src.col();

        let c = match self.src.advance() {
            Some(c) => c,
            None => return,
        };

        match c {
            // Literais numéricos — delega para literals.rs
            '0'..='9' => self.lex_number(c, line, col),

            // Literais de texto — delega para literals.rs
            '"' => self.lex_string(line, col),
            '\'' => self.lex_char(line, col),

            // Identificadores e keyworkds
            c if is_ident_start(c) => self.lex_identifier(c, line, col),

            // Operadores e delimitadores de um char só (sem lookahead)
            '%' => self.emit_at(TokenKind::Percent, "%", line, col),
            '^' => self.emit_at(TokenKind::Caret, "^", line, col),
            '~' => self.emit_at(TokenKind::Tilde, "~", line, col),
            '.' => self.emit_at(TokenKind::Dot, ".", line, col),
            '(' => self.emit_at(TokenKind::LeftParen, "(", line, col),
            ')' => self.emit_at(TokenKind::RightParen, ")", line, col),
            '{' => self.emit_at(TokenKind::LeftBrace, "{", line, col),
            '}' => self.emit_at(TokenKind::RightBrace, "}", line, col),
            '[' => self.emit_at(TokenKind::LeftBracket, "[", line, col),
            ']' => self.emit_at(TokenKind::RightBracket, "]", line, col),
            ';' => self.emit_at(TokenKind::Semicolon, ";", line, col),
            ',' => self.emit_at(TokenKind::Comma, ",", line, col),
            ':' => self.emit_at(TokenKind::Colon, ":", line, col),

            // Operadores que precisam de lookahead (+, -, *, /, etc.)
            // ficam aqui por enquanto como Unknown até operators.rs existir
            // MODIFICAR ESSA PARTE QUANDO IMPLEMENTAR AS OUTRAS REGRAS
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

    // Ignorar espaços em branco e Comentarios
    // Nao geram tokens
    fn skip_whitespaces_and_comments(&mut self) {
        loop {
            while matches!(
                self.src.peek(),
                Some(' ') | Some('\t') | Some('\r') | Some('\n')
            ) {
                self.src.advance();
            }

            // Comentários
            if self.src.peek() == Some('/') && self.src.peek_ahead() == Some('/') {
                while !matches!(self.src.peek(), Some('\n') | None) {
                    self.src.advance();
                }
                continue;
            }

            // Diretivas de pré-processador: ignora linha inteira se começa com '#'
            if self.src.peek() == Some('#') {
                // Consome até o fim da linha ou EOF
                while let Some(c) = self.src.peek() {
                    self.src.advance();
                    if c == '\n' { break; }
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
                end_line: line,
                column_start: col,
                column_end: col + 1,
            },
            invalid_char: c,
            unterminated_literal: None,
        }));
        self.emit_at(TokenKind::Unknown(c), &c.to_string(), line, col);
    }

    pub fn emit_unterminated_literal(&mut self, lit: &str, line: usize, col_start: usize, col_end: usize) {
        self.diagnostics.push(CompilerError::Lexical(LexicalError {
            span: Span {
                line,
                end_line: line,
                column_start: col_start,
                column_end: col_end,
            },
            invalid_char: '\0',
            unterminated_literal: Some(lit.to_string()),
        }));
    }
}
