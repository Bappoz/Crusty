use crate::common::ast::ast::QualifierType;
use crate::common::errors::error_data::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp{
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Less,
    Greater,
    Leq,
    Geq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal{
    Int(i32),
    FLoat(f32),
    Char(char),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp{
    Neg,
    Not,
    Deref,
    AddrOf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr{
    Literal(Literal, Span), // não são só numeros
    Ident(String, Span),
    Binary(Box<Expr>, BinOp, Box<Expr>, Span), // char so passa um caractere
    Unary(UnOp, Box<Expr>, Span),
    Call(String, Vec<Expr>, Span),
    Cast(QualifierType, Box<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
}

impl Expr{
    pub fn span(&self) -> Span{
        match self{
            Expr::Literal(_,s) => s.clone(),
            Expr::Ident(_,s) => s.clone(),
            Expr::Binary(_,_,_,s) => s.clone(),
            Expr::Unary(_,_,s) => s.clone(),
            Expr::Call(_,_,s) => s.clone(),
            Expr::Cast(_,_,s) => s.clone(),
            Expr::Index(_,_,s) => s.clone(),
        }
    }
}