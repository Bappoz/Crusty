pub mod symbol_table;

use crate::analyser::symbol_table::{Symbol, SymbolTable};
use crate::common::ast::ast::Program;
use crate::common::ast::decl::Decl;
use crate::common::ast::expr::Expr;
use crate::common::ast::stmt::Stmt;
use crate::common::errors::types::{CompilerError, SemanticError, SemanticErrorKind};

pub struct Analyser {
    pub symbols: SymbolTable,
    pub diagnostics: Vec<CompilerError>,
}

impl Analyser {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Analisa um programa completo. Retorna `Ok(())` se sem erros semânticos.
    pub fn analyse_program(&mut self, program: &Program) -> Result<(), Vec<CompilerError>> {
        self.symbols.enter_scope();
        for decl in &program.decls {
            self.analyse_decl(decl);
        }
        self.symbols.exit_scope();

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(std::mem::take(&mut self.diagnostics))
        }
    }

    fn analyse_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::GlobalVar(qty, name, init, span) => {
                if let Some(expr) = init {
                    self.analyse_expr(expr);
                }
                let symbol = Symbol {
                    name: name.clone(),
                    ty: qty.clone(),
                    mutable: !qty.is_const,
                    decl_span: span.clone(),
                };
                if let Err(e) = self.symbols.declare(symbol) {
                    self.diagnostics.push(e);
                }
            }
            Decl::StructDecl(_, _, _) => {}
            Decl::Function(_, _, params, body, span) => {
                self.symbols.enter_scope();
                for (qty, name) in params {
                    let symbol = Symbol {
                        name: name.clone(),
                        ty: qty.clone(),
                        mutable: !qty.is_const,
                        decl_span: span.clone(),
                    };
                    if let Err(e) = self.symbols.declare(symbol) {
                        self.diagnostics.push(e);
                    }
                }
                for stmt in body {
                    self.analyse_stmt(stmt);
                }
                self.symbols.exit_scope();
            }
        }
    }

    fn analyse_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(qty, name, init, span) => {
                if let Some(expr) = init {
                    self.analyse_expr(expr);
                }
                let symbol = Symbol {
                    name: name.clone(),
                    ty: qty.clone(),
                    mutable: !qty.is_const,
                    decl_span: span.clone(),
                };
                if let Err(e) = self.symbols.declare(symbol) {
                    self.diagnostics.push(e);
                }
            }
            Stmt::Block(stmts, _) => {
                self.symbols.enter_scope();
                for s in stmts {
                    self.analyse_stmt(s);
                }
                self.symbols.exit_scope();
            }
            Stmt::ExprStmt(expr, _) => self.analyse_expr(expr),
            Stmt::Return(expr, _) => {
                if let Some(e) = expr {
                    self.analyse_expr(e);
                }
            }
            Stmt::If(cond, then, else_, _) => {
                self.analyse_expr(cond);
                self.analyse_stmt(then);
                if let Some(e) = else_ {
                    self.analyse_stmt(e);
                }
            }
            Stmt::While(cond, body, _) => {
                self.analyse_expr(cond);
                self.analyse_stmt(body);
            }
            Stmt::DoWhile(cond, body, _) => {
                self.analyse_stmt(body);
                self.analyse_expr(cond);
            }
            Stmt::For(init, cond, inc, body, _) => {
                self.symbols.enter_scope();
                if let Some(s) = init {
                    self.analyse_stmt(s);
                }
                if let Some(e) = cond {
                    self.analyse_expr(e);
                }
                if let Some(e) = inc {
                    self.analyse_expr(e);
                }
                self.analyse_stmt(body);
                self.symbols.exit_scope();
            }
            Stmt::Switch(expr, cases, _) => {
                self.analyse_expr(expr);
                for case in cases {
                    for s in &case.stmts {
                        self.analyse_stmt(s);
                    }
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
        }
    }

    fn analyse_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(name, span) => {
                if self.symbols.lookup(name).is_none() {
                    self.diagnostics
                        .push(CompilerError::Semantic(SemanticError {
                            span: span.clone(),
                            kind: SemanticErrorKind::UndefinedVariable(name.clone()),
                        }));
                }
            }
            Expr::Assign(lhs, rhs, span) => {
                if let Expr::Ident(name, _) = lhs.as_ref() {
                    if let Some(sym) = self.symbols.lookup(name) {
                        if !sym.mutable {
                            self.diagnostics
                                .push(CompilerError::Semantic(SemanticError {
                                    span: span.clone(),
                                    kind: SemanticErrorKind::AssignToConst(name.clone()),
                                }));
                        }
                    }
                }
                self.analyse_expr(lhs);
                self.analyse_expr(rhs);
            }
            Expr::Binary(l, _, r, _) => {
                self.analyse_expr(l);
                self.analyse_expr(r);
            }
            Expr::Unary(_, e, _) | Expr::Prefix(_, e, _) | Expr::Postfix(_, e, _) => {
                self.analyse_expr(e);
            }
            Expr::Cast(_, e, _) | Expr::Sizeof(e, _) => self.analyse_expr(e),
            Expr::Call(callee, args, _) => {
                self.analyse_expr(callee);
                for a in args {
                    self.analyse_expr(a);
                }
            }
            Expr::Index(arr, idx, _) => {
                self.analyse_expr(arr);
                self.analyse_expr(idx);
            }
            Expr::Ternary(cond, then, else_, _) => {
                self.analyse_expr(cond);
                self.analyse_expr(then);
                self.analyse_expr(else_);
            }
            Expr::Member(obj, _, _, _) => self.analyse_expr(obj),
            Expr::Literal(_, _) | Expr::SizeofType(_, _) => {}
        }
    }
}
