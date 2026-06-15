use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Register {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

impl Register {
    // Função de conversão do enum em string que vai para um arquivo assembly
    pub fn to_asm_string(&self) -> String {
        match self {
            Register::Rax => "rax".to_string(),
            Register::Rbx => "rbx".to_string(),
            Register::Rcx => "rcx".to_string(),
            Register::Rdx => "rdx".to_string(),
            Register::R8 => "r8".to_string(),
            Register::R9 => "r9".to_string(),
            Register::R10 => "r10".to_string(),
            Register::R11 => "r11".to_string(),
            Register::R12 => "r12".to_string(),
            Register::R13 => "r13".to_string(),
            Register::R14 => "r14".to_string(),
            Register::R15 => "r15".to_string(),
            Register::Rsi => "rsi".to_string(),
            Register::Rdi => "rdi".to_string(),
        }
    }
}

// Lista de reg livres
pub struct RegisterPool {
    available: Vec<Register>,
}

impl Default for RegisterPool {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterPool {
    // Inicializa o pool com os registradores que foram definidos acima para o compilador
    pub fn new() -> Self {
        Self {
            // Inicialmente um teste com 4 registradores
            available: vec![Register::Rax, Register::Rbx, Register::Rcx, Register::Rdx],
        }
    }

    // Check-in
    pub fn allocate(&mut self) -> Option<Register> {
        self.available.pop() // Tira o reg da lista de disponível e retorna
    }

    //Check-out
    pub fn release(&mut self, reg: Register) {
        self.available.push(reg); // Volta para lista de disponíveis
    }
}

// LiveInterval
pub type TempId = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveInterval {
    pub temp: TempId, //(t0, t1,t2)
    pub start: usize,
    pub end: usize,
}

//alocador
pub struct LinearScan {
    pool: RegisterPool,
    active: Vec<LiveInterval>, // quem está ocupando o reg

    pub reg_map: HashMap<TempId, Register>, // o temporário X tá no Reg Y
    pub stack_map: HashMap<TempId, usize>, // o temporário X sofreu Spill e  tá no frama de pilha com offset Y

    next_stack_offset: usize, // controla onde colocar o proximo mano na stack
}

impl Default for LinearScan {
    fn default() -> Self {
        Self::new()
    }
}

// abre o local dos reg disponíveis e gerencia
impl LinearScan {
    pub fn new() -> Self {
        Self {
            pool: RegisterPool::new(),
            active: Vec::new(),
            reg_map: HashMap::new(),
            stack_map: HashMap::new(),
            next_stack_offset: 8,
        }
    }

    fn expire_old_intervals(&mut self, current_start: usize) {
        let mut i = 0;
        while i < self.active.len() {
            // verifica se a linha atual passou do intervalo recebido
            if self.active[i].end < current_start {
                let dead = self.active.remove(i);

                // procura no HashMap o registrador dado para o temp morto
                if let Some(reg) = self.reg_map.get(&dead.temp) {
                    self.pool.release(reg.clone());
                    println!(
                        "[Linha {}] Temporário t{} morreu. Registrador {:?} reciclado.",
                        current_start, dead.temp, reg
                    );
                }
            } else {
                i += 1;
            }
        }
    }

    pub fn allocate_register(&mut self, mut intervals: Vec<LiveInterval>) {
        //ordena cronologicamente
        intervals.sort_by_key(|i| i.start);

        for i in intervals {
            //limpa quem já is dead
            self.expire_old_intervals(i.start);

            match self.pool.allocate() {
                Some(reg) => {
                    println!(
                        "[Linha {}] Temporário t{} ganhou o registro {:?}",
                        i.start, i.temp, reg
                    );

                    //atualiza o mapa de reg
                    self.reg_map.insert(i.temp, reg);

                    //guarda o intevalo
                    self.active.push(i.clone());

                    //quem morre vai pro inicio
                    self.active.sort_by_key(|act| act.end);
                }
                None => {
                    // ta lotado
                    self.spill_at_interval(i);
                }
            }
        }
    }

    // gerencia o Spill enviando a variável com vida mais longa para a Stack
    fn spill_at_interval(&mut self, i: LiveInterval) {
        // self.active está ordanado pelo end (menor intevalo mais proximo do inicio)
        if let Some(candidate) = self.active.last().cloned() {
            if candidate.end > i.end {
                // caso 1: o candidato ativo vive mais que o novo intervalo 'i'

                if let Some(reg) = self.reg_map.remove(&candidate.temp) {
                    println!(
                        "[Spill] Roubando registrador {:?} do t{} e passando para t{}",
                        reg, candidate.temp, i.temp
                    );

                    // t_atual 'i' assume o registrador roubado
                    self.reg_map.insert(i.temp, reg);

                    // envia o antigo para a stack
                    self.stack_map
                        .insert(candidate.temp, self.next_stack_offset);
                    self.next_stack_offset += 8;

                    // remove o antigo da lista
                    self.active.pop();

                    //adiciona o novo intervalo
                    self.active.push(i);
                    self.active.sort_by_key(|act| act.end);
                }
            } else {
                //Caso 2: o prório intervalo novo 'i' vive masi que todos
                println!(
                    "[Spill] O prórpio t{} tem vida muito longa. Enviando direto para a Stack.",
                    i.temp
                );
                self.stack_map.insert(i.temp, self.next_stack_offset);
                self.next_stack_offset += 8;
            }
        } else {
            //Se o active estiver vaiza
            self.stack_map.insert(i.temp, self.next_stack_offset);
            self.next_stack_offset += 8;
        }
    }
}
