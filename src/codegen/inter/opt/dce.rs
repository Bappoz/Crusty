use super::OptPass;
use crate::codegen::inter::{Cfg, Instruction};

pub struct DeadCodeElimPass;

impl OptPass for DeadCodeElimPass {
    fn name(&self) -> &'static str {
        "dead-code-elimination"
    }

    fn run(&self, cfg: &mut Cfg) -> bool {
        let mut changed = false;

        for block in &mut cfg.blocks {
            let before = block.instructions.len();
            block
                .instructions
                .retain(|instruction| !matches!(instruction, Instruction::Nop));
            changed |= block.instructions.len() != before;
        }

        changed
    }
}
