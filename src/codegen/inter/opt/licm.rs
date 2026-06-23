use super::OptPass;
use crate::codegen::inter::{Cfg, BlockId};
use std::collections::{HashMap, HashSet};
use crate::ir::tac::{TacInstr, Operand, UnOp};

pub struct LoopInvariantCodeMotionPass;

impl OptPass for LoopInvariantCodeMotionPass {
    fn name(&self) -> &'static str {
        "loop-invariant-code-motion"
    }

    fn run(&self, cfg: &mut Cfg) -> bool {
        let dominators = self.compute_dominators(cfg);

        let mut mutated = false;
        let mut back_edge = Vec::new(); // aresta que liga o bloco dominante a dominado

        for &block in cfg.blocks.keys(){
            if let Some(cfg_block) = cfg.blocks.get(&block){
                for &succ in &cfg_block.successors {
                    if let Some(doms) = dominators.get(&block){
                        if doms.contains(&succ){
                            back_edge.push((succ, block));
                        }
                    }
                }

            }
        }

        for (header, tail) in back_edge {
            let loop_body = self.get_loop_body(cfg, header, tail);

            println!("Laço detectado! Header {:?} Tail {:?}", header, tail);
            println("Blocos pertencentes ao laço: {:?}", loop_body);

            let invariants = self.compute_invariants(cfg, &loop_body);
            println!("Operandos invariantes encontrados: {:?}", invariants);
        }
        mutated
    }
}

impl LoopInvariantCodeMotionPass{
    fn compute_dominators(&self, cfg: &Cfg) -> HashMap<BlockId, HashSet<BlockId>>{
        let mut dominators: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
        let all_blocks: HashSet<BlockId> = cfg.blocks.keys().cloned().collect();
        let entry = cfg.entry_block;

        dominators.insert(entry, vec![entry].into_iter().collect());

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
        dominators
    
    }

    fn get_loop_body(&self, cfg: &Cfg, header: BlockId, tail: BlockId) -> HashSet<BlockId> {
        let mut loop_body = HashSet::new();
        loop_body.insert(header);
        loop_body.insert(tail);

        let mut stack = vec![tail];

        while let Some(node) = stack.pop(){
            for pred in cfg.get_predecessors(node){
                if !loop_body.contains(&pred){
                    loop_body.insert(pred);
                    stack.push(pred);
                }
            }
        }
        loop_body
    }

fn compute_invariants(&self, cfg: &Cfg, loop_body: &HashSet<BlockId>) -> HashSet<Operand> {
        let mut invariants = HashSet::new();
        let mut changed = true;

        while changed {
            changed = false;

            for &block_id in loop_body {
                let cfg_block = cfg.blocks.get(&block_id).unwrap();
                
                for inst in &cfg_block.instructions {
                    match inst {
                        // 1. Operações Binárias (ex: t0 = t1 + t2)
                        TacInstr::BinOp { dst, lhs, rhs, .. } => {
                            let dst_operand = Operand::Temp(*dst);
                            if invariants.contains(&dst_operand) { continue; }

                            if self.is_operand_stable(cfg, lhs, loop_body, &invariants) && 
                               self.is_operand_stable(cfg, rhs, loop_body, &invariants) {
                                
                                invariants.insert(dst_operand);
                                changed = true;
                            }
                        }
                        
                        // 2. Operações Unárias (ex: t0 = -t1)
                        TacInstr::UnOp { dst, op, src } => {
                            let dst_operand = Operand::Temp(*dst);
                            if invariants.contains(&dst_operand) { continue; }

                            // Proteção: Desreferenciar (*p) ou pegar endereço (&x) pode ter efeitos colaterais
                            if matches!(op, UnOp::Deref | UnOp::AddrOf) { continue; }

                            if self.is_operand_stable(cfg, src, loop_body, &invariants) {
                                invariants.insert(dst_operand);
                                changed = true;
                            }
                        }
                        
                        // 3. Cópias / Atribuições (ex: t0 = 5 ou t0 = t1)
                        TacInstr::Copy { dst, src } => {
                            if invariants.contains(dst) { continue; }

                            if self.is_operand_stable(cfg, src, loop_body, &invariants) {
                                invariants.insert(dst.clone());
                                changed = true;
                            }
                        }

                        // Call, Return, Jump, Label não geram valores invariantes seguros para mover
                        _ => {}
                    }
                }
            }
        }
        invariants
    }

    fn is_operand_stable(
        &self, 
        cfg: &Cfg, 
        op: &Operand, 
        loop_body: &HashSet<BlockId>, 
        invariants: &HashSet<Operand>
    ) -> bool {
        match op {
            // Se for uma constante literal, é sempre estável!
            Operand::Const(_) => true,
            
            // Se for uma variável (Temp ou Var), precisamos ver de onde ela veio
            Operand::Temp(_) | Operand::Var(_) => {
                // Se a instrução que a criou já foi marcada como invariante
                if invariants.contains(op) {
                    return true;
                }
                
                // Se ela foi definida FORA do laço, é estável
                self.is_defined_outside_loop(cfg, op, loop_body)
            }
        }
    }

    // A função que rastreia se o valor nasceu fora do laço
    fn is_defined_outside_loop(
        &self, 
        cfg: &Cfg, 
        op: &Operand, 
        loop_body: &HashSet<BlockId>
    ) -> bool {
        // Varrer apenas os blocos de DENTRO do laço
        for &block_id in loop_body {
            let block = cfg.blocks.get(&block_id).unwrap();
            
            for inst in &block.instructions {
                match inst {
                    TacInstr::BinOp { dst, .. } | 
                    TacInstr::UnOp { dst, .. } if &Operand::Temp(*dst) == op => {
                        return false; // Nasceu DENTRO do laço
                    }
                    TacInstr::Copy { dst, .. } if dst == op => {
                        return false; // Nasceu DENTRO do laço
                    }
                    TacInstr::Call { dst: Some(dst), .. } if &Operand::Temp(*dst) == op => {
                        return false; // Nasceu DENTRO do laço
                    }
                    _ => {}
                }
            }
        }
       
        true
    }
} 