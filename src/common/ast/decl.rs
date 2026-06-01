use crate::common::ast::{ast::QualifierType, expr::Expr, stmt::Stmt};
use crate::common::errors::error_data::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Function(
        QualifierType,
        String,
        Vec<(QualifierType, String)>,
        Vec<Stmt>,
        Span,
    ),
    GlobalVar(QualifierType, String, Option<Expr>, Span),
    StructDecl(String, Vec<(QualifierType, String)>, Span),
    EnumDecl(String, Vec<(String, Option<Expr>)>, Span),
    Typedef(QualifierType, String, Span),
}

impl Decl {
    /// Retorna o `Span` de código-fonte associado à declaração, independente de seu tipo.
    pub fn span(&self) -> Span {
        match self {
            Decl::Function(_, _, _, _, s) => s.clone(),
            Decl::GlobalVar(_, _, _, s) => s.clone(),
            Decl::StructDecl(_, _, s) => s.clone(),
            Decl::EnumDecl(_, _, s) => s.clone(),
            Decl::Typedef(_, _, s) => s.clone(),
        }
    }
}
