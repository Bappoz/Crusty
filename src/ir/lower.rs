use crate::common::ast::{
    expr::{Expr, Literal},
    stmt::Stmt,
};
use crate::ir::tac::{ConstValue, LabelGen, Operand, TacInstr, TempGen, TempId};

#[derive(Debug, Clone)]
pub struct Lowerer {
    temps: TempGen,
    labels: LabelGen,
    instrs: Vec<TacInstr>,
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

    pub fn lower_stmt(&mut self, _stmt: &Stmt) {
        panic!("lowering ainda nao suporta esse statement");
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
}

impl Default for Lowerer {
    fn default() -> Self {
        Self::new()
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
