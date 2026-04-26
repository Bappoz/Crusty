use crate::common::ast::ast::QualifierType;
use crate::common::errors::error_data::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Less,
    Greater,
    Leq,
    Geq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Double(f64),
    Char(char),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
    Deref,
    AddrOf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrefixOp {
    Inc,
    Dec,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PostfixOp {
    Inc,
    Dec,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal, Span),
    Ident(String, Span),
    Binary(Box<Expr>, BinOp, Box<Expr>, Span),
    Unary(UnOp, Box<Expr>, Span),
    Prefix(PrefixOp, Box<Expr>, Span),
    Postfix(PostfixOp, Box<Expr>, Span),
    Call(Box<Expr>, Vec<Expr>, Span),
    Cast(QualifierType, Box<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Assign(Box<Expr>, Box<Expr>, Span),
    Sizeof(Box<Expr>, Span),
}

impl Expr {
    /// Retorna o `Span` de código-fonte associado à expressão, independente de seu tipo.
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(_, s) => s.clone(),
            Expr::Ident(_, s) => s.clone(),
            Expr::Binary(_, _, _, s) => s.clone(),
            Expr::Unary(_, _, s) => s.clone(),
            Expr::Prefix(_, _, s) => s.clone(),
            Expr::Postfix(_, _, s) => s.clone(),
            Expr::Call(_, _, s) => s.clone(),
            Expr::Cast(_, _, s) => s.clone(),
            Expr::Index(_, _, s) => s.clone(),
            Expr::Assign(_, _, s) => s.clone(),
            Expr::Sizeof(_, s) => s.clone(),
        }
    }
}
