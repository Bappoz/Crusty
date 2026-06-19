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
