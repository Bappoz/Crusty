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
                let resolved_qty = self.resolve_type(qty);
                if let Some(expr) = init {
                    let init_ty = self.analyse_expr(expr);
                    if !types_compatible_for_assign(&resolved_qty.ty, &init_ty.ty) {
                        self.diagnostics
                            .push(CompilerError::Semantic(SemanticError {
                                span: expr.span(),
                                kind: SemanticErrorKind::TypeMismatch {
                                    expected: type_name(&resolved_qty.ty),
                                    found: type_name(&init_ty.ty),
                                },
                            }));
                    }
                }
                let symbol = Symbol {
                    name: name.clone(),
                    ty: resolved_qty.clone(),
                    mutable: !resolved_qty.is_const,
                    params: None,
                    decl_span: span.clone(),
                };
                if let Err(e) = self.sym.declare(symbol) {
                    self.diagnostics.push(e);
                }
            }
            Decl::StructDecl(name, fields, _) => {
                self.sym.register_struct(name.clone(), fields.clone());
            }
            Decl::EnumDecl(_, variants, enum_span) => {
                for (variant_name, value) in variants {
                    if let Some(expr) = value {
                        self.analyse_expr(expr);
                    }

                    let symbol = Symbol {
                        name: variant_name.clone(),
                        ty: QualifierType {
                            ty: Type::Int,
                            is_const: true,
                            is_unsigned: false,
                        },
                        mutable: false,
                        params: None,
                        decl_span: value
                            .as_ref()
                            .map(|expr| expr.span())
                            .unwrap_or_else(|| enum_span.clone()),
                    };

                    if let Err(e) = self.sym.declare(symbol) {
                        self.diagnostics.push(e);
                    }
                }
            }
            Decl::Typedef(qty, name, _) => {
                let resolved_qty = self.resolve_type(qty);
                self.sym.register_type_alias(name.clone(), resolved_qty);
            }
            Decl::Function(return_type, name, params, body, span) => {
                let resolved_ret = self.resolve_type(return_type);
                let param_tys: Vec<QualifierType> = params
                    .iter()
                    .map(|(qty, _)| self.resolve_type(qty))
                    .collect();

                let function_symbol = Symbol {
                    name: name.clone(),
                    ty: resolved_ret.clone(),
                    mutable: false,
                    params: Some(param_tys.clone()),
                    decl_span: span.clone(),
                };

                if let Err(e) = self.sym.declare(function_symbol) {
                    self.diagnostics.push(e);
                }

                self.sym.enter_scope();
                let prev_ret = self.current_fn_ret.replace(resolved_ret);

                for (qty, name) in params {
                    let resolved_qty = self.resolve_type(qty);
                    let symbol = Symbol {
                        name: name.clone(),
                        ty: resolved_qty.clone(),
                        mutable: !resolved_qty.is_const,
                        params: None,
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
                let resolved_qty = self.resolve_type(qty);
                let symbol = Symbol {
                    name: name.clone(),
                    ty: resolved_qty.clone(),
                    mutable: !resolved_qty.is_const,
                    params: None,
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
            Stmt::Return(expr, span) => {
                let expected_fn_type = self.current_fn_ret.clone();

                match (expected_fn_type, expr) {
                    (Some(expected), Some(e)) if expected.ty == Type::Void => {
                        self.analyse_expr(e);
                        self.diagnostics
                            .push(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::ReturnInVoid,
                            }));
                    }
                    (Some(expected), Some(e)) => {
                        let found_expr_qty = self.analyse_expr(e);
                        if expected.ty != found_expr_qty.ty {
                            self.diagnostics
                                .push(CompilerError::Semantic(SemanticError {
                                    span: span.clone(),
                                    kind: SemanticErrorKind::TypeMismatch {
                                        expected: type_name(&expected.ty),
                                        found: type_name(&found_expr_qty.ty),
                                    },
                                }));
                        }
                    }
                    (Some(expected), None) if expected.ty != Type::Void => {
                        self.diagnostics
                            .push(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::TypeMismatch {
                                    expected: type_name(&expected.ty),
                                    found: "void (retorno vazio)".to_string(),
                                },
                            }));
                    }
                    _ => {}
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

    /// Analisa uma expressão, verifica compatibilidade de tipos e retorna o tipo inferido.
    /// Tipos não resolvíveis retornam `Type::Void` como sentinela.
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
                let rhs_ty = self.analyse_expr(rhs);
                if !types_compatible_for_assign(&lhs_ty.ty, &rhs_ty.ty) {
                    self.diagnostics
                        .push(CompilerError::Semantic(SemanticError {
                            span: span.clone(),
                            kind: SemanticErrorKind::TypeMismatch {
                                expected: type_name(&lhs_ty.ty),
                                found: type_name(&rhs_ty.ty),
                            },
                        }));
                }
                lhs_ty
            }
            Expr::Binary(l, op, r, span) => {
                let lhs_ty = self.analyse_expr(l);
                let rhs_ty = self.analyse_expr(r);
                match binary_result_type(&lhs_ty.ty, op, &rhs_ty.ty) {
                    Ok(result_ty) => result_ty,
                    Err((expected, found)) => {
                        self.diagnostics
                            .push(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::TypeMismatch { expected, found },
                            }));
                        unknown_type()
                    }
                }
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
                self.resolve_type(qty)
            }
            Expr::Sizeof(e, _) => {
                self.analyse_expr(e);
                uint_type()
            }
            Expr::SizeofType(_, _) => uint_type(),
            Expr::Call(callee, args, span) => {
                if let Expr::Ident(name, id_span) = callee.as_ref() {
                    match self.sym.lookup(name) {
                        None => {
                            self.diagnostics
                                .push(CompilerError::Semantic(SemanticError {
                                    span: id_span.clone(),
                                    kind: SemanticErrorKind::UndefinedVariable(name.clone()),
                                }));
                            for a in args {
                                self.analyse_expr(a);
                            }
                            return unknown_type();
                        }
                        Some(sym) => {
                            let params_clone = sym.params.clone();
                            let ret_ty_clone = sym.ty.clone();

                            match params_clone {
                                None => {
                                    self.diagnostics
                                        .push(CompilerError::Semantic(SemanticError {
                                            span: id_span.clone(),
                                            kind: SemanticErrorKind::CallNonFunction(name.clone()),
                                        }));
                                    for a in args {
                                        self.analyse_expr(a);
                                    }
                                    return unknown_type();
                                }
                                Some(param_types) => {
                                    if param_types.len() != args.len() {
                                        self.diagnostics.push(CompilerError::Semantic(
                                            SemanticError {
                                                span: span.clone(),
                                                kind: SemanticErrorKind::ArityMismatch {
                                                    expected: param_types.len(),
                                                    found: args.len(),
                                                },
                                            },
                                        ));
                                    }

                                    for (i, a) in args.iter().enumerate() {
                                        let arg_ty = self.analyse_expr(a);
                                        if i < param_types.len() {
                                            let param_ty = &param_types[i];
                                            if !types_compatible_for_assign(
                                                &param_ty.ty,
                                                &arg_ty.ty,
                                            ) {
                                                self.diagnostics.push(CompilerError::Semantic(
                                                    SemanticError {
                                                        span: a.span(),
                                                        kind: SemanticErrorKind::TypeMismatch {
                                                            expected: type_name(&param_ty.ty),
                                                            found: type_name(&arg_ty.ty),
                                                        },
                                                    },
                                                ));
                                            }
                                        }
                                    }

                                    return QualifierType {
                                        ty: ret_ty_clone.ty,
                                        is_const: ret_ty_clone.is_const,
                                        is_unsigned: ret_ty_clone.is_unsigned,
                                    };
                                }
                            }
                        }
                    }
                }

                self.analyse_expr(callee);
                for a in args {
                    self.analyse_expr(a);
                }
                unknown_type()
            }
            Expr::Index(arr, idx, span) => {
                let arr_ty = self.analyse_expr(arr);
                let idx_ty = self.analyse_expr(idx);

                let is_integer =
                    |t: &Type| matches!(t, Type::Int | Type::Long | Type::Short | Type::Char);
                if !is_integer(&idx_ty.ty) {
                    self.diagnostics
                        .push(CompilerError::Semantic(SemanticError {
                            span: span.clone(),
                            kind: SemanticErrorKind::InvalidIndexType {
                                found: type_name(&idx_ty.ty),
                            },
                        }));
                }

                match arr_ty.ty {
                    Type::Array(inner) | Type::Pointer(inner) => QualifierType {
                        ty: *inner,
                        is_const: arr_ty.is_const,
                        is_unsigned: arr_ty.is_unsigned,
                    },
                    _ => {
                        self.diagnostics
                            .push(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::NotIndexable {
                                    found: type_name(&arr_ty.ty),
                                },
                            }));
                        unknown_type()
                    }
                }
            }
            Expr::Ternary(cond, then, else_, span) => {
                let cond_ty = self.analyse_expr(cond);
                // 1. A condição deve ser um tipo escalar (numérico ou ponteiro)
                if !is_numeric(&cond_ty.ty) && !is_pointer(&cond_ty.ty) {
                    self.diagnostics
                        .push(CompilerError::Semantic(SemanticError {
                            span: cond.span(),
                            kind: SemanticErrorKind::TypeMismatch {
                                expected: "scalar".to_string(),
                                found: type_name(&cond_ty.ty),
                            },
                        }));
                }

                let then_ty = self.analyse_expr(then);
                let else_ty = self.analyse_expr(else_);

                // 2. Verificar compatibilidade entre os ramos then/else
                if !ternary_branches_compatible(&then_ty.ty, &else_ty.ty) {
                    self.diagnostics
                        .push(CompilerError::Semantic(SemanticError {
                            span: span.clone(),
                            kind: SemanticErrorKind::TypeMismatch {
                                expected: type_name(&then_ty.ty),
                                found: type_name(&else_ty.ty),
                            },
                        }));
                    return unknown_type();
                }

                // 3. Retornar o tipo promovido
                ternary_result_type(&then_ty, &else_ty)
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

    fn resolve_type(&self, qty: &QualifierType) -> QualifierType {
        QualifierType {
            ty: self.resolve_type_inner(&qty.ty),
            is_const: qty.is_const,
            is_unsigned: qty.is_unsigned,
        }
    }

    fn resolve_type_inner(&self, ty: &Type) -> Type {
        match ty {
            Type::Array(inner) => Type::Array(Box::new(self.resolve_type_inner(inner))),
            Type::Pointer(inner) => Type::Pointer(Box::new(self.resolve_type_inner(inner))),
            Type::Alias(name) => self
                .sym
                .lookup_type_alias(name)
                .map(|resolved| self.resolve_type(resolved).ty)
                .unwrap_or_else(|| Type::Alias(name.clone())),
            other => other.clone(),
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

// ── Type helpers ────────────────────────────────────────────────────────────

/// Retorna `true` se o tipo é numérico (inteiro ou ponto flutuante).
fn is_numeric(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Int | Type::Long | Type::Short | Type::Char | Type::Float | Type::Double
    )
}

/// Retorna `true` se o tipo é ponteiro ou array (array decai para ponteiro em C).
fn is_pointer(ty: &Type) -> bool {
    matches!(ty, Type::Pointer(_) | Type::Array(_))
}

/// Converte `Type` em string legível para mensagens de erro.
fn type_name(ty: &Type) -> String {
    match ty {
        Type::Int => "int".into(),
        Type::Long => "long".into(),
        Type::Short => "short".into(),
        Type::Char => "char".into(),
        Type::Float => "float".into(),
        Type::Double => "double".into(),
        Type::Void => "void".into(),
        Type::Pointer(inner) => format!("{}*", type_name(inner)),
        Type::Array(inner) => format!("{}[]", type_name(inner)),
        Type::Struct(n) => format!("struct {}", n),
        Type::Enum(n) => format!("enum {}", n),
        Type::Alias(n) => n.clone(),
    }
}

/// Promoção numérica implícita de C: Double > Float > Long > Int > Short/Char.
fn numeric_promotion(a: &Type, b: &Type) -> Type {
    use Type::*;
    match (a, b) {
        (Double, _) | (_, Double) => Double,
        (Float, _) | (_, Float) => Float,
        (Long, _) | (_, Long) => Long,
        _ => Int,
    }
}

/// Verifica se `rhs` é atribuível a uma variável do tipo `lhs`.
///
/// Regras (subconjunto de C):
/// - Tipos iguais → sempre compatível.
/// - Numérico ← Numérico → compatível (coerção implícita).
/// - `T*` ← `T*` (mesmo inner) → compatível.
/// - Qualquer outro par → incompatível.
fn types_compatible_for_assign(lhs: &Type, rhs: &Type) -> bool {
    if lhs == rhs {
        return true;
    }
    if is_numeric(lhs) && is_numeric(rhs) {
        return true;
    }
    if let (Type::Pointer(l_inner), Type::Pointer(r_inner)) = (lhs, rhs) {
        return l_inner == r_inner;
    }
    false
}

/// Verifica compatibilidade entre os ramos `then`/`else` de um ternário.
///
/// Segue as regras do C:
/// - Tipos iguais → compatível.
/// - Ambos numéricos → compatível (promoção implícita).
/// - Ambos ponteiros do mesmo tipo → compatível.
///
/// Nota: a regra do null pointer constant (`T* : 0`) exigiria análise de
/// valor constante (constant folding) e está fora do escopo desta verificação.
fn ternary_branches_compatible(a: &Type, b: &Type) -> bool {
    if a == b {
        return true;
    }
    if is_numeric(a) && is_numeric(b) {
        return true;
    }
    if let (Type::Pointer(ai), Type::Pointer(bi)) = (a, b) {
        return ai == bi;
    }
    false
}

/// Determina o tipo resultante do operador ternário a partir dos tipos dos ramos.
///
/// - Ambos numéricos → promoção numérica dominante (Double > Float > Long > Int).
/// - Demais casos (tipos iguais, ponteiros iguais) → tipo do ramo `then`.
fn ternary_result_type(then_ty: &QualifierType, else_ty: &QualifierType) -> QualifierType {
    let make = |ty: Type| QualifierType {
        ty,
        is_const: false,
        is_unsigned: false,
    };
    if is_numeric(&then_ty.ty) && is_numeric(&else_ty.ty) {
        return make(numeric_promotion(&then_ty.ty, &else_ty.ty));
    }
    then_ty.clone()
}

/// Calcula o tipo resultante de `lhs op rhs`.
///
/// Retorna `Ok(QualifierType)` se a operação é válida ou
/// `Err((expected, found))` com strings de diagnóstico se inválida.
///
/// Regras (subconjunto de C):
/// - `+`, `-`: numérico OP numérico → promoção; ponteiro ± inteiro → ponteiro.
/// - `*`, `/`: numérico OP numérico → promoção.
/// - `%`: **inteiro** OP **inteiro** (float proibido) → promoção.
/// - Bitwise (`&`, `|`, `^`, `<<`, `>>`): **inteiro** OP **inteiro** → promoção.
/// - Relacionais (`==`, `!=`, `<`, `>`, `<=`, `>=`): num↔num ou ptr↔ptr → `Int`.
/// - Lógicos (`&&`, `||`): escalar OP escalar → `Int`.
fn binary_result_type(
    lhs: &Type,
    op: &crate::common::ast::expr::BinOp,
    rhs: &Type,
) -> Result<QualifierType, (String, String)> {
    use crate::common::ast::expr::BinOp;

    let make = |ty: Type| QualifierType {
        ty,
        is_const: false,
        is_unsigned: false,
    };

    let is_integer = |t: &Type| matches!(t, Type::Int | Type::Long | Type::Short | Type::Char);

    match op {
        // ── Adição e Subtração ───────────────────────────────────────────────
        BinOp::Add | BinOp::Sub => {
            // numérico OP numérico → promoção
            if is_numeric(lhs) && is_numeric(rhs) {
                return Ok(make(numeric_promotion(lhs, rhs)));
            }
            // ponteiro ± inteiro → ponteiro (aritmética de ponteiro)
            if is_pointer(lhs) && is_integer(rhs) {
                return Ok(make(lhs.clone()));
            }
            // inteiro + ponteiro → ponteiro (só para Add)
            if is_integer(lhs) && is_pointer(rhs) && matches!(op, BinOp::Add) {
                return Ok(make(rhs.clone()));
            }
            Err((
                "numeric or pointer".into(),
                format!("{} op {}", type_name(lhs), type_name(rhs)),
            ))
        }
        // ── Multiplicação e Divisão ──────────────────────────────────────────
        BinOp::Mul | BinOp::Div => {
            if is_numeric(lhs) && is_numeric(rhs) {
                return Ok(make(numeric_promotion(lhs, rhs)));
            }
            Err((
                "numeric".into(),
                format!("{} op {}", type_name(lhs), type_name(rhs)),
            ))
        }
        // ── Módulo ──────────────────────────────────────────────────────────
        BinOp::Mod => {
            if is_integer(lhs) && is_integer(rhs) {
                return Ok(make(numeric_promotion(lhs, rhs)));
            }
            Err((
                "integer".into(),
                format!("{} % {}", type_name(lhs), type_name(rhs)),
            ))
        }
        // ── Bitwise ─────────────────────────────────────────────────────────
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
            if is_integer(lhs) && is_integer(rhs) {
                return Ok(make(numeric_promotion(lhs, rhs)));
            }
            Err((
                "integer".into(),
                format!("{} bitop {}", type_name(lhs), type_name(rhs)),
            ))
        }
        // ── Relacionais ─────────────────────────────────────────────────────
        BinOp::Eq | BinOp::Neq | BinOp::Less | BinOp::Greater | BinOp::Leq | BinOp::Geq => {
            let both_numeric = is_numeric(lhs) && is_numeric(rhs);
            let both_pointer = is_pointer(lhs) && is_pointer(rhs);
            if both_numeric || both_pointer || lhs == rhs {
                return Ok(make(Type::Int)); // bool em C é representado como int
            }
            Err((
                "comparable types".into(),
                format!("{} vs {}", type_name(lhs), type_name(rhs)),
            ))
        }
        // ── Lógicos ─────────────────────────────────────────────────────────
        BinOp::And | BinOp::Or => {
            let lhs_scalar = is_numeric(lhs) || is_pointer(lhs);
            let rhs_scalar = is_numeric(rhs) || is_pointer(rhs);
            if lhs_scalar && rhs_scalar {
                return Ok(make(Type::Int));
            }
            Err((
                "scalar".into(),
                format!("{} logical {}", type_name(lhs), type_name(rhs)),
            ))
        }
    }
}
