use super::Cfg;

pub mod constant_fold;
pub mod copy_prop;
pub mod cse;
pub mod dce;
pub mod inline;
pub mod licm;

pub use constant_fold::ConstantFoldPass;
pub use copy_prop::CopyPropagationPass;
pub use cse::CsePass;
pub use dce::DeadCodeElimPass;
pub use inline::InliningPass;
pub use licm::LoopInvariantCodeMotionPass;

/// Interface comum para passes de otimizacao sobre o CFG/TAC intermediario.
///
/// Um pass deve retornar `true` quando altera o `Cfg`, permitindo que o
/// `PassManager` itere ate ponto fixo.
pub trait OptPass {
    fn name(&self) -> &'static str;

    fn run(&self, cfg: &mut Cfg) -> bool;
}

pub struct PassManager {
    passes: Vec<Box<dyn OptPass>>,
}

impl PassManager {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add<P: OptPass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.passes.len()
    }

    pub fn pass_names(&self) -> Vec<&'static str> {
        self.passes.iter().map(|pass| pass.name()).collect()
    }

    /// Executa todos os passes ate ponto fixo ou `max_iter` iteracoes.
    ///
    /// Retorna o numero de iteracoes completas executadas.
    pub fn run(&self, cfg: &mut Cfg, max_iter: usize) -> usize {
        let mut iterations = 0;

        for _ in 0..max_iter {
            let mut changed = false;

            for pass in &self.passes {
                changed |= pass.run(cfg);
            }

            iterations += 1;

            if !changed {
                break;
            }
        }

        iterations
    }
}

impl Default for PassManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptLevel {
    #[default]
    O0,
    O1,
    O2,
    O3,
}

impl OptLevel {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "0" | "O0" | "-O0" => Some(Self::O0),
            "1" | "O1" | "-O1" => Some(Self::O1),
            "2" | "O2" | "-O2" => Some(Self::O2),
            "3" | "O3" | "-O3" => Some(Self::O3),
            _ => None,
        }
    }
}

pub fn pipeline_for_level(level: OptLevel) -> PassManager {
    let mut pm = PassManager::new();

    match level {
        OptLevel::O0 => {}
        OptLevel::O1 => {
            pm.add(ConstantFoldPass);
            pm.add(DeadCodeElimPass);
        }
        OptLevel::O2 => {
            pm.add(ConstantFoldPass);
            pm.add(DeadCodeElimPass);
            pm.add(CopyPropagationPass);
            pm.add(CsePass);
        }
        OptLevel::O3 => {
            pm.add(ConstantFoldPass);
            pm.add(DeadCodeElimPass);
            pm.add(CopyPropagationPass);
            pm.add(CsePass);
            pm.add(LoopInvariantCodeMotionPass);
            pm.add(InliningPass);
        }
    }

    pm
}

pub fn default_pipeline() -> PassManager {
    pipeline_for_level(OptLevel::O2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::inter::{BasicBlock, BinaryOp, Instruction, Value};

    #[test]
    fn pass_manager_runs_until_fixed_point() {
        let mut cfg = Cfg::new();
        let mut block = BasicBlock::new("entry");
        block.instructions.push(Instruction::Binary {
            dst: "t0".to_string(),
            op: BinaryOp::Add,
            lhs: Value::Int(2),
            rhs: Value::Int(3),
        });
        cfg.add_block(block);

        let mut pm = PassManager::new();
        pm.add(ConstantFoldPass);

        assert_eq!(pm.run(&mut cfg, 10), 2);
        assert_eq!(
            cfg.blocks[0].instructions[0],
            Instruction::Assign {
                dst: "t0".to_string(),
                value: Value::Int(5),
            }
        );
    }

    #[test]
    fn opt_level_selects_expected_pipeline() {
        assert!(pipeline_for_level(OptLevel::O0).is_empty());
        assert_eq!(
            pipeline_for_level(OptLevel::O1).pass_names(),
            vec!["constant-fold", "dead-code-elimination"]
        );
        assert_eq!(
            pipeline_for_level(OptLevel::O2).pass_names(),
            vec![
                "constant-fold",
                "dead-code-elimination",
                "copy-propagation",
                "common-subexpression-elimination",
            ]
        );
        assert_eq!(
            pipeline_for_level(OptLevel::O3).pass_names(),
            vec![
                "constant-fold",
                "dead-code-elimination",
                "copy-propagation",
                "common-subexpression-elimination",
                "loop-invariant-code-motion",
                "inlining",
            ]
        );
    }
}
