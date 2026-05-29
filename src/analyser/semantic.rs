use crate::analyser::symbol_table::{Symbol, SymbolTable};
use crate::common::ast::ast::{Program, QualifierType, Type};
use crate::common::ast::decl::Decl;
use crate::common::ast::expr::{Expr, Literal, MemberAccess};
use crate::common::ast::stmt::Stmt;
use crate::common::errors::types::{CompilerError, SemanticError, SemanticErrorKind};

#[derive(Default, Debug)]
pub struct SemanticAnalyser {
    pub sym: SymbolTable,
    /// Tipo de retorno da função sendo analisada no momento; `None` fora de funções.
    pub current_fn_ret: Option<QualifierType>,
    pub diagnostics: Vec<CompilerError>,
}

impl SemanticAnalyser {
    pub fn new() -> Self {
        Self {
            sym: SymbolTable::new(),
            current_fn_ret: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn analyse_program(&mut self, prog: &Program) {
        self.sym.enter_scope();
        for decl in &prog.decls {
            self.analyse_decl(decl);
        }
        self.sym.exit_scope();
    }

    pub fn analyse_decl(&mut self, decl: &Decl) {
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
                if let Err(e) = self.sym.declare(symbol) {
                    self.diagnostics.push(e);
                }
            }
            Decl::StructDecl(name, fields, _) => {
                self.sym.register_struct(name.clone(), fields.clone());
            }
            Decl::Function(return_type, _name, params, body, span) => {
                self.sym.enter_scope();
                let prev_ret = self.current_fn_ret.replace(return_type.clone());

                for (qty, name) in params {
                    let symbol = Symbol {
                        name: name.clone(),
                        ty: qty.clone(),
                        mutable: !qty.is_const,
                        decl_span: span.clone(),
                    };
                    if let Err(e) = self.sym.declare(symbol) {
                        self.diagnostics.push(e);
                    }
                }

                for stmt in body {
                    self.analyse_stmt(stmt);
                }

                self.current_fn_ret = prev_ret;
                self.sym.exit_scope();
            }
        }
    }

    pub fn analyse_stmt(&mut self, stmt: &Stmt) {
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
                if let Err(e) = self.sym.declare(symbol) {
                    self.diagnostics.push(e);
                }
            }
            Stmt::Block(stmts, _) => {
                self.sym.enter_scope();
                for s in stmts {
                    self.analyse_stmt(s);
                }
                self.sym.exit_scope();
            }
            Stmt::ExprStmt(expr, _) => {
                self.analyse_expr(expr);
            }
            Stmt::Return(expr, _) => {
                if let Some(e) = expr {
                    self.analyse_expr(e);
                    // TODO(#87): checar compatibilidade com current_fn_ret
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
                self.sym.enter_scope();
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
                self.sym.exit_scope();
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

    /// Analisa uma expressão e retorna o tipo inferido.
    /// Tipos não resolvidos retornam `Type::Void` como sentinela; TODO(#87): expansão completa.
    pub fn analyse_expr(&mut self, expr: &Expr) -> QualifierType {
        match expr {
            Expr::Literal(lit, _) => infer_literal_type(lit),
            Expr::Ident(name, span) => match self.sym.lookup(name) {
                Some(sym) => sym.ty.clone(),
                None => {
                    self.diagnostics
                        .push(CompilerError::Semantic(SemanticError {
                            span: span.clone(),
                            kind: SemanticErrorKind::UndefinedVariable(name.clone()),
                        }));
                    unknown_type()
                }
            },
            Expr::Assign(lhs, rhs, span) => {
                if let Expr::Ident(name, _) = lhs.as_ref() {
                    if let Some(sym) = self.sym.lookup(name) {
                        if !sym.mutable {
                            self.diagnostics
                                .push(CompilerError::Semantic(SemanticError {
                                    span: span.clone(),
                                    kind: SemanticErrorKind::AssignToConst(name.clone()),
                                }));
                        }
                    }
                }
                let lhs_ty = self.analyse_expr(lhs);
                self.analyse_expr(rhs);
                lhs_ty
            }
            Expr::Binary(l, _, r, _) => {
                let lhs_ty = self.analyse_expr(l);
                self.analyse_expr(r);
                // TODO(#87): resolver tipo resultante com base no operador
                lhs_ty
            }
            Expr::Unary(_, e, _) | Expr::Prefix(_, e, _) | Expr::Postfix(_, e, _) => {
                self.analyse_expr(e)
            }
            Expr::CompoundAssign(_, lhs, rhs, _) => {
                let lhs_ty = self.analyse_expr(lhs);
                self.analyse_expr(rhs);
                lhs_ty
            }
            Expr::Cast(qty, e, _) => {
                self.analyse_expr(e);
                qty.clone()
            }
            Expr::Sizeof(e, _) => {
                self.analyse_expr(e);
                uint_type()
            }
            Expr::SizeofType(_, _) => uint_type(),
            Expr::Call(callee, args, _) => {
                self.analyse_expr(callee);
                for a in args {
                    self.analyse_expr(a);
                }
                // TODO(#87): lookup do tipo de retorno da função
                unknown_type()
            }
            Expr::Index(arr, idx, _) => {
                let arr_ty = self.analyse_expr(arr);
                self.analyse_expr(idx);
                // TODO(#87): desreferenciar o tipo do array/ponteiro
                match arr_ty.ty {
                    Type::Array(inner) | Type::Pointer(inner) => QualifierType {
                        ty: *inner,
                        is_const: arr_ty.is_const,
                        is_unsigned: arr_ty.is_unsigned,
                    },
                    _ => unknown_type(),
                }
            }
            Expr::Ternary(cond, then, else_, _) => {
                self.analyse_expr(cond);
                let then_ty = self.analyse_expr(then);
                self.analyse_expr(else_);
                // TODO(#87): verificar compatibilidade de then/else
                then_ty
            }
            Expr::Member(obj, access_kind, field_name, span) => {
                let left_type = self.analyse_expr(obj);

                let struct_name = match access_kind {
                    MemberAccess::Direct => match &left_type.ty {
                        Type::Struct(name) => name.clone(),
                        _ => {
                            self.diagnostics
                                .push(CompilerError::Semantic(SemanticError {
                                    span: span.clone(),
                                    kind: SemanticErrorKind::TypeMismatch {
                                        expected: "Struct".to_string(),
                                        found: format!("{:?}", left_type.ty),
                                    },
                                }));
                            return unknown_type();
                        }
                    },
                    MemberAccess::Pointer => match &left_type.ty {
                        Type::Pointer(inner) => match &**inner {
                            Type::Struct(name) => name.clone(),
                            _ => {
                                self.diagnostics
                                    .push(CompilerError::Semantic(SemanticError {
                                        span: span.clone(),
                                        kind: SemanticErrorKind::TypeMismatch {
                                            expected: "*Struct".to_string(),
                                            found: format!("{:?}", left_type.ty),
                                        },
                                    }));
                                return unknown_type();
                            }
                        },
                        _ => {
                            self.diagnostics
                                .push(CompilerError::Semantic(SemanticError {
                                    span: span.clone(),
                                    kind: SemanticErrorKind::TypeMismatch {
                                        expected: "Pointer".to_string(),
                                        found: format!("{:?}", left_type.ty),
                                    },
                                }));
                            return unknown_type();
                        }
                    },
                };

                let fields = match self.sym.lookup_struct(&struct_name) {
                    Some(f) => f.to_vec(),
                    None => {
                        self.diagnostics
                            .push(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::UndefinedStruct(struct_name.clone()),
                            }));
                        return unknown_type();
                    }
                };

                match fields.iter().find(|(_, name)| name == field_name) {
                    Some((field_type, _)) => field_type.clone(),
                    None => {
                        self.diagnostics
                            .push(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::FieldNotFound {
                                    struct_name: struct_name.clone(),
                                    field_name: field_name.clone(),
                                },
                            }));
                        unknown_type()
                    }
                }
            }
        }
    }
}

/// API pública: analisa o programa e retorna todos os diagnósticos semânticos.
pub fn analyse(prog: &Program) -> Vec<CompilerError> {
    let mut analyser = SemanticAnalyser::new();
    analyser.analyse_program(prog);
    analyser.diagnostics
}

fn infer_literal_type(lit: &Literal) -> QualifierType {
    let ty = match lit {
        Literal::Int(_) => Type::Int,
        Literal::Double(_) => Type::Double,
        Literal::Char(_) => Type::Char,
        Literal::String(_) => Type::Pointer(Box::new(Type::Char)),
    };
    QualifierType {
        ty,
        is_const: false,
        is_unsigned: false,
    }
}

fn unknown_type() -> QualifierType {
    QualifierType {
        ty: Type::Void,
        is_const: false,
        is_unsigned: false,
    }
}

fn uint_type() -> QualifierType {
    QualifierType {
        ty: Type::Int,
        is_const: false,
        is_unsigned: true,
    }
}
