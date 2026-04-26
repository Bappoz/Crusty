use crate::common::ast::{ast::QualifierType, expr::Expr};
use crate::common::errors::error_data::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Block(Vec<Stmt>, Span),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>, Span),
    While(Expr, Box<Stmt>, Span),
    For(
        Option<Box<Stmt>>,
        Option<Expr>,
        Option<Expr>,
        Box<Stmt>,
        Span,
    ), // For(Init, Cond, Inc, Body, Span)
    Break(Span),
    Continue(Span),
    ExprStmt(Expr, Span),
    Return(Option<Expr>, Span),
    VarDecl(QualifierType, String, Option<Expr>, Span),
}

impl Stmt {
    /// Retorna o `Span` de código-fonte associado ao statement, independente de seu tipo.
    pub fn span(&self) -> Span {
        match self {
            Stmt::Block(_, s) => s.clone(),
            Stmt::If(_, _, _, s) => s.clone(),
            Stmt::While(_, _, s) => s.clone(),
            Stmt::For(_, _, _, _, s) => s.clone(),
            Stmt::Break(s) => s.clone(),
            Stmt::Continue(s) => s.clone(),
            Stmt::ExprStmt(_, s) => s.clone(),
            Stmt::Return(_, s) => s.clone(),
            Stmt::VarDecl(_, _, _, s) => s.clone(),
        }
    }
}
