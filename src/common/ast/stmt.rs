use crate::common::ast::{
    ast::QualifierType,
    expr::Expr,
};
use crate::common::errors::error_data::Span;


#[derive(Debug, Clone, PartialEq)]
pub enum Stmt{
    Block(Vec<Stmt>, Span), // Conteúdo entre as chaves
    If(Expr, Box<Stmt>, Option<Box<Stmt>>, Span),
    While(Expr, Box<Stmt>, Span),
    ExprStmt(Expr, Span), // Qualquer expressão que vira instrução
    Return(Option<Expr>, Span),
    VarDecl(QualifierType, String, Option<Expr>, Span),
}

impl Stmt{
    pub fn span(&self) -> Span{
        match self{
            Stmt::Block(_,s) => s.clone(),
            Stmt::If(_,_,_,s) => s.clone(),
            Stmt::While(_,_,s) => s.clone(),
            Stmt::ExprStmt(_,s) => s.clone(),
            Stmt::Return(_,s) => s.clone(),
            Stmt::VarDecl(_,_,_,s) => s.clone(),
        }
    }
}
