use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::ir::tac::{LabelId, TacFunction, TacInstr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "B{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instrs: Vec<TacInstr>,
    pub succs: Vec<BlockId>,
    pub preds: Vec<BlockId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cfg {
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    pub exit: BlockId,
}

impl Cfg {
    pub fn predecessors(&self, id: BlockId) -> &[BlockId] {
        &self.blocks[id.0 as usize].preds
    }

    pub fn successors(&self, id: BlockId) -> &[BlockId] {
        &self.blocks[id.0 as usize].succs
    }
}

impl fmt::Display for Cfg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for block in &self.blocks {
            writeln!(f, "{}:", block.id)?;
            for instr in &block.instrs {
                writeln!(f, "  {instr}")?;
            }
            let succs = block
                .succs
                .iter()
                .map(BlockId::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(f, "  succs: [{succs}]")?;
        }
        Ok(())
    }
}

/// Indices in `instrs` that start a new basic block: the function start,
/// label targets, and the instruction right after a Jump/CondJump.
pub fn identify_leaders(instrs: &[TacInstr]) -> HashSet<usize> {
    let mut leaders = HashSet::new();
    if instrs.is_empty() {
        return leaders;
    }

    leaders.insert(0);

    for (index, instr) in instrs.iter().enumerate() {
        match instr {
            TacInstr::Jump { .. } | TacInstr::CondJump { .. } => {
                if index + 1 < instrs.len() {
                    leaders.insert(index + 1);
                }
            }
            TacInstr::Label(_) => {
                leaders.insert(index);
            }
            _ => {}
        }
    }

    leaders
}

pub fn build_cfg(func: &TacFunction) -> Cfg {
    let mut leaders: Vec<usize> = identify_leaders(&func.instrs).into_iter().collect();
    leaders.sort_unstable();

    if leaders.is_empty() {
        let entry = BlockId(0);
        let block = BasicBlock {
            id: entry,
            instrs: Vec::new(),
            succs: Vec::new(),
            preds: Vec::new(),
        };
        return Cfg {
            blocks: vec![block],
            entry,
            exit: entry,
        };
    }

    let mut blocks = Vec::with_capacity(leaders.len());
    let mut label_to_block: HashMap<LabelId, BlockId> = HashMap::new();

    for (block_index, &start) in leaders.iter().enumerate() {
        let end = leaders
            .get(block_index + 1)
            .copied()
            .unwrap_or(func.instrs.len());
        let instrs = func.instrs[start..end].to_vec();
        let id = BlockId(block_index as u32);

        if let Some(TacInstr::Label(label)) = instrs.first() {
            label_to_block.insert(*label, id);
        }

        blocks.push(BasicBlock {
            id,
            instrs,
            succs: Vec::new(),
            preds: Vec::new(),
        });
    }

    for block_index in 0..blocks.len() {
        let succs = match blocks[block_index].instrs.last() {
            Some(TacInstr::Jump { label }) => vec![label_to_block[label]],
            Some(TacInstr::CondJump {
                then_label,
                else_label,
                ..
            }) => vec![label_to_block[then_label], label_to_block[else_label]],
            Some(TacInstr::Return { .. }) => Vec::new(),
            _ => {
                if block_index + 1 < blocks.len() {
                    vec![BlockId((block_index + 1) as u32)]
                } else {
                    Vec::new()
                }
            }
        };
        blocks[block_index].succs = succs;
    }

    for block_index in 0..blocks.len() {
        let id = blocks[block_index].id;
        for succ in blocks[block_index].succs.clone() {
            blocks[succ.0 as usize].preds.push(id);
        }
    }

    let entry = BlockId(0);
    let exit = blocks
        .iter()
        .rev()
        .find(|block| block.succs.is_empty())
        .map(|block| block.id)
        .unwrap_or_else(|| blocks.last().unwrap().id);

    Cfg {
        blocks,
        entry,
        exit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::tac::{ConstValue, Operand, TempId};

    fn func(instrs: Vec<TacInstr>) -> TacFunction {
        TacFunction {
            name: "test".to_string(),
            params: Vec::new(),
            instrs,
        }
    }

    #[test]
    fn straight_line_code_is_one_block() {
        let f = func(vec![
            TacInstr::Copy {
                dst: TempId(0),
                src: Operand::Const(ConstValue::Int(1)),
            },
            TacInstr::Copy {
                dst: TempId(1),
                src: Operand::Const(ConstValue::Int(2)),
            },
            TacInstr::Return {
                val: Some(Operand::Temp(TempId(1))),
            },
        ]);

        let cfg = build_cfg(&f);

        assert_eq!(cfg.blocks.len(), 1);
    }

    #[test]
    fn if_else_produces_three_blocks() {
        let cond = Operand::Const(ConstValue::Int(1));
        let then_label = LabelId(0);
        let else_label = LabelId(1);
        let merge_label = LabelId(2);

        let f = func(vec![
            TacInstr::CondJump {
                cond,
                then_label,
                else_label,
            },
            TacInstr::Label(then_label),
            TacInstr::Copy {
                dst: TempId(0),
                src: Operand::Const(ConstValue::Int(1)),
            },
            TacInstr::Jump { label: merge_label },
            TacInstr::Label(else_label),
            TacInstr::Copy {
                dst: TempId(0),
                src: Operand::Const(ConstValue::Int(2)),
            },
            TacInstr::Label(merge_label),
            TacInstr::Return {
                val: Some(Operand::Temp(TempId(0))),
            },
        ]);

        let cfg = build_cfg(&f);

        assert!(cfg.blocks.len() >= 3);
    }

    #[test]
    fn while_loop_has_back_edge() {
        let cond_label = LabelId(0);
        let body_label = LabelId(1);
        let exit_label = LabelId(2);
        let cond = Operand::Const(ConstValue::Int(1));

        let f = func(vec![
            TacInstr::Label(cond_label),
            TacInstr::CondJump {
                cond,
                then_label: body_label,
                else_label: exit_label,
            },
            TacInstr::Label(body_label),
            TacInstr::Copy {
                dst: TempId(0),
                src: Operand::Const(ConstValue::Int(1)),
            },
            TacInstr::Jump { label: cond_label },
            TacInstr::Label(exit_label),
            TacInstr::Return { val: None },
        ]);

        let cfg = build_cfg(&f);

        let cond_block = cfg
            .blocks
            .iter()
            .find(|b| matches!(b.instrs.first(), Some(TacInstr::Label(l)) if *l == cond_label))
            .unwrap()
            .id;
        let body_block = cfg
            .blocks
            .iter()
            .find(|b| matches!(b.instrs.first(), Some(TacInstr::Label(l)) if *l == body_label))
            .unwrap()
            .id;

        assert!(cfg.successors(body_block).contains(&cond_block));
    }

    #[test]
    fn cfg_entry_has_no_predecessors() {
        let f = func(vec![TacInstr::Return { val: None }]);

        let cfg = build_cfg(&f);

        assert!(cfg.predecessors(cfg.entry).is_empty());
    }
}
