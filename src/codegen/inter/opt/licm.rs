use super::OptPass;
use crate::codegen::inter::{Cfg, BlockId};
use crate::collections::{HashMap, HashSet};

pub struct LoopInvariantCodeMotionPass;

impl OptPass for LoopInvariantCodeMotionPass {
    fn name(&self) -> &'static str {
        "loop-invariant-code-motion"
    }

    fn run(&self, _cfg: &mut Cfg) -> bool {
        let dominators = self.compute_dominators(cfg);

        let mut mutated = false;
        let mut back_edge = Vec::new(); // aresta que liga o bloco dominante a dominado

        for &block in cfg.block.keys(){
            if let Some(cfg_block) = cfg.blocks.get(&block){
                for &succ in &cfg_block.succesors {
                    if let Some(doms) = dominators.get(&block){
                        if doms.contains(&succ){
                            back_edges.push((succ, block));
                        }
                    }
                }

            }
        }

        for (header, tail) in back_edges {
            let loop_body = self.get_loop_body(cfg, header, tail);

            prtinln("Laço detectado! Header {:?}", header, tail);
            println("Blocos pertencentes ao laço: {:?}", loop_body);

            TODO:
        }
    }
}
