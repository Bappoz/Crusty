use crate::common::ast::expr::Expr;
use crate::common::errors::error_data::Span;
use crate::common::errors::types::{CompilerError, SyntaxError};
use crate::lexer::tokens::token::Token;
use crate::lexer::tokens::token_kind::TokenKind;
use crate::parser::rules::expressions::{infix, postfix, prefix};

// TODO(parser): manter somente o parser de expressoes neste arquivo por enquanto.
// TODO(parser): declarações, statements e parse de programa completo serão adicionados depois.
// pub fn parse_program(&mut self) -> Result<Program, Diagnostic> { ... }

// Alias local para refletir a assinatura pedida sem alterar o sistema global de erros.
type Diagnostic = CompilerError;

// Estrutura mínima pedida: apenas fluxo de tokens para parser de expressões.
pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
}

impl Parser {
    // Construtor mínimo para o parser de expressões.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    // Implementação atual com tipo de erro já existente no projeto.
    pub fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, Diagnostic> {
        let mut lhs = prefix::parse_prefix_expr(self)?;

        loop {
            // Primeiro tratamos todos os postfix, pois têm maior precedência efetiva.
            if postfix::try_parse_postfix(self, &mut lhs)? {
                continue;
            }

            let op = self.peek_kind().clone();
            let Some((lbp, rbp, ternary)) = crate::parser::precedence::infix_binding_power(&op) else {
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

        if !self.is_expression_terminator(self.peek_kind()) {
            let found = self.peek().clone();
            return Err(self.syntax_error(&found, "fim de expressão", &format!("{:?}", found.kind)));
        }

        Ok(lhs)
    }

    // Helpers de navegação de token.
    pub(crate) fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    pub(crate) fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    pub(crate) fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        &self.tokens[self.pos.saturating_sub(1)]
    }

    pub(crate) fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        // O parser usa essa verificação apenas para tokens sem payload, como ')', ',', ']'.
        // Comparar discriminante evita depender do valor interno de literais/identificadores.
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    pub(crate) fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            return true;
        }
        false
    }

    pub(crate) fn expect(&mut self, kind: &TokenKind, expected: &str) -> Result<&Token, CompilerError> {
        if self.check(kind) {
            return Ok(self.advance());
        }
        let found = self.peek().clone();
        Err(self.syntax_error(&found, expected, &format!("{:?}", found.kind)))
    }

    // Helpers de erro e span.
    pub(crate) fn span_of(&self, token: &Token) -> Span {
        let width = token.span.end.saturating_sub(token.span.start).max(1);
        Span {
            line: token.line,
            end_line: token.line,
            column_start: token.col,
            column_end: token.col + width,
        }
    }

    pub(crate) fn join_span(&self, start: Span, end: Span) -> Span {
        Span {
            line: start.line,
            end_line: end.end_line,
            column_start: start.column_start,
            column_end: end.column_end,
        }
    }

    fn is_expression_terminator(&self, kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Eof
                | TokenKind::Comma
                | TokenKind::RightParen
                | TokenKind::RightBracket
                | TokenKind::Colon
                | TokenKind::Semicolon
                | TokenKind::RightBrace
        )
    }

    pub(crate) fn syntax_error(&self, token: &Token, expected: &str, found: &str) -> CompilerError {
        CompilerError::Syntax(SyntaxError {
            span: self.span_of(token),
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }

    pub(crate) fn syntax_error_from_span(&self, span: Span, expected: &str, found: &str) -> CompilerError {
        CompilerError::Syntax(SyntaxError {
            span,
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }
}

