use super::OptPass;
use crate::codegen::inter::{BinaryOp, Cfg, Instruction, Value};

pub struct ConstantFoldPass;

impl OptPass for ConstantFoldPass {
    fn name(&self) -> &'static str {
        "constant-fold"
    }

    fn run(&self, cfg: &mut Cfg) -> bool {
        let mut changed = false;

        for block in &mut cfg.blocks {
            for instruction in &mut block.instructions {
                if let Instruction::Binary { dst, op, lhs, rhs } = instruction {
                    let folded = fold_binary(*op, lhs, rhs);
                    if let Some(value) = folded {
                        *instruction = Instruction::Assign {
                            dst: dst.clone(),
                            value,
                        };
                        changed = true;
                    }
                }
            }
        }

        changed
    }
}

fn fold_binary(op: BinaryOp, lhs: &Value, rhs: &Value) -> Option<Value> {
    let (Value::Int(lhs), Value::Int(rhs)) = (lhs, rhs) else {
        return None;
    };

    let value = match op {
        BinaryOp::Add => lhs.checked_add(*rhs)?,
        BinaryOp::Sub => lhs.checked_sub(*rhs)?,
        BinaryOp::Mul => lhs.checked_mul(*rhs)?,
        BinaryOp::Div => {
            if *rhs == 0 {
                return None;
            }
            lhs.checked_div(*rhs)?
        }
        BinaryOp::Mod => {
            if *rhs == 0 {
                return None;
            }
            lhs.checked_rem(*rhs)?
        }
    };

    Some(Value::Int(value))
}
