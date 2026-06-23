use super::OptPass;
use crate::codegen::inter::{Cfg, BasicBlock, Instruction, Value};
use std::collections::{HashMap, HashSet};

pub struct LoopInvariantCodeMotionPass;

impl OptPass for LoopInvariantCodeMotionPass {
    fn name(&self) -> &'static str {
        "loop-invariant-code-motion"
    }

    fn run(&self, cfg: &mut Cfg) -> bool {
        let predecessors = get_predecessors(cfg);
        let dominators = self.compute_dominators(cfg, &predecessors);

        let mut back_edges = Vec::new();
        for (i, block) in cfg.blocks.iter().enumerate() {
            for &succ in &block.successors {
                if let Some(doms) = dominators.get(&i) {
                    if doms.contains(&succ) {
                        back_edges.push((succ, i)); // (header, tail)
                    }
                }
            }
        }

        for (header, tail) in back_edges {
            let loop_body = self.get_loop_body(&predecessors, header, tail);
            let invariants = self.compute_invariants(cfg, &loop_body, &dominators);

            if !invariants.is_empty() {
                // Collect invariant destinations
                let invariants_set: HashSet<String> = invariants
                    .iter()
                    .filter_map(|inst| match inst {
                        Instruction::Assign { dst, .. } | Instruction::Binary { dst, .. } => {
                            Some(dst.clone())
                        }
                        _ => None,
                    })
                    .collect();

                // 1. Remove the invariant instructions from the loop body blocks
                for &block_id in &loop_body {
                    cfg.blocks[block_id].instructions.retain(|inst| match inst {
                        Instruction::Assign { dst, .. } | Instruction::Binary { dst, .. } => {
                            !invariants_set.contains(dst)
                        }
                        _ => true,
                    });
                }

                // 2. Create the preheader
                let header_label = &cfg.blocks[header].label;
                let preheader_label = format!("{}_preheader", header_label);
                let mut preheader = BasicBlock::new(preheader_label);
                preheader.instructions = invariants;
                preheader.successors.push(header + 1);

                // 3. Find outside predecessors
                let outside_preds: Vec<usize> = predecessors[header]
                    .iter()
                    .copied()
                    .filter(|pred| !loop_body.contains(pred))
                    .collect();

                // 4. Insert the preheader block at index `header`
                cfg.blocks.insert(header, preheader);

                // 5. Update successors in other blocks
                for (i, b) in cfg.blocks.iter_mut().enumerate() {
                    if i == header {
                        continue;
                    }
                    for succ in &mut b.successors {
                        if *succ >= header {
                            *succ += 1;
                        }
                    }
                }

                // 6. Redirect outside predecessors
                for pred in outside_preds {
                    let new_pred_idx = if pred >= header { pred + 1 } else { pred };
                    let pred_block = &mut cfg.blocks[new_pred_idx];
                    for succ in &mut pred_block.successors {
                        if *succ == header + 1 {
                            *succ = header;
                        }
                    }
                }

                return true;
            }
        }

        false
    }
}

fn get_predecessors(cfg: &Cfg) -> Vec<Vec<usize>> {
    let mut preds = vec![Vec::new(); cfg.blocks.len()];
    for (i, block) in cfg.blocks.iter().enumerate() {
        for &succ in &block.successors {
            if succ < preds.len() {
                preds[succ].push(i);
            }
        }
    }
    preds
}

fn uses_variable(inst: &Instruction, name: &str) -> bool {
    match inst {
        Instruction::Assign { value, .. } => {
            matches!(value, Value::Temp(n) if n == name)
        }
        Instruction::Binary { lhs, rhs, .. } => {
            matches!(lhs, Value::Temp(n) if n == name) || matches!(rhs, Value::Temp(n) if n == name)
        }
        Instruction::Nop => false,
    }
}

impl LoopInvariantCodeMotionPass {
    fn compute_dominators(
        &self,
        cfg: &Cfg,
        predecessors: &[Vec<usize>],
    ) -> HashMap<usize, HashSet<usize>> {
        let mut dominators: HashMap<usize, HashSet<usize>> = HashMap::new();
        let all_blocks: HashSet<usize> = (0..cfg.blocks.len()).collect();
        let entry = 0;

        dominators.insert(entry, vec![entry].into_iter().collect());

        for block in 1..cfg.blocks.len() {
            dominators.insert(block, all_blocks.clone());
        }

        let mut changed = true;
        while changed {
            changed = false;
            for block in 1..cfg.blocks.len() {
                let preds = &predecessors[block];
                if preds.is_empty() {
                    continue;
                }

                let mut current_intersection = dominators.get(&preds[0]).cloned().unwrap_or_default();
                for &pred in preds.iter().skip(1) {
                    if let Some(pred_doms) = dominators.get(&pred) {
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

    fn get_loop_body(
        &self,
        predecessors: &[Vec<usize>],
        header: usize,
        tail: usize,
    ) -> HashSet<usize> {
        let mut loop_body = HashSet::new();
        loop_body.insert(header);
        loop_body.insert(tail);

        let mut stack = vec![tail];

        while let Some(node) = stack.pop() {
            for &pred in &predecessors[node] {
                if !loop_body.contains(&pred) {
                    loop_body.insert(pred);
                    stack.push(pred);
                }
            }
        }
        loop_body
    }

    fn compute_invariants(
        &self,
        cfg: &Cfg,
        loop_body: &HashSet<usize>,
        dominators: &HashMap<usize, HashSet<usize>>,
    ) -> Vec<Instruction> {
        let mut invariants_set: HashSet<String> = HashSet::new();
        let mut invariant_instrs: Vec<Instruction> = Vec::new();

        let mut def_counts = HashMap::new();
        for &block_id in loop_body {
            for inst in &cfg.blocks[block_id].instructions {
                match inst {
                    Instruction::Assign { dst, .. } | Instruction::Binary { dst, .. } => {
                        *def_counts.entry(dst.clone()).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for &block_id in loop_body {
                for inst in &cfg.blocks[block_id].instructions {
                    match inst {
                        Instruction::Assign { dst, value } => {
                            if invariants_set.contains(dst) {
                                continue;
                            }
                            if def_counts.get(dst) != Some(&1) {
                                continue;
                            }
                            if self.is_value_stable(value, loop_body, &invariants_set, cfg) {
                                if self.is_safe_to_move(dst, block_id, loop_body, dominators, cfg) {
                                    invariants_set.insert(dst.clone());
                                    invariant_instrs.push(inst.clone());
                                    changed = true;
                                }
                            }
                        }
                        Instruction::Binary { dst, op: _, lhs, rhs } => {
                            if invariants_set.contains(dst) {
                                continue;
                            }
                            if def_counts.get(dst) != Some(&1) {
                                continue;
                            }
                            if self.is_value_stable(lhs, loop_body, &invariants_set, cfg)
                                && self.is_value_stable(rhs, loop_body, &invariants_set, cfg)
                            {
                                if self.is_safe_to_move(dst, block_id, loop_body, dominators, cfg) {
                                    invariants_set.insert(dst.clone());
                                    invariant_instrs.push(inst.clone());
                                    changed = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        invariant_instrs
    }

    fn is_safe_to_move(
        &self,
        dst: &str,
        def_block: usize,
        loop_body: &HashSet<usize>,
        dominators: &HashMap<usize, HashSet<usize>>,
        cfg: &Cfg,
    ) -> bool {
        // 1. Find exit blocks of the loop
        let mut exit_blocks = HashSet::new();
        for &block_id in loop_body {
            let block = &cfg.blocks[block_id];
            for &succ in &block.successors {
                if !loop_body.contains(&succ) {
                    exit_blocks.insert(block_id);
                }
            }
        }

        // 2. Check if def_block dominates all exit blocks
        let dominates_all_exits = exit_blocks.iter().all(|&exit_block| {
            if let Some(doms) = dominators.get(&exit_block) {
                doms.contains(&def_block)
            } else {
                false
            }
        });

        if dominates_all_exits {
            return true;
        }

        // 3. Check if variable is not used after/outside the loop
        let mut used_after_loop = false;
        for (i, block) in cfg.blocks.iter().enumerate() {
            if !loop_body.contains(&i) {
                for inst in &block.instructions {
                    if uses_variable(inst, dst) {
                        used_after_loop = true;
                        break;
                    }
                }
            }
        }

        !used_after_loop
    }

    fn is_value_stable(
        &self,
        val: &Value,
        loop_body: &HashSet<usize>,
        invariants: &HashSet<String>,
        cfg: &Cfg,
    ) -> bool {
        match val {
            Value::Int(_) => true,
            Value::Temp(name) => {
                if invariants.contains(name) {
                    return true;
                }
                self.is_defined_outside_loop(name, loop_body, cfg)
            }
        }
    }

    fn is_defined_outside_loop(&self, name: &str, loop_body: &HashSet<usize>, cfg: &Cfg) -> bool {
        for &block_id in loop_body {
            let block = &cfg.blocks[block_id];
            for inst in &block.instructions {
                match inst {
                    Instruction::Assign { dst, .. } | Instruction::Binary { dst, .. } => {
                        if dst == name {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
        }
        true
    }
}