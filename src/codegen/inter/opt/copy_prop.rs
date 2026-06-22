use std::collections::HashMap;

use super::OptPass;
use crate::codegen::inter::{Cfg, Instruction, Value};

/// Copy propagation e constant propagation, combinadas, sobre o TAC.
///
/// Para cada bloco basico, mantem um mapa `temp -> valor` com a ultima
/// atribuicao simples (`dst = src` ou `dst = literal`) conhecida para cada
/// destino. Usos subsequentes desse destino sao substituidos diretamente
/// pelo valor mapeado, ate que o destino seja redefinido.
pub struct CopyPropagationPass;

impl OptPass for CopyPropagationPass {
    fn name(&self) -> &'static str {
        "copy-propagation"
    }

    fn run(&self, cfg: &mut Cfg) -> bool {
        let mut changed = false;

        for block in &mut cfg.blocks {
            changed |= propagate_in_block(&mut block.instructions);
        }

        changed
    }
}

type CopyMap = HashMap<String, Value>;

fn propagate_in_block(instructions: &mut [Instruction]) -> bool {
    let mut changed = false;
    let mut copies: CopyMap = HashMap::new();

    for instruction in instructions.iter_mut() {
        match instruction {
            Instruction::Assign { dst, value } => {
                let resolved = resolve(&copies, value);
                if resolved != *value {
                    *value = resolved.clone();
                    changed = true;
                }

                invalidate(&mut copies, dst);
                if !matches!(&resolved, Value::Temp(name) if name == dst) {
                    copies.insert(dst.clone(), resolved);
                }
            }
            Instruction::Binary { dst, lhs, rhs, .. } => {
                let resolved_lhs = resolve(&copies, lhs);
                if resolved_lhs != *lhs {
                    *lhs = resolved_lhs;
                    changed = true;
                }

                let resolved_rhs = resolve(&copies, rhs);
                if resolved_rhs != *rhs {
                    *rhs = resolved_rhs;
                    changed = true;
                }

                invalidate(&mut copies, dst);
            }
            Instruction::Nop => {}
        }
    }

    changed
}

fn resolve(copies: &CopyMap, value: &Value) -> Value {
    match value {
        Value::Temp(name) => copies.get(name).cloned().unwrap_or_else(|| value.clone()),
        other => other.clone(),
    }
}

/// Remove do mapa qualquer entrada que dependa de `name`, seja como chave
/// (o proprio destino esta sendo redefinido) ou como valor (algum outro
/// destino havia sido marcado como copia de `name`).
fn invalidate(copies: &mut CopyMap, name: &str) {
    copies.remove(name);
    copies.retain(|_, v| !matches!(v, Value::Temp(n) if n == name));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::inter::{BasicBlock, BinaryOp};

    fn assign(dst: &str, value: Value) -> Instruction {
        Instruction::Assign {
            dst: dst.to_string(),
            value,
        }
    }

    fn binary(dst: &str, op: BinaryOp, lhs: Value, rhs: Value) -> Instruction {
        Instruction::Binary {
            dst: dst.to_string(),
            op,
            lhs,
            rhs,
        }
    }

    #[test]
    fn propagates_simple_copy() {
        let mut block = BasicBlock::new("entry");
        block.instructions.push(binary(
            "a",
            BinaryOp::Add,
            Value::Temp("p".into()),
            Value::Temp("q".into()),
        ));
        block
            .instructions
            .push(assign("b", Value::Temp("a".into())));
        block.instructions.push(binary(
            "c",
            BinaryOp::Add,
            Value::Temp("b".into()),
            Value::Int(1),
        ));

        let changed = propagate_in_block(&mut block.instructions);

        assert!(changed);
        assert_eq!(
            block.instructions[2],
            binary("c", BinaryOp::Add, Value::Temp("a".into()), Value::Int(1))
        );
    }

    #[test]
    fn propagates_constant() {
        let mut block = BasicBlock::new("entry");
        block.instructions.push(assign("x", Value::Int(10)));
        block.instructions.push(binary(
            "y",
            BinaryOp::Mul,
            Value::Temp("x".into()),
            Value::Int(2),
        ));

        let changed = propagate_in_block(&mut block.instructions);

        assert!(changed);
        assert_eq!(
            block.instructions[1],
            binary("y", BinaryOp::Mul, Value::Int(10), Value::Int(2))
        );
    }

    #[test]
    fn stops_propagating_after_redefinition() {
        let mut block = BasicBlock::new("entry");
        block.instructions.push(assign("x", Value::Int(10)));
        block
            .instructions
            .push(assign("x", Value::Temp("p".into())));
        block.instructions.push(binary(
            "y",
            BinaryOp::Mul,
            Value::Temp("x".into()),
            Value::Int(2),
        ));

        propagate_in_block(&mut block.instructions);

        assert_eq!(
            block.instructions[2],
            binary("y", BinaryOp::Mul, Value::Temp("p".into()), Value::Int(2))
        );
    }

    #[test]
    fn invalidates_dependents_when_source_is_redefined() {
        let mut block = BasicBlock::new("entry");
        block
            .instructions
            .push(assign("b", Value::Temp("a".into())));
        block.instructions.push(assign("a", Value::Int(7)));
        block.instructions.push(binary(
            "c",
            BinaryOp::Add,
            Value::Temp("b".into()),
            Value::Int(1),
        ));

        propagate_in_block(&mut block.instructions);

        assert_eq!(
            block.instructions[2],
            binary("c", BinaryOp::Add, Value::Temp("b".into()), Value::Int(1))
        );
    }

    #[test]
    fn pass_runs_across_blocks_and_reports_changes() {
        let mut cfg = Cfg::new();

        let mut block = BasicBlock::new("entry");
        block.instructions.push(assign("a", Value::Int(3)));
        block.instructions.push(binary(
            "b",
            BinaryOp::Add,
            Value::Temp("a".into()),
            Value::Int(1),
        ));
        cfg.add_block(block);

        let pass = CopyPropagationPass;
        let changed = pass.run(&mut cfg);

        assert!(changed);
        assert_eq!(
            cfg.blocks[0].instructions[1],
            binary("b", BinaryOp::Add, Value::Int(3), Value::Int(1))
        );
        assert!(!pass.run(&mut cfg));
    }
}
