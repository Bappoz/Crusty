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

impl LoopInvariantCodeMotionPass{
    fn compute_dominators(&self, cfg: &Cfg) -> HashMap<BLockId, HashSet<BlockId>>{
        let mut dominators: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
        let all_blocks: HashSet<BlockId> = cfg.blocks.key().cloned().collect();
        let entry = cfg.entry_block;

        dominators.insert(entry, vec![entry].intro_iter().collect());

        for &block in &all_blocks{
            if block != entry {
                dominators.insert(block, all_blocks.clone()); // (key, value) se já existi no dicionario subtitui
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for &block in &all_blocks{
                if block == entry {continue;}

                let preds = cfg.get_predecessors(block);
                if preds.is_empty() {continue;}

                let mut current_intersection = dominators.get(&preds[0]).cloned().unwrap_or_default();
                for pred in preds.iter().skip(1) {
                    if let Some(pred_doms) = dominators.get(pred) {
                        current_intersection = current_intersection
                            .intersection(pred_doms)
                            .cloned()
                            .collect();
                    }
                }
                current_intersection.insert(block);

                let old_doms = dominators.get(&block).unwrap();
                if &current_intersection != old_doms {
                    dominators.insert(block, current_intersection);
                    changed = true;
            }
        }

    }
    
}