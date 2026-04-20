use crate::common::ast::{ast::QualifierType, expr::Expr, stmt::Stmt};
use crate::common::errors::error_data::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Function(
        QualifierType,
        String,
        Vec<(QualifierType, String)>,
        Vec<Stmt>, // Mudei para vec, ja que o body é sempre um bloco.
        Span,
    ),
    GlobalVar(QualifierType, String, Option<Expr>, Span),
    StructDecl(String, Vec<(QualifierType, String)>, Span),
}

impl Decl {
    pub fn span(&self) -> Span {
        match self {
            Decl::Function(_, _, _, _, s) => s.clone(),
            Decl::GlobalVar(_, _, _, s) => s.clone(), //Troca de nome para melhor identificação
            Decl::StructDecl(_, _, s) => s.clone(),
        }
    }
}
