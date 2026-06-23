pub type AsmInstr = String;

pub struct PeepholePass {
    pub window: usize,
}

impl PeepholePass {
    pub fn new() -> Self {
        Self { window: 2 }
    }

    pub fn run(&self, instrs: &mut Vec<AsmInstr>) -> bool {
        let mut overall_mutated = false;
        let mut changed = true;

        while changed {
            changed = false;
            let mut optimized = Vec::new();
            let mut i = 0;

            while i < instrs.len() {
                let l1 = &instrs[i];
                let t1 = l1.trim(); // Remove espaços antes e depois para facilitar a leitura

                // PADRÃO 3: Add/Sub por zero
                // Exemplo gerado: addq $0, %rax ou subq $0, %rcx
                if t1.starts_with("addq $0,") || t1.starts_with("subq $0,") {
                    i += 1; // Pula essa instrução (deleta)
                    changed = true;
                    continue;
                }

                // Padrões que exigem olhar 2 instruções (Janela de 2)
                if i + 1 < instrs.len() {
                    let l2 = &instrs[i + 1];
                    let t2 = l2.trim();

                    // PADRÃO 1 e 2: Mov redundante E Load após Store no mesmo endereço
                    // No formato AT&T: "movq A, B" seguido de "movq B, A"
                    if t1.starts_with("movq ") && t2.starts_with("movq ") {
                        let p1: Vec<&str> = t1.split_whitespace().collect();
                        let p2: Vec<&str> = t2.split_whitespace().collect();
                        
                        // movq [1] [2] -> [1] tem uma vírgula no final
                        if p1.len() == 3 && p2.len() == 3 {
                            let src1 = p1[1].trim_end_matches(',');
                            let dst1 = p1[2];
                            let src2 = p2[1].trim_end_matches(',');
                            let dst2 = p2[2];

                            // Se a origem do 1º for o destino do 2º e vice-versa
                            if src1 == dst2 && dst1 == src2 {
                                optimized.push(l1.clone()); // Mantém só a primeira
                                i += 2; // Pula a segunda
                                changed = true;
                                continue;
                            }
                        }
                    }

                    // PADRÃO 5: Jump para instrução seguinte
                    // Exemplo: jmp .L_main_L1 \n .L_main_L1:
                    if t1.starts_with("jmp ") && t2.ends_with(':') {
                        let target = t1.strip_prefix("jmp ").unwrap();
                        let label = t2.strip_suffix(':').unwrap();
                        
                        if target == label {
                            optimized.push(l2.clone()); // Mantém só a Label (apaga o jmp)
                            i += 2;
                            changed = true;
                            continue;
                        }
                    }

                    // PADRÃO 4: Multiplicação por potência de 2
                    // O Crusty gera: movq $8, %rcx \n imulq %rcx, %rax
                    if t1.starts_with("movq $") && t1.ends_with(", %rcx") && t2 == "imulq %rcx, %rax" {
                        let val_str = t1.strip_prefix("movq $").unwrap().strip_suffix(", %rcx").unwrap();
                        
                        if let Ok(val) = val_str.parse::<i64>() {
                            
                            if val > 0 && (val as u64).is_power_of_two() {
                                let shift = (val as u64).trailing_zeros();
                                // Substitui as duas por um shift rápido
                                optimized.push(format!("    shlq ${}, %rax", shift));
                                i += 2;
                                changed = true;
                                continue;
                            }
                        }
                    }

                    // PADRÃO 6: Compare com 0 -> Test
                    // O Crusty gera: movq $0, %rcx \n cmpq %rcx, %rax
                    if t1 == "movq $0, %rcx" && t2 == "cmpq %rcx, %rax" {
                        optimized.push("    testq %rax, %rax".to_string());
                        i += 2;
                        changed = true;
                        continue;
                    }
                }

                // Se não casou com nenhum padrão, apenas salva a instrução e vai para a próxima
                optimized.push(l1.clone());
                i += 1;
            }

            // Atualiza o vetor de instruções para a próxima rodada do while
            if changed {
                *instrs = optimized;
                overall_mutated = true;
            }
        }

        overall_mutated
    }
}