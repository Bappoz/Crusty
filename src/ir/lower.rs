use crate::common::ast::{
    ast::Program,
    decl::Decl,
    expr::{Expr, Literal},
    stmt::Stmt,
};
use crate::ir::tac::{
    ConstValue, LabelGen, LabelId, Operand, TacFunction, TacInstr, TacProgram, TempGen, TempId,
};

#[derive(Debug, Clone)]
pub struct Lowerer {
    temps: TempGen,
    labels: LabelGen,
    instrs: Vec<TacInstr>,
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
        }
    }

    pub fn lower_expr(&mut self, expr: &Expr) -> Operand {
        match expr {
            Expr::Literal(value, _) => Operand::Const(lower_literal(value)),
            Expr::Ident(name, _) => Operand::Var(name.clone()),
            Expr::Binary(lhs, op, rhs, _) => {
                let lhs = self.lower_expr(lhs);
                let rhs = self.lower_expr(rhs);
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::BinOp {
                    dst,
                    op: op.clone(),
                    lhs,
                    rhs,
                });
                Operand::Temp(dst)
            }
            Expr::Unary(op, src, _) => {
                let src = self.lower_expr(src);
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::UnOp {
                    dst,
                    op: op.clone(),
                    src,
                });
                Operand::Temp(dst)
            }
            Expr::Call(callee, args, _) => {
                let fn_name = match callee.as_ref() {
                    Expr::Ident(name, _) => name.clone(),
                    _ => panic!("lowering ainda nao suporta chamada por expressao"),
                };
                let args = args.iter().map(|arg| self.lower_expr(arg)).collect();
                let dst = self.fresh_temp();
                self.instrs.push(TacInstr::Call {
                    dst: Some(dst),
                    fn_name,
                    args,
                });
                Operand::Temp(dst)
            }
            Expr::Cast(_, inner, _) => self.lower_expr(inner),
            Expr::Assign(lhs, rhs, _) => {
                let src = self.lower_expr(rhs);
                let dst = self.lower_assignment_target(lhs);
                self.emit_copy(dst.clone(), src);
                dst
            }
            _ => panic!("lowering ainda nao suporta essa expressao"),
        }
    }

    pub fn lower_stmt(&mut self, stmt: &Stmt) {
        self.lower_stmt_with_control(stmt, ControlLabels::default());
    }

    fn lower_stmt_with_control(&mut self, stmt: &Stmt, control: ControlLabels) {
        match stmt {
            Stmt::Block(stmts, _) => {
                for stmt in stmts {
                    self.lower_stmt_with_control(stmt, control);
                }
            }
            Stmt::If(cond, then_branch, else_branch, _) => {
                let cond = self.lower_expr(cond);
                let then_label = self.labels.fresh();
                let else_label = self.labels.fresh();
                let end_label = self.labels.fresh();

                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label,
                    else_label,
                });

                self.instrs.push(TacInstr::Label(then_label));
                self.lower_stmt_with_control(then_branch, control);
                self.emit_jump_unless_terminated(end_label);

                self.instrs.push(TacInstr::Label(else_label));
                if let Some(else_branch) = else_branch {
                    self.lower_stmt_with_control(else_branch, control);
                }
                self.instrs.push(TacInstr::Label(end_label));
            }
            Stmt::While(cond, body, _) => {
                let cond_label = self.labels.fresh();
                let body_label = self.labels.fresh();
                let end_label = self.labels.fresh();

                self.instrs.push(TacInstr::Label(cond_label));
                let cond = self.lower_expr(cond);
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
                );
                self.emit_jump_unless_terminated(cond_label);

                self.instrs.push(TacInstr::Label(end_label));
            }
            Stmt::For(init, cond, inc, body, _) => {
                if let Some(init) = init {
                    self.lower_stmt_with_control(init, control);
                }

                let cond_label = self.labels.fresh();
                let body_label = self.labels.fresh();
                let inc_label = inc.as_ref().map(|_| self.labels.fresh());
                let end_label = self.labels.fresh();
                let continue_label = inc_label.unwrap_or(cond_label);

                self.instrs.push(TacInstr::Label(cond_label));
                if let Some(cond) = cond {
                    let cond = self.lower_expr(cond);
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
                );

                if let Some(inc_label) = inc_label {
                    self.instrs.push(TacInstr::Label(inc_label));
                    if let Some(inc) = inc {
                        self.lower_expr(inc);
                    }
                }
                self.emit_jump_unless_terminated(cond_label);

                self.instrs.push(TacInstr::Label(end_label));
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
                );

                self.instrs.push(TacInstr::Label(cond_label));
                let cond = self.lower_expr(cond);
                self.instrs.push(TacInstr::CondJump {
                    cond,
                    then_label: body_label,
                    else_label: end_label,
                });

                self.instrs.push(TacInstr::Label(end_label));
            }
            Stmt::Break(_) => {
                let label = control
                    .break_label
                    .expect("break fora de loop/switch nao pode ser baixado");
                self.instrs.push(TacInstr::Jump { label });
            }
            Stmt::Continue(_) => {
                let label = control
                    .continue_label
                    .expect("continue fora de loop nao pode ser baixado");
                self.instrs.push(TacInstr::Jump { label });
            }
            Stmt::ExprStmt(expr, _) => {
                self.lower_expr(expr);
            }
            Stmt::Return(expr, _) => {
                let val = expr.as_ref().map(|expr| self.lower_expr(expr));
                self.instrs.push(TacInstr::Return { val });
            }
            Stmt::VarDecl(_, name, init, _) => {
                if let Some(init) = init {
                    let src = self.lower_expr(init);
                    self.emit_copy(Operand::Var(name.clone()), src);
                }
            }
            _ => panic!("lowering ainda nao suporta esse statement"),
        }
    }

    pub fn finish(self) -> Vec<TacInstr> {
        self.instrs
    }

    fn fresh_temp(&mut self) -> TempId {
        self.temps.fresh()
    }

    fn lower_assignment_target(&mut self, expr: &Expr) -> Operand {
        match expr {
            Expr::Ident(name, _) => Operand::Var(name.clone()),
            _ => panic!("lowering ainda nao suporta esse destino de atribuicao"),
        }
    }

    fn emit_copy(&mut self, dst: Operand, src: Operand) {
        match dst {
            Operand::Temp(_) | Operand::Var(_) => self.instrs.push(TacInstr::Copy { dst, src }),
            Operand::Const(_) => panic!("constante nao pode ser destino de copia"),
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

pub fn lower_function(decl: &Decl) -> TacFunction {
    match decl {
        Decl::Function(_, name, params, body, _) => {
            let mut lowerer = Lowerer::new();
            for stmt in body {
                lowerer.lower_stmt(stmt);
            }

            TacFunction {
                name: name.clone(),
                params: params.iter().map(|(_, name)| name.clone()).collect(),
                instrs: lowerer.finish(),
            }
        }
        _ => panic!("lower_function espera Decl::Function"),
    }
}

pub fn lower_program(prog: &Program) -> TacProgram {
    TacProgram {
        functions: prog
            .decls
            .iter()
            .filter(|decl| matches!(decl, Decl::Function(..)))
            .map(lower_function)
            .collect(),
    }
}

fn lower_literal(value: &Literal) -> ConstValue {
    match value {
        Literal::Int(value) => ConstValue::Int(*value),
        Literal::Double(value) => ConstValue::Double(*value),
        Literal::Char(value) => ConstValue::Char(*value),
        Literal::String(value) => ConstValue::String(value.clone()),
    }
}
