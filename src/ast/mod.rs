#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Char,
    Float,
    Void,
    Array,
    Pointer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span{
    pub line_start: u32,
    pub col_start: u32,
    pub line_end: u32,
    pub col_end: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr{
    Literal(i32, Span),
    Ident(String, Span),
    Binary(Box<Expr>, char, Box<Expr>, Span),
    Unary(char,Box<Expr>, Span),
    Call(String, Vec<Expr>, Span),
    Cast(Type, Box<Expr>, Span),
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

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt{
    Block(Vec<Stmt>, Span), // Conteúdo entre as chaves
    If(Expr, Box<Stmt>, Option<Box<Stmt>>, Span),
    While(Expr, Box<Stmt>, Span),
    ExprStmt(Expr, Span), // Qualquer expressão que vira instrução
    Return(Option<Expr>, Span),
    VarDecl(Type, String, Option<Expr>, Span),
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


#[derive(Debug, Clone, PartialEq)]
pub enum Decl{
    Function(Type, String, Vec<(Type,String)>, Box<Stmt>, Span),
    GlobalVec(Type, String, Option<Expr>, Span),
    StructDecl(String, Vec<(String, Type)>, Span),
}

impl Decl{
    pub fn span(&self) -> Span{
        match self{
            Decl::Function(_,_,_,_,s) => s.clone(),
            Decl::GlobalVec(_,_,_,s) => s.clone(),
            Decl::StructDecl(_,_,s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program{
    pub decls: Vec<Decl>,
}

#[cfg(test)]
mod test{
    use super::*; //sobe onível de hierarquia

    #[test]
fn ast_nodes_are_debug_printable() {
        let span = Span { line_start: 1, col_start: 1, line_end: 1, col_end: 5 };
        // Criando um literal "10"
        let expr = Expr::Literal(10, span.clone());
        
        println!("{:?}", expr);
        assert_eq!(expr.span().line_start, 1);
    }

    #[test]
    fn binary_expr_span_covers_both_operands() {
        let span1 = Span { line_start: 1, col_start: 1, line_end: 1, col_end: 2 };
        let span2 = Span { line_start: 1, col_start: 5, line_end: 1, col_end: 6 };
        let span_total = Span { line_start: 1, col_start: 1, line_end: 1, col_end: 6 };

        let left = Box::new(Expr::Literal(1, span1));
        let right = Box::new(Expr::Literal(2, span2));
        
        // 1 + 2
        let binary = Expr::Binary(left, '+', right, span_total.clone());
        
        assert_eq!(binary.span(), span_total);
    }
}