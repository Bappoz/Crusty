use std::collections::HashMap;

use crate::common::ast::ast::{Program, QualifierType, Type};
use crate::common::ast::decl::Decl;
use crate::common::ast::expr::{BinOp, Expr, Literal, MemberAccess, UnOp};
use crate::common::ast::stmt::{Stmt, SwitchCase, SwitchLabel};
use crate::common::errors::error_data::Span;
use crate::common::errors::types::{CompilerError, SemanticError, SemanticErrorKind};

#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
    structs: HashMap<String, StructInfo>,
}

#[derive(Debug, Clone)]
struct Symbol {
    kind: SymbolKind,
}

#[derive(Debug, Clone)]
enum SymbolKind {
    Variable(QualifierType),
    Function {
        return_type: QualifierType,
        params: Vec<QualifierType>,
    },
}

#[derive(Debug, Clone)]
struct StructInfo {
    fields: HashMap<String, QualifierType>,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            structs: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn insert_variable(&mut self, name: String, ty: QualifierType, _span: Span) -> bool {
        let scope = self.scopes.last_mut().expect("at least one scope exists");
        if scope.contains_key(&name) {
            return false;
        }

        scope.insert(
            name,
            Symbol {
                kind: SymbolKind::Variable(ty),
            },
        );
        true
    }

    pub fn insert_function(
        &mut self,
        name: String,
        return_type: QualifierType,
        params: Vec<QualifierType>,
        _span: Span,
    ) -> bool {
        let scope = self.scopes.first_mut().expect("global scope exists");
        if scope.contains_key(&name) {
            return false;
        }

        scope.insert(
            name,
            Symbol {
                kind: SymbolKind::Function {
                    return_type,
                    params,
                },
            },
        );
        true
    }

    pub fn insert_struct(
        &mut self,
        name: String,
        fields: HashMap<String, QualifierType>,
        _span: Span,
    ) -> bool {
        if self.structs.contains_key(&name) {
            return false;
        }

        self.structs.insert(name, StructInfo { fields });
        true
    }

    fn lookup(&self, name: &str) -> Option<Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol.clone());
            }
        }
        None
    }

    fn lookup_struct(&self, name: &str) -> Option<StructInfo> {
        self.structs.get(name).cloned()
    }

    pub fn has_function(&self, name: &str) -> bool {
        matches!(
            self.lookup(name),
            Some(Symbol {
                kind: SymbolKind::Function { .. },
                ..
            })
        )
    }

    pub fn has_struct(&self, name: &str) -> bool {
        self.structs.contains_key(name)
    }
}

#[derive(Debug)]
pub struct SemanticAnalyser {
    pub sym: SymbolTable,
    pub current_fn_ret: Option<QualifierType>,
    pub diagnostics: Vec<CompilerError>,
}

impl Default for SemanticAnalyser {
    fn default() -> Self {
        Self::new()
    }
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
        self.predeclare_program(prog);

        for decl in &prog.decls {
            self.analyse_decl(decl);
        }
    }

    pub fn analyse_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Function(return_type, name, params, body, span) => {
                let param_types = params.iter().map(|(ty, _)| ty.clone()).collect();
                if !self.sym.has_function(name) {
                    self.sym.insert_function(
                        name.clone(),
                        return_type.clone(),
                        param_types,
                        span.clone(),
                    );
                }

                let previous_return_type = self.current_fn_ret.clone();
                self.current_fn_ret = Some(return_type.clone());

                self.sym.enter_scope();
                for (param_type, param_name) in params {
                    self.validate_type(param_type, span.clone());
                    if !self.sym.insert_variable(
                        param_name.clone(),
                        param_type.clone(),
                        span.clone(),
                    ) {
                        self.push_type_mismatch(span.clone(), "parametro unico", param_name);
                    }
                }

                for stmt in body {
                    self.analyse_stmt(stmt);
                }
                self.sym.exit_scope();

                self.current_fn_ret = previous_return_type;
            }
            Decl::GlobalVar(qty, name, init, span) => {
                self.validate_type(qty, span.clone());
                if let Some(expr) = init {
                    let found = self.analyse_expr(expr);
                    self.ensure_assignable(qty, &found, expr.span());
                }

                if !self
                    .sym
                    .insert_variable(name.clone(), qty.clone(), span.clone())
                {
                    self.push_type_mismatch(span.clone(), "declaracao unica", name);
                }
            }
            Decl::StructDecl(name, fields, span) => {
                let mut field_map = HashMap::new();
                for (field_type, field_name) in fields {
                    self.validate_type(field_type, span.clone());
                    field_map.insert(field_name.clone(), field_type.clone());
                }

                if !self
                    .sym
                    .insert_struct(name.clone(), field_map, span.clone())
                {
                    self.push_type_mismatch(span.clone(), "struct unica", name);
                }
            }
        }
    }

    pub fn analyse_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(stmts, _) => {
                self.sym.enter_scope();
                for inner in stmts {
                    self.analyse_stmt(inner);
                }
                self.sym.exit_scope();
            }
            Stmt::If(cond, then_branch, else_branch, _) => {
                let cond_ty = self.analyse_expr(cond);
                self.ensure_condition(&cond_ty, cond.span());
                self.analyse_stmt(then_branch);
                if let Some(else_branch) = else_branch {
                    self.analyse_stmt(else_branch);
                }
            }
            Stmt::While(cond, body, _) => {
                let cond_ty = self.analyse_expr(cond);
                self.ensure_condition(&cond_ty, cond.span());
                self.analyse_stmt(body);
            }
            Stmt::For(init, cond, inc, body, _) => {
                self.sym.enter_scope();
                if let Some(init) = init {
                    self.analyse_stmt(init);
                }
                if let Some(cond) = cond {
                    let cond_ty = self.analyse_expr(cond);
                    self.ensure_condition(&cond_ty, cond.span());
                }
                if let Some(inc) = inc {
                    self.analyse_expr(inc);
                }
                self.analyse_stmt(body);
                self.sym.exit_scope();
            }
            Stmt::DoWhile(cond, body, _) => {
                self.analyse_stmt(body);
                let cond_ty = self.analyse_expr(cond);
                self.ensure_condition(&cond_ty, cond.span());
            }
            Stmt::Switch(expr, cases, _) => {
                let switch_ty = self.analyse_expr(expr);
                self.sym.enter_scope();
                for case in cases {
                    self.analyse_switch_case(&switch_ty, case);
                }
                self.sym.exit_scope();
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::ExprStmt(expr, _) => {
                self.analyse_expr(expr);
            }
            Stmt::Return(value, span) => {
                let expected = self.current_fn_ret.clone();
                match (expected, value) {
                    (Some(expected), Some(expr)) => {
                        let found = self.analyse_expr(expr);
                        self.ensure_assignable(&expected, &found, expr.span());
                    }
                    (Some(expected), None) => {
                        if !matches!(expected.ty, Type::Void) {
                            self.push_type_mismatch(
                                span.clone(),
                                &self.type_name(&expected),
                                "void",
                            );
                        }
                    }
                    (None, Some(expr)) => {
                        self.analyse_expr(expr);
                    }
                    (None, None) => {}
                }
            }
            Stmt::VarDecl(qty, name, init, span) => {
                self.validate_type(qty, span.clone());
                if let Some(expr) = init {
                    let found = self.analyse_expr(expr);
                    self.ensure_assignable(qty, &found, expr.span());
                }

                if !self
                    .sym
                    .insert_variable(name.clone(), qty.clone(), span.clone())
                {
                    self.push_type_mismatch(span.clone(), "declaracao unica", name);
                }
            }
        }
    }

    pub fn analyse_expr(&mut self, expr: &Expr) -> QualifierType {
        match expr {
            Expr::Literal(lit, _) => self.literal_type(lit),
            Expr::Ident(name, span) => match self.sym.lookup(name) {
                Some(symbol) => match symbol.kind {
                    SymbolKind::Variable(ty) => ty,
                    SymbolKind::Function { return_type, .. } => return_type,
                },
                None => {
                    self.push_semantic_error(
                        span.clone(),
                        SemanticErrorKind::UndefinedVariable(name.clone()),
                    );
                    self.int_type()
                }
            },
            Expr::Binary(lhs, op, rhs, span) => {
                let lhs_ty = self.analyse_expr(lhs);
                let rhs_ty = self.analyse_expr(rhs);
                self.analyse_binary_op(op, &lhs_ty, &rhs_ty, span.clone())
            }
            Expr::Unary(op, inner, span) => {
                let inner_ty = self.analyse_expr(inner);
                self.analyse_unary_op(op, &inner_ty, span.clone())
            }
            Expr::Prefix(_, inner, span) => {
                let inner_ty = self.analyse_expr(inner);
                if !self.is_numeric_or_pointer(&inner_ty.ty) {
                    self.push_type_mismatch(
                        span.clone(),
                        "valor numerico ou ponteiro",
                        &self.type_name(&inner_ty),
                    );
                }
                inner_ty
            }
            Expr::Postfix(_, inner, span) => {
                let inner_ty = self.analyse_expr(inner);
                if !self.is_numeric_or_pointer(&inner_ty.ty) {
                    self.push_type_mismatch(
                        span.clone(),
                        "valor numerico ou ponteiro",
                        &self.type_name(&inner_ty),
                    );
                }
                inner_ty
            }
            Expr::Call(callee, args, span) => self.analyse_call(callee, args, span.clone()),
            Expr::Cast(target, inner, _) => {
                self.validate_type(target, expr.span());
                self.analyse_expr(inner);
                target.clone()
            }
            Expr::Index(array, index, span) => {
                let array_ty = self.analyse_expr(array);
                let index_ty = self.analyse_expr(index);
                if !self.is_numeric(&index_ty.ty) {
                    self.push_type_mismatch(
                        index.span(),
                        "indice numerico",
                        &self.type_name(&index_ty),
                    );
                }

                match &array_ty.ty {
                    Type::Pointer(inner) | Type::Array(inner) => self.qualifier((**inner).clone()),
                    _ => {
                        self.push_type_mismatch(
                            span.clone(),
                            "ponteiro ou array",
                            &self.type_name(&array_ty),
                        );
                        self.int_type()
                    }
                }
            }
            Expr::Assign(lhs, rhs, span) => {
                let lhs_ty = self.analyse_expr(lhs);
                let rhs_ty = self.analyse_expr(rhs);
                self.ensure_assignable(&lhs_ty, &rhs_ty, span.clone());
                lhs_ty
            }
            Expr::Sizeof(inner, _) => {
                self.analyse_expr(inner);
                self.int_type()
            }
            Expr::Ternary(cond, then_expr, else_expr, span) => {
                let cond_ty = self.analyse_expr(cond);
                self.ensure_condition(&cond_ty, cond.span());

                let then_ty = self.analyse_expr(then_expr);
                let else_ty = self.analyse_expr(else_expr);
                if self.are_assignable(&then_ty, &else_ty) {
                    self.merge_types(&then_ty, &else_ty)
                } else {
                    self.push_type_mismatch(
                        span.clone(),
                        &self.type_name(&then_ty),
                        &self.type_name(&else_ty),
                    );
                    then_ty
                }
            }
            Expr::Member(base, access, field, span) => {
                let base_ty = self.analyse_expr(base);
                self.analyse_member_access(access, &base_ty, field, span.clone())
            }
        }
    }

    fn predeclare_program(&mut self, prog: &Program) {
        for decl in &prog.decls {
            match decl {
                Decl::Function(return_type, name, params, _, span) => {
                    let param_types = params.iter().map(|(ty, _)| ty.clone()).collect();
                    if !self.sym.has_function(name) {
                        self.sym.insert_function(
                            name.clone(),
                            return_type.clone(),
                            param_types,
                            span.clone(),
                        );
                    }
                }
                Decl::StructDecl(name, fields, span) => {
                    if !self.sym.has_struct(name) {
                        let mut field_map = HashMap::new();
                        for (field_type, field_name) in fields {
                            field_map.insert(field_name.clone(), field_type.clone());
                        }
                        self.sym
                            .insert_struct(name.clone(), field_map, span.clone());
                    }
                }
                Decl::GlobalVar(_, _, _, _) => {}
            }
        }
    }

    fn analyse_switch_case(&mut self, switch_ty: &QualifierType, case: &SwitchCase) {
        match &case.label {
            SwitchLabel::Case(expr) => {
                let case_ty = self.analyse_expr(expr);
                if !self.are_assignable(switch_ty, &case_ty) {
                    self.push_type_mismatch(
                        expr.span(),
                        &self.type_name(switch_ty),
                        &self.type_name(&case_ty),
                    );
                }
            }
            SwitchLabel::Default => {}
        }

        for stmt in &case.stmts {
            self.analyse_stmt(stmt);
        }
    }

    fn analyse_call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> QualifierType {
        if let Expr::Ident(name, ident_span) = callee {
            if let Some(symbol) = self.sym.lookup(name) {
                match symbol.kind {
                    SymbolKind::Function {
                        return_type,
                        params,
                    } => {
                        for (arg, expected) in args.iter().zip(params.iter()) {
                            let found = self.analyse_expr(arg);
                            self.ensure_assignable(expected, &found, arg.span());
                        }

                        if args.len() != params.len() {
                            self.push_type_mismatch(
                                span,
                                &format!("{} argumento(s)", params.len()),
                                &format!("{} argumento(s)", args.len()),
                            );
                        }

                        return return_type;
                    }
                    SymbolKind::Variable(found) => {
                        self.push_type_mismatch(
                            ident_span.clone(),
                            "funcao",
                            &self.type_name(&found),
                        );
                    }
                }
            }
        } else {
            self.analyse_expr(callee);
        }

        for arg in args {
            self.analyse_expr(arg);
        }

        self.int_type()
    }

    fn analyse_member_access(
        &mut self,
        access: &MemberAccess,
        base_ty: &QualifierType,
        field: &str,
        span: Span,
    ) -> QualifierType {
        let struct_name = match (&access, &base_ty.ty) {
            (MemberAccess::Direct, Type::Struct(name)) => Some(name.clone()),
            (MemberAccess::Pointer, Type::Pointer(inner)) => match inner.as_ref() {
                Type::Struct(name) => Some(name.clone()),
                _ => None,
            },
            (MemberAccess::Direct, Type::Pointer(_)) => None,
            (MemberAccess::Pointer, Type::Struct(_)) => None,
            _ => None,
        };

        let Some(struct_name) = struct_name else {
            let expected = match access {
                MemberAccess::Direct => "struct".to_string(),
                MemberAccess::Pointer => "ponteiro para struct".to_string(),
            };
            self.push_type_mismatch(span, &expected, &self.type_name(base_ty));
            return self.int_type();
        };

        let Some(info) = self.sym.lookup_struct(&struct_name) else {
            self.push_semantic_error(span, SemanticErrorKind::UndefinedVariable(struct_name));
            return self.int_type();
        };

        match info.fields.get(field) {
            Some(ty) => ty.clone(),
            None => {
                self.push_semantic_error(
                    span,
                    SemanticErrorKind::UndefinedVariable(field.to_string()),
                );
                self.int_type()
            }
        }
    }

    fn analyse_binary_op(
        &mut self,
        op: &BinOp,
        lhs_ty: &QualifierType,
        rhs_ty: &QualifierType,
        span: Span,
    ) -> QualifierType {
        match op {
            BinOp::Add => self.analyse_add(lhs_ty, rhs_ty, span),
            BinOp::Sub => self.analyse_sub(lhs_ty, rhs_ty, span),
            BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Shl
            | BinOp::Shr
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor => {
                if !self.is_numeric(&lhs_ty.ty) || !self.is_numeric(&rhs_ty.ty) {
                    self.push_type_mismatch(
                        span,
                        "tipos numericos",
                        &self.binary_type_name(lhs_ty, rhs_ty),
                    );
                }
                self.merge_numeric(lhs_ty, rhs_ty)
            }
            BinOp::Eq | BinOp::Neq => {
                if !self.are_assignable(lhs_ty, rhs_ty) && !self.are_assignable(rhs_ty, lhs_ty) {
                    self.push_type_mismatch(span, &self.type_name(lhs_ty), &self.type_name(rhs_ty));
                }
                self.int_type()
            }
            BinOp::Less | BinOp::Greater | BinOp::Leq | BinOp::Geq => {
                if !self.is_numeric(&lhs_ty.ty) || !self.is_numeric(&rhs_ty.ty) {
                    self.push_type_mismatch(
                        span,
                        "tipos numericos",
                        &self.binary_type_name(lhs_ty, rhs_ty),
                    );
                }
                self.int_type()
            }
            BinOp::And | BinOp::Or => {
                self.ensure_condition(lhs_ty, span.clone());
                self.ensure_condition(rhs_ty, span);
                self.int_type()
            }
        }
    }

    fn analyse_add(
        &mut self,
        lhs_ty: &QualifierType,
        rhs_ty: &QualifierType,
        span: Span,
    ) -> QualifierType {
        if self.is_numeric(&lhs_ty.ty) && self.is_numeric(&rhs_ty.ty) {
            return self.merge_numeric(lhs_ty, rhs_ty);
        }

        if self.is_pointer(&lhs_ty.ty) && self.is_numeric(&rhs_ty.ty) {
            return lhs_ty.clone();
        }

        if self.is_pointer(&rhs_ty.ty) && self.is_numeric(&lhs_ty.ty) {
            return rhs_ty.clone();
        }

        self.push_type_mismatch(
            span,
            "tipos numericos ou ponteiro + inteiro",
            &self.binary_type_name(lhs_ty, rhs_ty),
        );
        self.merge_numeric(lhs_ty, rhs_ty)
    }

    fn analyse_sub(
        &mut self,
        lhs_ty: &QualifierType,
        rhs_ty: &QualifierType,
        span: Span,
    ) -> QualifierType {
        if self.is_numeric(&lhs_ty.ty) && self.is_numeric(&rhs_ty.ty) {
            return self.merge_numeric(lhs_ty, rhs_ty);
        }

        if self.is_pointer(&lhs_ty.ty) && self.is_numeric(&rhs_ty.ty) {
            return lhs_ty.clone();
        }

        if self.is_pointer(&lhs_ty.ty)
            && self.is_pointer(&rhs_ty.ty)
            && self.same_pointee(&lhs_ty.ty, &rhs_ty.ty)
        {
            return self.int_type();
        }

        self.push_type_mismatch(
            span,
            "tipos numericos ou ponteiros compatíveis",
            &self.binary_type_name(lhs_ty, rhs_ty),
        );
        self.merge_numeric(lhs_ty, rhs_ty)
    }

    fn analyse_unary_op(
        &mut self,
        op: &UnOp,
        inner_ty: &QualifierType,
        span: Span,
    ) -> QualifierType {
        match op {
            UnOp::Neg | UnOp::BitNot => {
                if !self.is_numeric(&inner_ty.ty) {
                    self.push_type_mismatch(span, "tipo numerico", &self.type_name(inner_ty));
                }
                inner_ty.clone()
            }
            UnOp::Not => {
                self.ensure_condition(inner_ty, span);
                self.int_type()
            }
            UnOp::Deref => match &inner_ty.ty {
                Type::Pointer(inner) | Type::Array(inner) => self.qualifier((**inner).clone()),
                _ => {
                    self.push_type_mismatch(span, "ponteiro ou array", &self.type_name(inner_ty));
                    self.int_type()
                }
            },
            UnOp::AddrOf => self.pointer_to(inner_ty.clone()),
        }
    }

    fn ensure_condition(&mut self, ty: &QualifierType, span: Span) {
        if !self.is_scalar(&ty.ty) {
            self.push_type_mismatch(span, "tipo escalar", &self.type_name(ty));
        }
    }

    fn ensure_assignable(&mut self, expected: &QualifierType, found: &QualifierType, span: Span) {
        if !self.are_assignable(expected, found) {
            self.push_type_mismatch(span, &self.type_name(expected), &self.type_name(found));
        }
    }

    fn are_assignable(&self, expected: &QualifierType, found: &QualifierType) -> bool {
        if self.same_qualifier_type(expected, found) {
            return true;
        }

        if self.is_numeric(&expected.ty) && self.is_numeric(&found.ty) {
            return true;
        }

        match (&expected.ty, &found.ty) {
            (Type::Pointer(expected_inner), Type::Pointer(found_inner)) => {
                self.same_type(expected_inner, found_inner)
            }
            (Type::Array(expected_inner), Type::Array(found_inner)) => {
                self.same_type(expected_inner, found_inner)
            }
            (Type::Struct(expected_name), Type::Struct(found_name)) => expected_name == found_name,
            _ => false,
        }
    }

    fn merge_types(&self, lhs: &QualifierType, rhs: &QualifierType) -> QualifierType {
        if self.same_qualifier_type(lhs, rhs) {
            return lhs.clone();
        }
        self.merge_numeric(lhs, rhs)
    }

    fn merge_numeric(&self, lhs: &QualifierType, rhs: &QualifierType) -> QualifierType {
        if matches!(lhs.ty, Type::Double) || matches!(rhs.ty, Type::Double) {
            return self.qualifier(Type::Double);
        }
        if matches!(lhs.ty, Type::Float) || matches!(rhs.ty, Type::Float) {
            return self.qualifier(Type::Float);
        }
        self.qualifier(Type::Int)
    }

    fn validate_type(&mut self, qty: &QualifierType, span: Span) {
        self.validate_qualified_type(&qty.ty, span);
    }

    fn validate_qualified_type(&mut self, ty: &Type, span: Span) {
        match ty {
            Type::Pointer(inner) | Type::Array(inner) => self.validate_qualified_type(inner, span),
            Type::Struct(name) => {
                if !self.sym.has_struct(name) {
                    self.push_semantic_error(
                        span,
                        SemanticErrorKind::UndefinedVariable(name.clone()),
                    );
                }
            }
            Type::Int | Type::Char | Type::Float | Type::Double | Type::Void => {}
        }
    }

    fn literal_type(&self, lit: &Literal) -> QualifierType {
        match lit {
            Literal::Int(_) => self.qualifier(Type::Int),
            Literal::Double(_) => self.qualifier(Type::Double),
            Literal::Char(_) => self.qualifier(Type::Char),
            Literal::String(_) => QualifierType {
                ty: Type::Pointer(Box::new(Type::Char)),
                is_const: true,
                is_unsigned: false,
            },
        }
    }

    fn qualifier(&self, ty: Type) -> QualifierType {
        QualifierType {
            ty,
            is_const: false,
            is_unsigned: false,
        }
    }

    fn pointer_to(&self, inner: QualifierType) -> QualifierType {
        QualifierType {
            ty: Type::Pointer(Box::new(inner.ty)),
            is_const: false,
            is_unsigned: false,
        }
    }

    fn int_type(&self) -> QualifierType {
        self.qualifier(Type::Int)
    }

    fn push_semantic_error(&mut self, span: Span, kind: SemanticErrorKind) {
        self.diagnostics
            .push(CompilerError::Semantic(SemanticError { span, kind }));
    }

    fn push_type_mismatch(&mut self, span: Span, expected: &str, found: &str) {
        self.push_semantic_error(
            span,
            SemanticErrorKind::TypeMismatch {
                expected: expected.to_string(),
                found: found.to_string(),
            },
        );
    }

    fn same_qualifier_type(&self, lhs: &QualifierType, rhs: &QualifierType) -> bool {
        self.same_type(&lhs.ty, &rhs.ty)
    }

    fn same_type(&self, lhs: &Type, rhs: &Type) -> bool {
        match (lhs, rhs) {
            (Type::Int, Type::Int)
            | (Type::Char, Type::Char)
            | (Type::Float, Type::Float)
            | (Type::Double, Type::Double)
            | (Type::Void, Type::Void) => true,
            (Type::Struct(lhs_name), Type::Struct(rhs_name)) => lhs_name == rhs_name,
            (Type::Pointer(lhs_inner), Type::Pointer(rhs_inner))
            | (Type::Array(lhs_inner), Type::Array(rhs_inner)) => {
                self.same_type(lhs_inner, rhs_inner)
            }
            _ => false,
        }
    }

    fn same_pointee(&self, lhs: &Type, rhs: &Type) -> bool {
        match (lhs, rhs) {
            (Type::Pointer(lhs_inner), Type::Pointer(rhs_inner)) => {
                self.same_type(lhs_inner, rhs_inner)
            }
            _ => false,
        }
    }

    fn is_scalar(&self, ty: &Type) -> bool {
        self.is_numeric(ty) || self.is_pointer(ty) || matches!(ty, Type::Struct(_))
    }

    fn is_numeric_or_pointer(&self, ty: &Type) -> bool {
        self.is_numeric(ty) || self.is_pointer(ty)
    }

    fn is_numeric(&self, ty: &Type) -> bool {
        matches!(ty, Type::Int | Type::Char | Type::Float | Type::Double)
    }

    fn is_pointer(&self, ty: &Type) -> bool {
        matches!(ty, Type::Pointer(_))
    }

    fn binary_type_name(&self, lhs: &QualifierType, rhs: &QualifierType) -> String {
        format!("{} and {}", self.type_name(lhs), self.type_name(rhs))
    }

    fn type_name(&self, qty: &QualifierType) -> String {
        let mut prefix = String::new();
        if qty.is_const {
            prefix.push_str("const ");
        }
        if qty.is_unsigned {
            prefix.push_str("unsigned ");
        }

        let suffix = match &qty.ty {
            Type::Int => "int".to_string(),
            Type::Char => "char".to_string(),
            Type::Float => "float".to_string(),
            Type::Double => "double".to_string(),
            Type::Void => "void".to_string(),
            Type::Struct(name) => format!("struct {}", name),
            Type::Pointer(inner) => format!(
                "{}*",
                self.type_name(&QualifierType {
                    ty: (**inner).clone(),
                    is_const: false,
                    is_unsigned: false,
                })
            ),
            Type::Array(inner) => format!(
                "{}[]",
                self.type_name(&QualifierType {
                    ty: (**inner).clone(),
                    is_const: false,
                    is_unsigned: false,
                })
            ),
        };

        format!("{}{}", prefix, suffix)
    }
}

pub fn analyse(prog: &Program) -> Vec<CompilerError> {
    let mut analyser = SemanticAnalyser::new();
    analyser.analyse_program(prog);
    analyser.diagnostics
}
