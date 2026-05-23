use crate::common::ast::ast::Program;
use crate::common::ast::expr::Expr;
use crate::common::ast::stmt::Stmt;
use crate::common::errors::error_data::Span;
use crate::common::errors::types::{CompilerError, SyntaxError};
use crate::lexer::tokens::token::Token;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::rules::expressions::{infix, postfix, prefix};

type Diagnostic = CompilerError;

// Estrutura mínima pedida: apenas fluxo de tokens para parser de expressões.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Cria um novo `Parser` a partir de um vetor de tokens produzido pelo `Scanner`.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub(crate) fn peek_next(&self) -> &Token {
        let next = (self.pos + 1).min(self.tokens.len() - 1);
        &self.tokens[next]
    }

    /// Parseia uma expressão com precedência mínima `min_bp` usando o algoritmo Pratt (top-down operator precedence).
    /// Retorna a expressão construída ou um `Diagnostic` de erro sintático.
    pub fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, Diagnostic> {
        let mut lhs = prefix::parse_prefix_expr(self)?;

        loop {
            // Primeiro tratamos todos os postfix, pois têm maior precedência efetiva.
            if postfix::try_parse_postfix(self, &mut lhs)? {
                continue;
            }

            let op = self.peek_kind().clone();
            let Some((lbp, rbp, ternary)) = crate::parser::precedence::infix_binding_power(&op)
            else {
                break;
            };

            if lbp < min_bp {
                break;
            }

            let op_token = self.advance().clone();

            if ternary {
                lhs = infix::parse_ternary(self, lhs, rbp)?;
                continue;
            }

            let rhs = self.parse_expr(rbp)?;
            let span = self.join_span(lhs.span(), rhs.span());

            if op == TokenKind::Equal {
                lhs = Expr::Assign(Box::new(lhs), Box::new(rhs), span);
            } else {
                let bin = infix::token_to_bin_op(self, &op, &op_token)?;
                lhs = Expr::Binary(Box::new(lhs), bin, Box::new(rhs), span);
            }
        }

        Ok(lhs)
    }

    /// Dispatcher de statements: delega para o parser completo de statements.
    pub fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        crate::parser::rules::statements::parse_stmt(self)
    }

    /// Parseia um programa C completo: declarações globais até EOF.
    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        crate::parser::rules::declarations::parse_program(self)
    }

    /// Retorna o token atual sem avançar a posição.
    pub(crate) fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    /// Retorna o `TokenKind` do token atual sem avançar a posição.
    pub(crate) fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    /// Consome e retorna o token atual, avançando a posição (sem ultrapassar EOF).
    pub(crate) fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        if !self.is_at_end() {
            self.pos += 1;
        }
        token
    }

    /// Retorna `true` se o token atual é `Eof`, indicando fim do stream de tokens.
    pub(crate) fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    /// Verifica se o token atual tem o mesmo discriminante de `kind`, ignorando valores internos.
    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        // O parser usa essa verificação apenas para tokens sem payload, como ')', ',', ']'.
        // Comparar discriminante evita depender do valor interno de literais/identificadores.
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    /// Avança e retorna `true` se o token atual corresponde a `kind`; não avança caso contrário.
    pub(crate) fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            return true;
        }
        false
    }

    /// Consome o token atual se corresponder a `kind`; retorna erro sintático com `expected` caso contrário.
    pub(crate) fn expect(
        &mut self,
        kind: &TokenKind,
        expected: &str,
    ) -> Result<&Token, CompilerError> {
        if self.check(kind) {
            return Ok(self.advance());
        }
        let found = self.peek().clone();
        Err(self.syntax_error(&found, expected, &format!("{:?}", found.kind)))
    }

    /// Converte um `Token` em `Span` de diagnóstico, usando a largura em bytes como extensão da coluna.
    pub(crate) fn span_of(&self, token: &Token) -> Span {
        let width = token.span.end.saturating_sub(token.span.start).max(1);
        Span {
            line: token.line,
            end_line: token.line,
            column_start: token.col,
            column_end: token.col + width,
        }
    }

    /// Une dois spans em um único span que vai do início de `start` até o fim de `end`.
    pub(crate) fn join_span(&self, start: Span, end: Span) -> Span {
        Span {
            line: start.line,
            end_line: end.end_line,
            column_start: start.column_start,
            column_end: end.column_end,
        }
    }

    /// Constrói um `CompilerError::Syntax` com o span do token, o que era esperado e o que foi encontrado.
    pub(crate) fn syntax_error(&self, token: &Token, expected: &str, found: &str) -> CompilerError {
        CompilerError::Syntax(SyntaxError {
            span: self.span_of(token),
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }

    /// Constrói um `CompilerError::Syntax` a partir de um `Span` já calculado (sem token direto).
    pub(crate) fn syntax_error_from_span(
        &self,
        span: Span,
        expected: &str,
        found: &str,
    ) -> CompilerError {
        CompilerError::Syntax(SyntaxError {
            span,
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }
}
