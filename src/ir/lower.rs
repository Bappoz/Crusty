use crate::common::ast::{
    ast::{Program, Type},
    decl::Decl,
    expr::{BinOp, Expr, Literal, PostfixOp, PrefixOp, UnOp},
    stmt::{Stmt, SwitchLabel},
};
use crate::common::errors::types::CodegenError;
use crate::ir::tac::{
    ConstValue, LabelGen, LabelId, Operand, TacFunction, TacInstr, TacProgram, TempGen, TempId,
};

type LowerResult<T> = Result<T, CodegenError>;

#[derive(Debug, Clone)]
pub struct Lowerer {
    temps: TempGen,
    labels: LabelGen,
    instrs: Vec<TacInstr>,
    /// Tipo declarado de cada variavel/parametro visto ate agora na funcao
    /// atual. Usado apenas para resolver `sizeof(expr)`; nao substitui a
    /// analise semantica (que ja validou o programa antes do lowering).
    var_types: std::collections::HashMap<String, Type>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ControlLabels {
    break_label: Option<LabelId>,
    continue_label: Option<LabelId>,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            temps: TempGen::new(),
            labels: LabelGen::new(),
            instrs: Vec::new(),
            var_types: std::collections::HashMap::new(),
        }
    }

    /// Registra o tipo declarado de `name`, usado depois para resolver
    /// `sizeof(name)`. Chamado para parametros de funcao e para cada
    /// `VarDecl` conforme o lowering avanca.
    fn declare_var_type(&mut self, name: &str, ty: &Type) {
        self.var_types.insert(name.to_string(), ty.clone());
    }

    /// Infere o tipo estatico de um subconjunto limitado de expressoes
    /// (identificadores, deref, indice via ponteiro e cast) — o suficiente
    /// para resolver o tamanho do elemento em `arr[i]`. Nao substitui a
    /// analise semantica completa; expressoes fora desse subconjunto
    /// retornam um erro explicito em vez de um palpite.
    fn infer_type(&self, expr: &Expr) -> LowerResult<Type> {
        match expr {
            Expr::Ident(name, _) => self.var_types.get(name).cloned().ok_or_else(|| {
                codegen_error(
                    "tipo de variavel desconhecido no lowering",
                    Some("type"),
                )
            }),
            Expr::Unary(UnOp::Deref, inner, _) => match self.infer_type(inner)? {
                Type::Pointer(t) | Type::Array(t) => Ok(*t),
                _ => Err(codegen_error(
                    "deref de valor que nao e ponteiro/array",
                    Some("type"),
                )),
            },
            Expr::Index(arr, _, _) => match self.infer_type(arr)? {
                Type::Pointer(t) => Ok(*t),
                Type::Array(_) => Err(codegen_error(
                    "indexacao de array fixo ainda nao suportada (tamanho do array nao e rastreado pelo lowering); indexacao via ponteiro funciona normalmente",
                    Some("index"),
                )),
                _ => Err(codegen_error(
                    "indexacao de valor que nao e ponteiro/array",
                    Some("index"),
                )),
            },
            Expr::Cast(qty, _, _) => Ok(qty.ty.clone()),
            _ => Err(codegen_error(
                "tipo de expressao nao inferido no lowering (suporte limitado a identificador, deref, indice e cast)",
                Some("type"),
            )),
        }
    }

    /// Calcula o endereco (em bytes) de `arr[idx]`, assumindo que `arr` e um
    /// ponteiro: `endereco = lower_expr(arr) + idx * sizeof(elemento)`.
    fn lower_index_address(&mut self, arr: &Expr, idx: &Expr) -> LowerResult<Operand> {
        let elem_ty = self.infer_type(arr)?;
        let elem_size = type_size(&elem_ty)?;

        let base_ptr = self.lower_expr(arr)?;
        let idx_op = self.lower_expr(idx)?;

        let offset = if elem_size == 1 {
            idx_op
        } else {
            let scaled = self.fresh_temp();
            self.instrs.push(TacInstr::BinOp {
                dst: scaled,
                op: BinOp::Mul,
                lhs: idx_op,
                rhs: Operand::Const(ConstValue::Int(elem_size)),
            });
            Operand::Temp(scaled)
        };

        let addr = self.fresh_temp();
        self.instrs.push(TacInstr::BinOp {
            dst: addr,
            op: BinOp::Add,
            lhs: base_ptr,
            rhs: offset,
        });
        Ok(Operand::Temp(addr))
    }

    pub fn lower_expr(&mut self, expr: &Expr) -> LowerResult<Operand> {
        match expr {
            Expr::Literal(value, _) => Ok(Operand::Const(lower_literal(value))),
            Expr::Ident(name, _) => Ok(Operand::Var(name.clone())),
            Expr::Binary(lhs, op, rhs, _) => {
                let lhs = self.lower_expr(lhs)?;
                let rhs = self.lower_expr(rhs)?;
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::BinOp {
                    dst,
                    op: op.clone(),
                    lhs,
                    rhs,
                });
                Ok(Operand::Temp(dst))
            }
            Expr::Unary(op, src, _) => {
                let src = self.lower_expr(src)?;
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::UnOp {
                    dst,
                    op: op.clone(),
                    src,
                });
                Ok(Operand::Temp(dst))
            }
            Expr::Prefix(op, target, _) => self.lower_prefix(op, target),
            Expr::Postfix(op, target, _) => self.lower_postfix(op, target),
            Expr::Call(callee, args, _) => {
                let fn_name = match callee.as_ref() {
                    Expr::Ident(name, _) => name.clone(),
                    _ => {
                        return Err(codegen_error(
                            "chamada por expressao nao suportada no lowering",
                            Some("call"),
                        ));
                    }
                };
                let mut lowered_args = Vec::with_capacity(args.len());
                for arg in args {
                    lowered_args.push(self.lower_expr(arg)?);
                }
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::Call {
                    dst: Some(dst),
                    fn_name,
                    args: lowered_args,
                });
                Ok(Operand::Temp(dst))
            }
            Expr::Cast(_, inner, _) => self.lower_expr(inner),
            Expr::Assign(lhs, rhs, _) => {
                let src = self.lower_expr(rhs)?;
                let dst = self.lower_assignment_target(lhs)?;
                self.emit_copy(dst.clone(), src)?;
                Ok(dst)
            }
            Expr::CompoundAssign(op, lhs, rhs, _) => {
                let dst = self.lower_assignment_target(lhs)?;
                let rhs = self.lower_expr(rhs)?;
                let temp = self.fresh_temp();
                self.instrs.push(TacInstr::BinOp {
                    dst: temp,
                    op: op.clone(),
                    lhs: dst.clone(),
                    rhs,
                });
                self.emit_copy(dst.clone(), Operand::Temp(temp))?;
                Ok(dst)
            }
            Expr::SizeofType(qty, _) => Ok(Operand::Const(ConstValue::Int(type_size(&qty.ty)?))),
            Expr::Ternary(cond, then_expr, else_expr, _) => {
                let cond = self.lower_expr(cond)?;
                let then_label = self.labels.fresh();
                let else_label = self.labels.fresh();
                let end_label = self.labels.fresh();
                let dst = self.fresh_temp();

                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label,
                    else_label,
                });

                self.instrs.push(TacInstr::Label(then_label));
                let then_val = self.lower_expr(then_expr)?;
                self.emit_copy(Operand::Temp(dst), then_val)?;
                self.emit_jump_unless_terminated(end_label);

                self.instrs.push(TacInstr::Label(else_label));
                let else_val = self.lower_expr(else_expr)?;
                self.emit_copy(Operand::Temp(dst), else_val)?;

                self.instrs.push(TacInstr::Label(end_label));
                Ok(Operand::Temp(dst))
            }
            Expr::Index(arr, idx, _) => {
                let addr = self.lower_index_address(arr, idx)?;
                Ok(Operand::Deref(Box::new(addr)))
            }
            Expr::Member(_, _, _, _) => Err(codegen_error(
                "acesso a membro nao suportado no lowering",
                Some("member"),
            )),
            // `sizeof(expr)`: o caso pratico mais comum e `sizeof(variavel)`.
            // O tipo declarado de identificadores e rastreado em
            // `var_types` (preenchido a partir de parametros e `VarDecl`);
            // para qualquer outra forma de expressao ainda nao ha
            // informacao de tipo disponivel no lowering.
            Expr::Sizeof(inner, _) => match inner.as_ref() {
                Expr::Ident(name, _) => {
                    let ty = self.var_types.get(name).ok_or_else(|| {
                        codegen_error(
                            "sizeof(expr): tipo da variavel desconhecido no lowering",
                            Some("sizeof"),
                        )
                    })?;
                    Ok(Operand::Const(ConstValue::Int(type_size(ty)?)))
                }
                _ => Err(codegen_error(
                    "sizeof(expr) so e suportado para identificadores simples neste backend",
                    Some("sizeof"),
                )),
            },
        }
    }

    pub fn lower_stmt(&mut self, stmt: &Stmt) -> LowerResult<()> {
        self.lower_stmt_with_control(stmt, ControlLabels::default())
    }

    fn lower_stmt_with_control(&mut self, stmt: &Stmt, control: ControlLabels) -> LowerResult<()> {
        match stmt {
            Stmt::Block(stmts, _) => {
                for stmt in stmts {
                    self.lower_stmt_with_control(stmt, control)?;
                }
                Ok(())
            }
            Stmt::If(cond, then_branch, else_branch, _) => {
                let cond = self.lower_expr(cond)?;
                let then_label = self.labels.fresh();
                let else_label = self.labels.fresh();
                let end_label = self.labels.fresh();

                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label,
                    else_label,
                });

                self.instrs.push(TacInstr::Label(then_label));
                self.lower_stmt_with_control(then_branch, control)?;
                self.emit_jump_unless_terminated(end_label);

                self.instrs.push(TacInstr::Label(else_label));
                if let Some(else_branch) = else_branch {
                    self.lower_stmt_with_control(else_branch, control)?;
                }
                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
            Stmt::While(cond, body, _) => {
                let cond_label = self.labels.fresh();
                let body_label = self.labels.fresh();
                let end_label = self.labels.fresh();

                self.instrs.push(TacInstr::Label(cond_label));
                let cond = self.lower_expr(cond)?;
                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label: body_label,
                    else_label: end_label,
                });

                self.instrs.push(TacInstr::Label(body_label));
                self.lower_stmt_with_control(
                    body,
                    ControlLabels {
                        break_label: Some(end_label),
                        continue_label: Some(cond_label),
                    },
                )?;
                self.emit_jump_unless_terminated(cond_label);

                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
            Stmt::For(init, cond, inc, body, _) => {
                if let Some(init) = init {
                    self.lower_stmt_with_control(init, control)?;
                }

                let cond_label = self.labels.fresh();
                let body_label = self.labels.fresh();
                let inc_label = inc.as_ref().map(|_| self.labels.fresh());
                let end_label = self.labels.fresh();
                let continue_label = inc_label.unwrap_or(cond_label);

                self.instrs.push(TacInstr::Label(cond_label));
                if let Some(cond) = cond {
                    let cond = self.lower_expr(cond)?;
                    self.instrs.push(TacInstr::CondJump {
                        cond,
                        then_label: body_label,
                        else_label: end_label,
                    });
                }

                self.instrs.push(TacInstr::Label(body_label));
                self.lower_stmt_with_control(
                    body,
                    ControlLabels {
                        break_label: Some(end_label),
                        continue_label: Some(continue_label),
                    },
                )?;

                if let Some(inc_label) = inc_label {
                    self.instrs.push(TacInstr::Label(inc_label));
                    if let Some(inc) = inc {
                        self.lower_expr(inc)?;
                    }
                }
                self.emit_jump_unless_terminated(cond_label);

                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
            Stmt::DoWhile(cond, body, _) => {
                let body_label = self.labels.fresh();
                let cond_label = self.labels.fresh();
                let end_label = self.labels.fresh();

                self.instrs.push(TacInstr::Label(body_label));
                self.lower_stmt_with_control(
                    body,
                    ControlLabels {
                        break_label: Some(end_label),
                        continue_label: Some(cond_label),
                    },
                )?;

                self.instrs.push(TacInstr::Label(cond_label));
                let cond = self.lower_expr(cond)?;
                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label: body_label,
                    else_label: end_label,
                });

                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
            Stmt::Break(_) => {
                let label = control.break_label.ok_or_else(|| {
                    codegen_error("break fora de loop/switch nao suportado", Some("break"))
                })?;
                self.instrs.push(TacInstr::Jump { label });
                Ok(())
            }
            Stmt::Continue(_) => {
                let label = control.continue_label.ok_or_else(|| {
                    codegen_error("continue fora de loop nao suportado", Some("continue"))
                })?;
                self.instrs.push(TacInstr::Jump { label });
                Ok(())
            }
            Stmt::ExprStmt(expr, _) => {
                self.lower_expr(expr)?;
                Ok(())
            }
            Stmt::Return(expr, _) => {
                let val = expr
                    .as_ref()
                    .map(|expr| self.lower_expr(expr))
                    .transpose()?;
                self.instrs.push(TacInstr::Return { val });
                Ok(())
            }
            Stmt::VarDecl(qty, name, init, _) => {
                self.declare_var_type(name, &qty.ty);
                if let Some(init) = init {
                    let src = self.lower_expr(init)?;
                    self.emit_copy(Operand::Var(name.clone()), src)?;
                }
                Ok(())
            }
            Stmt::Switch(disc, cases, _) => {
                let disc_op = self.lower_expr(disc)?;
                let end_label = self.labels.fresh();

                // Um label por `case`/`default`; serve tanto de alvo da
                // comparacao quanto de entrada do corpo daquele caso.
                let case_labels: Vec<LabelId> = cases.iter().map(|_| self.labels.fresh()).collect();
                let default_index = cases
                    .iter()
                    .position(|case| matches!(case.label, SwitchLabel::Default));

                // Cadeia de comparacoes: testa cada `case` (na ordem em que
                // aparece); `default` nao entra na comparacao, e usado como
                // fallback ao final da cadeia.
                for (index, case) in cases.iter().enumerate() {
                    if let SwitchLabel::Case(case_expr) = &case.label {
                        let case_val = self.lower_expr(case_expr)?;
                        let cmp = self.fresh_temp();
                        self.instrs.push(TacInstr::BinOp {
                            dst: cmp,
                            op: BinOp::Eq,
                            lhs: disc_op.clone(),
                            rhs: case_val,
                        });
                        let next_test = self.labels.fresh();
                        self.instrs.push(TacInstr::CondJump {
                            cond: Operand::Temp(cmp),
                            then_label: case_labels[index],
                            else_label: next_test,
                        });
                        self.instrs.push(TacInstr::Label(next_test));
                    }
                }
                let fallback_label = default_index.map_or(end_label, |i| case_labels[i]);
                self.instrs.push(TacInstr::Jump {
                    label: fallback_label,
                });

                // Corpo dos casos, em ordem, sem break implicito entre eles
                // (fallthrough real de C); `break` salta para `end_label`.
                for (index, case) in cases.iter().enumerate() {
                    self.instrs.push(TacInstr::Label(case_labels[index]));
                    for stmt in &case.stmts {
                        self.lower_stmt_with_control(
                            stmt,
                            ControlLabels {
                                break_label: Some(end_label),
                                continue_label: control.continue_label,
                            },
                        )?;
                    }
                }

                self.instrs.push(TacInstr::Label(end_label));
                Ok(())
            }
        }
    }

    pub fn finish(self) -> Vec<TacInstr> {
        self.instrs
    }

    fn fresh_temp(&mut self) -> TempId {
        self.temps.fresh()
    }

    fn lower_prefix(&mut self, op: &PrefixOp, target: &Expr) -> LowerResult<Operand> {
        let dst = self.lower_assignment_target(target)?;
        let temp = self.fresh_temp();
        self.instrs.push(TacInstr::BinOp {
            dst: temp,
            op: prefix_bin_op(op),
            lhs: dst.clone(),
            rhs: Operand::Const(ConstValue::Int(1)),
        });
        self.emit_copy(dst.clone(), Operand::Temp(temp))?;
        Ok(dst)
    }

    fn lower_postfix(&mut self, op: &PostfixOp, target: &Expr) -> LowerResult<Operand> {
        let dst = self.lower_assignment_target(target)?;
        let old = self.fresh_temp();
        self.emit_copy(Operand::Temp(old), dst.clone())?;

        let new = self.fresh_temp();
        self.instrs.push(TacInstr::BinOp {
            dst: new,
            op: postfix_bin_op(op),
            lhs: dst.clone(),
            rhs: Operand::Const(ConstValue::Int(1)),
        });
        self.emit_copy(dst, Operand::Temp(new))?;
        Ok(Operand::Temp(old))
    }

    fn lower_assignment_target(&mut self, expr: &Expr) -> LowerResult<Operand> {
        match expr {
            Expr::Ident(name, _) => Ok(Operand::Var(name.clone())),
            // `*p` como destino (`*p = x;`, `*p += 1;`, `(*p)++` etc.): o
            // ponteiro em si e um rvalue comum, mas o destino da escrita e o
            // endereco para o qual ele aponta.
            Expr::Unary(UnOp::Deref, inner, _) => {
                let ptr = self.lower_expr(inner)?;
                Ok(Operand::Deref(Box::new(ptr)))
            }
            // `arr[i] = x;` (com `arr` ponteiro): mesmo enderecamento usado
            // na leitura, via `lower_index_address`.
            Expr::Index(arr, idx, _) => {
                let addr = self.lower_index_address(arr, idx)?;
                Ok(Operand::Deref(Box::new(addr)))
            }
            _ => Err(codegen_error(
                "destino de atribuicao nao suportado no lowering",
                Some("assign"),
            )),
        }
    }

    fn emit_copy(&mut self, dst: Operand, src: Operand) -> LowerResult<()> {
        match dst {
            Operand::Temp(_) | Operand::Var(_) | Operand::Deref(_) => {
                self.instrs.push(TacInstr::Copy { dst, src });
                Ok(())
            }
            Operand::Const(_) => Err(codegen_error(
                "constante nao pode ser destino de copia",
                Some("copy"),
            )),
        }
    }

    fn emit_jump_unless_terminated(&mut self, label: LabelId) {
        if !matches!(
            self.instrs.last(),
            Some(TacInstr::Jump { .. } | TacInstr::Return { .. })
        ) {
            self.instrs.push(TacInstr::Jump { label });
        }
    }
}

impl Default for Lowerer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn lower_function(decl: &Decl) -> LowerResult<TacFunction> {
    match decl {
        Decl::Function(_, name, params, body, _) => {
            let mut lowerer = Lowerer::new();
            for (qty, param_name) in params {
                lowerer.declare_var_type(param_name, &qty.ty);
            }
            for stmt in body {
                lowerer.lower_stmt(stmt)?;
            }

            Ok(TacFunction {
                name: name.clone(),
                params: params.iter().map(|(_, name)| name.clone()).collect(),
                instrs: lowerer.finish(),
            })
        }
        _ => Err(codegen_error(
            "lower_function espera Decl::Function",
            Some("lower_function"),
        )),
    }
}

pub fn lower_program(prog: &Program) -> LowerResult<TacProgram> {
    let mut functions = Vec::new();
    for decl in &prog.decls {
        if matches!(decl, Decl::Function(..)) {
            functions.push(lower_function(decl)?);
        }
    }

    Ok(TacProgram { functions })
}

/// Gera o TAC e aplica todas as otimizações básicas (constant folding,
/// constant propagation e dead code elimination) até ponto fixo.
///
/// Este é o ponto de entrada recomendado para a pipeline de compilação.
pub fn lower_and_optimize(prog: &Program) -> LowerResult<TacProgram> {
    use crate::codegen::inter::optimizations::optimize_function;

    let mut tac = lower_program(prog)?;
    for func in &mut tac.functions {
        optimize_function(&mut func.instrs);
    }
    Ok(tac)
}

fn lower_literal(value: &Literal) -> ConstValue {
    match value {
        Literal::Int(value) => ConstValue::Int(*value),
        Literal::Double(value) => ConstValue::Double(*value),
        Literal::Char(value) => ConstValue::Char(*value),
        Literal::String(value) => ConstValue::String(value.clone()),
    }
}

fn prefix_bin_op(op: &PrefixOp) -> BinOp {
    match op {
        PrefixOp::Inc => BinOp::Add,
        PrefixOp::Dec => BinOp::Sub,
    }
}

fn postfix_bin_op(op: &PostfixOp) -> BinOp {
    match op {
        PostfixOp::Inc => BinOp::Add,
        PostfixOp::Dec => BinOp::Sub,
    }
}

fn type_size(ty: &Type) -> LowerResult<i64> {
    match ty {
        Type::Char => Ok(1),
        Type::Short => Ok(2),
        Type::Int | Type::Float | Type::Enum(_) => Ok(4),
        Type::Long | Type::Double | Type::Pointer(_) => Ok(8),
        Type::Array(_) | Type::Void | Type::Struct(_) | Type::Alias(_) | Type::Function(_, _) => {
            Err(codegen_error(
                "lowering de sizeof(type) requer layout/tamanho completo",
                Some("sizeof"),
            ))
        }
    }
}

fn codegen_error(message: &str, instruction: Option<&str>) -> CodegenError {
    CodegenError {
        message: message.to_string(),
        instruction: instruction.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{
        ast::ast::{QualifierType, Type},
        errors::error_data::Span,
    };

    fn span() -> Span {
        Span {
            line: 1,
            end_line: 1,
            column_start: 1,
            column_end: 1,
        }
    }

    fn int_ty() -> QualifierType {
        QualifierType {
            ty: Type::Int,
            is_const: false,
            is_unsigned: false,
        }
    }

    fn int(value: i64) -> Expr {
        Expr::Literal(Literal::Int(value), span())
    }

    fn ident(name: &str) -> Expr {
        Expr::Ident(name.to_string(), span())
    }

    #[test]
    fn lower_binary_expr_produces_temp() {
        let expr = Expr::Binary(Box::new(int(2)), BinOp::Add, Box::new(int(3)), span());
        let mut lowerer = Lowerer::new();

        let result = lowerer.lower_expr(&expr).unwrap();

        assert_eq!(result, Operand::Temp(TempId(0)));
        assert_eq!(
            lowerer.finish(),
            vec![TacInstr::BinOp {
                dst: TempId(0),
                op: BinOp::Add,
                lhs: Operand::Const(ConstValue::Int(2)),
                rhs: Operand::Const(ConstValue::Int(3)),
            }]
        );
    }

    #[test]
    fn lower_if_else_produces_labels() {
        let stmt = Stmt::If(
            int(1),
            Box::new(Stmt::VarDecl(
                int_ty(),
                "x".to_string(),
                Some(int(2)),
                span(),
            )),
            Some(Box::new(Stmt::VarDecl(
                int_ty(),
                "y".to_string(),
                Some(int(3)),
                span(),
            ))),
            span(),
        );
        let mut lowerer = Lowerer::new();

        lowerer.lower_stmt(&stmt).unwrap();
        let instrs = lowerer.finish();

        assert!(matches!(
            instrs[0],
            TacInstr::CondJump {
                then_label: LabelId(0),
                else_label: LabelId(1),
                ..
            }
        ));
        assert_eq!(instrs[1], TacInstr::Label(LabelId(0)));
        assert_eq!(
            instrs[2],
            TacInstr::Copy {
                dst: Operand::Var("x".to_string()),
                src: Operand::Const(ConstValue::Int(2)),
            }
        );
        assert_eq!(instrs[3], TacInstr::Jump { label: LabelId(2) });
        assert_eq!(instrs[4], TacInstr::Label(LabelId(1)));
        assert_eq!(
            instrs[5],
            TacInstr::Copy {
                dst: Operand::Var("y".to_string()),
                src: Operand::Const(ConstValue::Int(3)),
            }
        );
        assert_eq!(instrs[6], TacInstr::Label(LabelId(2)));
    }

    #[test]
    fn lower_while_produces_backedge() {
        let stmt = Stmt::While(
            ident("keep_going"),
            Box::new(Stmt::VarDecl(
                int_ty(),
                "x".to_string(),
                Some(int(1)),
                span(),
            )),
            span(),
        );
        let mut lowerer = Lowerer::new();

        lowerer.lower_stmt(&stmt).unwrap();
        let instrs = lowerer.finish();

        assert_eq!(instrs[0], TacInstr::Label(LabelId(0)));
        assert!(matches!(
            instrs[1],
            TacInstr::CondJump {
                then_label: LabelId(1),
                else_label: LabelId(2),
                ..
            }
        ));
        assert_eq!(instrs[2], TacInstr::Label(LabelId(1)));
        assert_eq!(instrs[4], TacInstr::Jump { label: LabelId(0) });
        assert_eq!(instrs[5], TacInstr::Label(LabelId(2)));
    }

    #[test]
    fn lower_function_call_passes_args() {
        let arg0 = Expr::Binary(Box::new(int(1)), BinOp::Add, Box::new(int(2)), span());
        let expr = Expr::Call(Box::new(ident("sum")), vec![arg0, int(3)], span());
        let mut lowerer = Lowerer::new();

        let result = lowerer.lower_expr(&expr).unwrap();
        let instrs = lowerer.finish();

        assert_eq!(result, Operand::Temp(TempId(1)));
        assert_eq!(
            instrs[1],
            TacInstr::Call {
                dst: Some(TempId(1)),
                fn_name: "sum".to_string(),
                args: vec![Operand::Temp(TempId(0)), Operand::Const(ConstValue::Int(3))],
            }
        );
    }

    #[test]
    fn lower_nested_expr_correct_temp_order() {
        let rhs = Expr::Binary(Box::new(int(3)), BinOp::Mul, Box::new(int(4)), span());
        let expr = Expr::Binary(Box::new(int(2)), BinOp::Add, Box::new(rhs), span());
        let mut lowerer = Lowerer::new();

        let result = lowerer.lower_expr(&expr).unwrap();

        assert_eq!(result, Operand::Temp(TempId(1)));
        assert_eq!(
            lowerer.finish(),
            vec![
                TacInstr::BinOp {
                    dst: TempId(0),
                    op: BinOp::Mul,
                    lhs: Operand::Const(ConstValue::Int(3)),
                    rhs: Operand::Const(ConstValue::Int(4)),
                },
                TacInstr::BinOp {
                    dst: TempId(1),
                    op: BinOp::Add,
                    lhs: Operand::Const(ConstValue::Int(2)),
                    rhs: Operand::Temp(TempId(0)),
                },
            ]
        );
    }

    #[test]
    fn lower_function_keeps_name_params_and_body() {
        let decl = Decl::Function(
            int_ty(),
            "main".to_string(),
            vec![(int_ty(), "argc".to_string())],
            vec![Stmt::Return(Some(ident("argc")), span())],
            span(),
        );

        let func = lower_function(&decl).unwrap();

        assert_eq!(func.name, "main");
        assert_eq!(func.params, vec!["argc"]);
        assert_eq!(
            func.instrs,
            vec![TacInstr::Return {
                val: Some(Operand::Var("argc".to_string()))
            }]
        );
    }
}
