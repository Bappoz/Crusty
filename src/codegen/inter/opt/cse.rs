use std::collections::HashMap;

use super::OptPass;
use crate::codegen::inter::{BinaryOp, Cfg, Instruction, Value};

/// Eliminacao local (intra-bloco) de subexpressoes comuns.
///
/// Para cada bloco basico, mantem um mapa das expressoes `lhs op rhs` ja
/// computadas. Quando a mesma expressao aparece de novo sem que `lhs`/`rhs`
/// tenham sido modificados, a instrucao redundante e removida e os usos
/// futuros do seu destino sao reescritos para o destino ja calculado.
pub struct CsePass;

impl OptPass for CsePass {
    fn name(&self) -> &'static str {
        "common-subexpression-elimination"
    }

    fn run(&self, cfg: &mut Cfg) -> bool {
        let mut changed = false;

        for block in &mut cfg.blocks {
            changed |= eliminate_in_block(&mut block.instructions);
        }

        changed
    }
}

type AvailableExprs = HashMap<(Value, BinaryOp, Value), String>;

fn eliminate_in_block(instructions: &mut Vec<Instruction>) -> bool {
    let mut changed = false;
    let mut available: AvailableExprs = HashMap::new();
    let mut rename: HashMap<String, String> = HashMap::new();
    let mut result = Vec::with_capacity(instructions.len());

    for instruction in instructions.drain(..) {
        match instruction {
            Instruction::Binary { dst, op, lhs, rhs } => {
                let lhs = resolve(&rename, lhs);
                let rhs = resolve(&rename, rhs);

                invalidate(&mut available, &mut rename, &dst);

                let key = (lhs.clone(), op, rhs.clone());
                if let Some(cached) = available.get(&key) {
                    rename.insert(dst, cached.clone());
                    changed = true;
                    continue;
                }

                available.insert(key, dst.clone());
                result.push(Instruction::Binary { dst, op, lhs, rhs });
            }
            Instruction::Assign { dst, value } => {
                let value = resolve(&rename, value);
                invalidate(&mut available, &mut rename, &dst);
                result.push(Instruction::Assign { dst, value });
            }
            other => result.push(other),
        }
    }

    *instructions = result;
    changed
}

fn resolve(rename: &HashMap<String, String>, value: Value) -> Value {
    match value {
        Value::Temp(name) => {
            let mut current = name;
            while let Some(next) = rename.get(&current) {
                if *next == current {
                    break;
                }
                current = next.clone();
            }
            Value::Temp(current)
        }
        other => other,
    }
}

/// Remove do cache qualquer expressao disponivel que dependa de `name`,
/// seja como operando ou como destino ja calculado, pois `name` esta
/// sendo redefinido.
fn invalidate(available: &mut AvailableExprs, rename: &mut HashMap<String, String>, name: &str) {
    available.retain(|(lhs, _, rhs), cached| {
        !references(lhs, name) && !references(rhs, name) && cached != name
    });
    rename.retain(|_, target| target != name);
}

fn references(value: &Value, name: &str) -> bool {
    matches!(value, Value::Temp(n) if n == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::inter::BasicBlock;

    fn binary(dst: &str, op: BinaryOp, lhs: Value, rhs: Value) -> Instruction {
        Instruction::Binary {
            dst: dst.to_string(),
            op,
            lhs,
            rhs,
        }
    }

    #[test]
    fn eliminates_repeated_expression_in_same_block() {
        let mut block = BasicBlock::new("entry");
        block.instructions.push(binary(
            "t1",
            BinaryOp::Add,
            Value::Temp("a".into()),
            Value::Temp("b".into()),
        ));
        block.instructions.push(binary(
            "t2",
            BinaryOp::Add,
            Value::Temp("a".into()),
            Value::Temp("b".into()),
        ));
        block.instructions.push(Instruction::Assign {
            dst: "y".to_string(),
            value: Value::Temp("t2".into()),
        });

        let changed = eliminate_in_block(&mut block.instructions);

        assert!(changed);
        assert_eq!(
            block.instructions,
            vec![
                binary(
                    "t1",
                    BinaryOp::Add,
                    Value::Temp("a".into()),
                    Value::Temp("b".into())
                ),
                Instruction::Assign {
                    dst: "y".to_string(),
                    value: Value::Temp("t1".into()),
                },
            ]
        );
    }

    #[test]
    fn does_not_eliminate_when_operand_is_redefined_between_uses() {
        let mut block = BasicBlock::new("entry");
        block.instructions.push(binary(
            "t1",
            BinaryOp::Add,
            Value::Temp("a".into()),
            Value::Temp("b".into()),
        ));
        block.instructions.push(Instruction::Assign {
            dst: "a".to_string(),
            value: Value::Int(5),
        });
        block.instructions.push(binary(
            "t2",
            BinaryOp::Add,
            Value::Temp("a".into()),
            Value::Temp("b".into()),
        ));

        let changed = eliminate_in_block(&mut block.instructions);

        assert!(!changed);
        assert_eq!(block.instructions.len(), 3);
    }

    #[test]
    fn keeps_different_expressions_in_cache_independently() {
        let mut block = BasicBlock::new("entry");
        block.instructions.push(binary(
            "x",
            BinaryOp::Mul,
            Value::Temp("a".into()),
            Value::Temp("b".into()),
        ));
        block.instructions.push(binary(
            "y",
            BinaryOp::Sub,
            Value::Temp("a".into()),
            Value::Temp("d".into()),
        ));

        let changed = eliminate_in_block(&mut block.instructions);

        assert!(!changed);
        assert_eq!(block.instructions.len(), 2);
    }

    #[test]
    fn pass_reports_changes_across_multiple_blocks() {
        let mut cfg = Cfg::new();

        let mut block = BasicBlock::new("entry");
        block.instructions.push(binary(
            "t1",
            BinaryOp::Add,
            Value::Temp("a".into()),
            Value::Temp("b".into()),
        ));
        block.instructions.push(binary(
            "t2",
            BinaryOp::Add,
            Value::Temp("a".into()),
            Value::Temp("b".into()),
        ));
        cfg.add_block(block);

        let pass = CsePass;
        let changed = pass.run(&mut cfg);

        assert!(changed);
        assert_eq!(cfg.blocks[0].instructions.len(), 1);
        assert!(!pass.run(&mut cfg));
    }
}
