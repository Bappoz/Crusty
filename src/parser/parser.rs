use crate::common::ast::ast::QualifierType;
use crate::common::ast::expr::{BinOp, Expr, Literal, UnOp};
use crate::common::errors::error_data::Span;
use crate::common::errors::types::{CompilerError, SyntaxError};
use crate::lexer::tokens::token::Token;
use crate::lexer::tokens::token_kind::TokenKind;

// TODO(parser): manter somente o parser de expressoes neste arquivo por enquanto.
// TODO(parser): declarações, statements e parse de programa completo serão adicionados depois.
// pub fn parse_program(&mut self) -> Result<Program, Diagnostic> { ... }

// Alias local para refletir a assinatura pedida sem alterar o sistema global de erros.
type Diagnostic = CompilerError;

// Estrutura mínima pedida: apenas fluxo de tokens para parser de expressões.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    // Construtor mínimo para o parser de expressões.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    // Implementação atual com tipo de erro já existente no projeto.
    pub fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_prefix_expr()?;

        loop {
            // Primeiro tratamos todos os postfix, pois têm maior precedência efetiva.
            if self.try_parse_postfix(&mut lhs)? {
                continue;
            }

            let op = self.peek_kind().clone();
            let Some((lbp, rbp, ternary)) = self.infix_binding_power(&op) else {
                break;
            };

            if lbp < min_bp {
                break;
            }

            let op_token = self.advance().clone();

            if ternary {
                // Operador ternário C-style: cond ? then_expr : else_expr
                let then_expr = self.parse_expr(rbp)?;
                self.expect(&TokenKind::Colon, "':' após expressão do braço true em ?:")?;
                let else_expr = self.parse_expr(rbp)?;

                // TODO(ast): quando houver nó dedicado de ternário, substituir este encode provisório.
                // Encode temporário: (cond ? then : else) -> Binary(cond, Or, Binary(then, Or, else)).
                let span = self.join_span(lhs.span(), else_expr.span());
                let encoded = Expr::Binary(
                    Box::new(then_expr),
                    BinOp::Or,
                    Box::new(else_expr),
                    span.clone(),
                );
                lhs = Expr::Binary(Box::new(lhs), BinOp::Or, Box::new(encoded), span);
                continue;
            }

            let rhs = self.parse_expr(rbp)?;
            let span = self.join_span(lhs.span(), rhs.span());

            if op == TokenKind::Equal {
                lhs = Expr::Assign(Box::new(lhs), Box::new(rhs), span);
            } else {
                let bin = self.token_to_bin_op(&op, &op_token)?;
                lhs = Expr::Binary(Box::new(lhs), bin, Box::new(rhs), span);
            }
        }

        Ok(lhs)
    }

    // Parse de prefixos: !, ~, -, ++, --, *, &, sizeof e cast.
    fn parse_prefix_expr(&mut self) -> Result<Expr, CompilerError> {
        let token = self.peek().clone();
        let kind = self.peek_kind().clone();

        if self.looks_like_cast() {
            return self.parse_cast_expr();
        }

        match kind {
            TokenKind::Bang
            | TokenKind::Tilde
            | TokenKind::Minus
            | TokenKind::PlusPlus
            | TokenKind::MinusMinus
            | TokenKind::Star
            | TokenKind::Ampersand
            | TokenKind::Sizeof => {
                let op = self.advance().clone();
                let bp = self.prefix_binding_power(&op.kind).ok_or_else(|| {
                    self.syntax_error(&op, "operador prefixo", &format!("{:?}", op.kind))
                })?;
                let rhs = self.parse_expr(bp)?;
                let span = self.join_span(self.span_of(&op), rhs.span());
                self.build_prefix_expr(op.kind, rhs, span)
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RightParen, "')' para fechar agrupamento")?;
                Ok(expr)
            }
            TokenKind::IntLiteral(v) => {
                self.advance();
                Ok(Expr::Literal(Literal::Int(v), self.span_of(&token)))
            }
            TokenKind::FloatLiteral(v) => {
                self.advance();
                Ok(Expr::Literal(Literal::Double(v), self.span_of(&token)))
            }
            TokenKind::StringLiteral(v) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(v), self.span_of(&token)))
            }
            TokenKind::CharLiteral(v) => {
                self.advance();
                Ok(Expr::Literal(Literal::Char(v), self.span_of(&token)))
            }
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(Expr::Ident(name, self.span_of(&token)))
            }
            _ => Err(self.syntax_error(&token, "expressão", &format!("{:?}", token.kind))),
        }
    }

    // Postfix: () call, [] index, . member, -> member, ++, --.
    fn try_parse_postfix(&mut self, lhs: &mut Expr) -> Result<bool, CompilerError> {
        match self.peek_kind() {
            TokenKind::LeftParen => {
                let start = lhs.span();
                self.advance();
                let mut args = Vec::new();

                if !self.check(&TokenKind::RightParen) {
                    loop {
                        args.push(self.parse_expr(0)?);
                        if !self.match_kind(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                let end = self
                    .expect(&TokenKind::RightParen, "')' ao fechar chamada")?
                    .clone();
                let span = self.join_span(start, self.span_of(&end));
                *lhs = Expr::Call(Box::new(lhs.clone()), args, span);
                Ok(true)
            }
            TokenKind::LeftBracket => {
                let start = lhs.span();
                self.advance();
                let index = self.parse_expr(0)?;
                let end = self
                    .expect(&TokenKind::RightBracket, "']' ao fechar indexação")?
                    .clone();
                let span = self.join_span(start, self.span_of(&end));
                *lhs = Expr::Index(Box::new(lhs.clone()), Box::new(index), span);
                Ok(true)
            }
            TokenKind::Dot | TokenKind::Arrow => {
                let op = self.advance().clone();
                let field_token = self.advance().clone();
                let TokenKind::Identifier(field_name) = field_token.kind.clone() else {
                    return Err(self.syntax_error(
                        &field_token,
                        "identificador de campo",
                        &format!("{:?}", field_token.kind),
                    ));
                };

                let field = Expr::Ident(field_name, self.span_of(&field_token));
                let span = self.join_span(lhs.span(), self.span_of(&field_token));

                // TODO(ast): substituir encode provisório quando AST tiver MemberAccess dedicado.
                let op_kind = if op.kind == TokenKind::Dot {
                    BinOp::BitOr
                } else {
                    BinOp::BitAnd
                };
                *lhs = Expr::Binary(Box::new(lhs.clone()), op_kind, Box::new(field), span);
                Ok(true)
            }
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let op = self.advance().clone();
                let one = Expr::Literal(Literal::Int(1), self.span_of(&op));
                let span = self.join_span(lhs.span(), self.span_of(&op));

                // TODO(ast): substituir por nó postfix específico quando existir na AST.
                let bop = if op.kind == TokenKind::PlusPlus {
                    BinOp::Add
                } else {
                    BinOp::Sub
                };
                *lhs = Expr::Assign(
                    Box::new(lhs.clone()),
                    Box::new(Expr::Binary(
                        Box::new(lhs.clone()),
                        bop,
                        Box::new(one),
                        span.clone(),
                    )),
                    span,
                );
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    // Tabela de binding power para operadores infixos.
    // Retorna: (left_bp, right_bp, is_ternary)
    fn infix_binding_power(&self, op: &TokenKind) -> Option<(u8, u8, bool)> {
        let bp = match op {
            TokenKind::Equal => (1, 1, false),
            TokenKind::OrOr => (2, 3, false),
            TokenKind::AndAnd => (4, 5, false),
            TokenKind::Pipe => (6, 7, false),
            TokenKind::Caret => (8, 9, false),
            TokenKind::Ampersand => (10, 11, false),
            TokenKind::EqualEqual | TokenKind::BangEqual => (12, 13, false),
            TokenKind::Less
            | TokenKind::Greater
            | TokenKind::LessEqual
            | TokenKind::GreaterEqual => (14, 15, false),
            // TODO(lexer+tokens): adicionar suporte a << e >> no TokenKind e no scanner.
            // Quando existir, usar binding power (16, 17) para ambos.
            TokenKind::Plus | TokenKind::Minus => (18, 19, false),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => (20, 21, false),
            // TODO(lexer+tokens): adicionar token '?' e habilitar ternário com binding power (22, 22).
            _ => return None,
        };
        Some(bp)
    }

    // Tabela de binding power para operadores prefixos.
    fn prefix_binding_power(&self, op: &TokenKind) -> Option<u8> {
        match op {
            TokenKind::Bang
            | TokenKind::Tilde
            | TokenKind::Minus
            | TokenKind::PlusPlus
            | TokenKind::MinusMinus
            | TokenKind::Star
            | TokenKind::Ampersand
            | TokenKind::Sizeof => Some(30),
            _ => None,
        }
    }

    // TODO(cast): por enquanto usa parser de tipo mínimo para cast; expandir com todos os qualificadores de C.
    fn parse_cast_expr(&mut self) -> Result<Expr, CompilerError> {
        let lpar = self
            .expect(&TokenKind::LeftParen, "'(' para iniciar cast")?
            .clone();
        let ty = self.parse_cast_type()?;
        self.expect(&TokenKind::RightParen, "')' após tipo no cast")?;
        let expr = self.parse_expr(30)?;
        let span = self.join_span(self.span_of(&lpar), expr.span());
        Ok(Expr::Cast(ty, Box::new(expr), span))
    }

    // Detecta padrão de cast simples: '(' tipo ... ')'.
    fn looks_like_cast(&self) -> bool {
        if !self.check(&TokenKind::LeftParen) {
            return false;
        }

        let Some(next) = self.tokens.get(self.pos + 1) else {
            return false;
        };

        matches!(
            next.kind,
            TokenKind::Const
                | TokenKind::Unsigned
                | TokenKind::Int
                | TokenKind::Char
                | TokenKind::Float
                | TokenKind::Double
                | TokenKind::Void
                | TokenKind::Struct
        )
    }

    // Parse mínimo de tipo para cast, reaproveitando QualifierType já existente no AST.
    fn parse_cast_type(&mut self) -> Result<QualifierType, CompilerError> {
        let mut is_const = false;
        let mut is_unsigned = false;

        if self.match_kind(&TokenKind::Const) {
            is_const = true;
        }

        if self.match_kind(&TokenKind::Unsigned) {
            is_unsigned = true;
        }

        // TODO(type): integrar com parser de tipos completo quando ele existir.
        let base = match self.peek_kind() {
            TokenKind::Int => {
                self.advance();
                crate::common::ast::ast::Type::Int
            }
            TokenKind::Char => {
                self.advance();
                crate::common::ast::ast::Type::Char
            }
            TokenKind::Float | TokenKind::Double => {
                self.advance();
                crate::common::ast::ast::Type::Float
            }
            TokenKind::Void => {
                self.advance();
                crate::common::ast::ast::Type::Void
            }
            TokenKind::Struct => {
                self.advance();
                let t = self.advance().clone();
                let TokenKind::Identifier(name) = t.kind else {
                    return Err(self.syntax_error(&t, "nome de struct", &format!("{:?}", t.kind)));
                };
                crate::common::ast::ast::Type::Struct(name)
            }
            _ => {
                let found = self.peek().clone();
                return Err(self.syntax_error(
                    &found,
                    "tipo para cast",
                    &format!("{:?}", found.kind),
                ));
            }
        };

        let mut ty = base;
        while self.match_kind(&TokenKind::Star) {
            ty = crate::common::ast::ast::Type::Pointer(Box::new(ty));
        }

        Ok(QualifierType {
            ty,
            is_const,
            is_unsigned,
        })
    }

    // Mapeia token para BinOp da AST já existente.
    fn token_to_bin_op(&self, kind: &TokenKind, found: &Token) -> Result<BinOp, CompilerError> {
        let op = match kind {
            TokenKind::OrOr => BinOp::Or,
            TokenKind::AndAnd => BinOp::And,
            TokenKind::Pipe => BinOp::BitOr,
            TokenKind::Caret => BinOp::BitXor,
            TokenKind::Ampersand => BinOp::BitAnd,
            TokenKind::EqualEqual => BinOp::Eq,
            TokenKind::BangEqual => BinOp::Neq,
            TokenKind::Less => BinOp::Less,
            TokenKind::Greater => BinOp::Greater,
            TokenKind::LessEqual => BinOp::Leq,
            TokenKind::GreaterEqual => BinOp::Geq,
            // TODO(lexer+tokens): mapear Shl/Shr aqui quando os tokens << e >> existirem.
            TokenKind::Plus => BinOp::Add,
            TokenKind::Minus => BinOp::Sub,
            TokenKind::Star => BinOp::Mul,
            TokenKind::Slash => BinOp::Div,
            TokenKind::Percent => BinOp::Mod,
            _ => {
                return Err(self.syntax_error(found, "operador binário", &format!("{:?}", kind)));
            }
        };
        Ok(op)
    }

    // Constrói nós para prefixos suportados.
    fn build_prefix_expr(
        &self,
        op: TokenKind,
        rhs: Expr,
        span: Span,
    ) -> Result<Expr, CompilerError> {
        let expr = match op {
            TokenKind::Bang => Expr::Unary(UnOp::Not, Box::new(rhs), span),
            TokenKind::Minus => Expr::Unary(UnOp::Neg, Box::new(rhs), span),
            TokenKind::Star => Expr::Unary(UnOp::Deref, Box::new(rhs), span),
            TokenKind::Ampersand => Expr::Unary(UnOp::AddrOf, Box::new(rhs), span),
            TokenKind::Tilde => {
                // TODO(ast): criar UnOp::BitNot e substituir encode temporário.
                Expr::Unary(UnOp::Not, Box::new(rhs), span)
            }
            TokenKind::Sizeof => {
                // TODO(ast): criar nó específico de sizeof (por tipo e por expressão).
                Expr::Unary(UnOp::Not, Box::new(rhs), span)
            }
            TokenKind::PlusPlus => {
                // TODO(ast): criar nó prefix increment; encode temporário como atribuição.
                let one = Expr::Literal(Literal::Int(1), span.clone());
                Expr::Assign(
                    Box::new(rhs.clone()),
                    Box::new(Expr::Binary(
                        Box::new(rhs),
                        BinOp::Add,
                        Box::new(one),
                        span.clone(),
                    )),
                    span,
                )
            }
            TokenKind::MinusMinus => {
                // TODO(ast): criar nó prefix decrement; encode temporário como atribuição.
                let one = Expr::Literal(Literal::Int(1), span.clone());
                Expr::Assign(
                    Box::new(rhs.clone()),
                    Box::new(Expr::Binary(
                        Box::new(rhs),
                        BinOp::Sub,
                        Box::new(one),
                        span.clone(),
                    )),
                    span,
                )
            }
            _ => {
                return Err(self.syntax_error_from_span(
                    span,
                    "operador prefixo suportado",
                    &format!("{:?}", op),
                ));
            }
        };

        Ok(expr)
    }

    // Helpers de navegação de token.
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        &self.tokens[self.pos.saturating_sub(1)]
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            return true;
        }
        false
    }

    fn expect(&mut self, kind: &TokenKind, expected: &str) -> Result<&Token, CompilerError> {
        if self.check(kind) {
            return Ok(self.advance());
        }
        let found = self.peek().clone();
        Err(self.syntax_error(&found, expected, &format!("{:?}", found.kind)))
    }

    // Helpers de erro e span.
    fn span_of(&self, token: &Token) -> Span {
        let width = token.span.end.saturating_sub(token.span.start).max(1);
        Span {
            line: token.line,
            column_start: token.col,
            column_end: token.col + width,
        }
    }

    fn join_span(&self, start: Span, end: Span) -> Span {
        if start.line == end.line {
            Span {
                line: start.line,
                column_start: start.column_start,
                column_end: end.column_end,
            }
        } else {
            start
        }
    }

    fn syntax_error(&self, token: &Token, expected: &str, found: &str) -> CompilerError {
        CompilerError::Syntax(SyntaxError {
            span: self.span_of(token),
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }

    fn syntax_error_from_span(&self, span: Span, expected: &str, found: &str) -> CompilerError {
        CompilerError::Syntax(SyntaxError {
            span,
            expected: expected.to_string(),
            found: found.to_string(),
        })
    }
}
