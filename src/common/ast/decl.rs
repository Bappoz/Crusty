
use crate::common::ast::{
    ast::{Type,QualifierType},
    stmt::Stmt,
    expr::Expr,
};
use crate::common::errors::error_data::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Decl{
    Function(QualifierType, String, Vec<(QualifierType,String)>, Box<Stmt>, Span),
    GlobalVar(QualifierType, String, Option<Expr>, Span),
    StructDecl(String, Vec<(String, Type)>, Span),
}

impl Decl{
    pub fn span(&self) -> Span{
        match self{
            Decl::Function(_,_,_,_,s) => s.clone(),
            Decl::GlobalVar(_,_,_,s) => s.clone(), //Troca de nome para melhor identificação
            Decl::StructDecl(_,_,s) => s.clone(),
        }
    }
}