//! Selecao de instrucoes e emissao de assembly x86-64 (sintaxe AT&T / GAS).
//!
//! O backend traduz cada instrucao da TAC para uma ou mais instrucoes x86-64.
//! A estrategia adotada e a geracao "naive" (sem alocacao de registradores):
//! todo temp e variavel vive em um slot da stack, e usamos `%rax`/`%rcx` como
//! scratch para computar cada operacao. Isso mantem a correcao enquanto o
//! alocador (`codegen::reg_alloc`) nao esta integrado ao fluxo de TAC.
//!
//! Convencao de chamada: System V AMD64 ABI.
//! - Argumentos inteiros: `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`
//! - Retorno: `rax`
//! - Stack alinhada em 16 bytes no ponto de `call`.
//!
//! A saida pode ser montada diretamente com `gcc -x assembler-with-cpp` ou
//! `as`.

use crate::codegen::last::abi;
use crate::codegen::last::frame::{Frame, SlotKey};
use crate::codegen::last::peephole::PeepholePass;
use crate::common::ast::expr::{BinOp, UnOp};
use crate::common::errors::types::CodegenError;
use crate::ir::tac::{ConstValue, LabelId, Operand, TacFunction, TacInstr, TacProgram};
use std::collections::HashMap;
use crate::common::ast::ast::Type;

type EmitResult<T> = Result<T, CodegenError>;

/// Registrador escolhido para materializar o endereco de um `Operand::Deref`
/// antes do deref propriamente dito. Caller-saved e nunca usado como `reg`
/// pelos chamadores de `load_op`/`store_op` neste backend (que usam apenas
/// `rax`/`rcx`/registradores de argumento), entao e seguro como scratch
/// dedicado mesmo em derefs aninhados.
const DEREF_SCRATCH_REG: &str = "r11";

/// Acumulador de linhas de assembly com indentacao controlada.
struct Emitter {
    out: String,
}

#[derive(Default)]
struct StringPool {
    entries: Vec<(String, String)>,
    labels: HashMap<String, String>,
    double_entries: Vec<(String, f64)>,
    double_labels: HashMap<String, String>,
}

impl StringPool {
    fn collect(prog: &TacProgram) -> Self {
        let mut pool = Self::default();
        for func in &prog.functions {
            for instr in &func.instrs {
                pool.visit_instr(instr);
            }
        }
        pool
    }

    fn visit_instr(&mut self, instr: &TacInstr) {
        match instr {
            TacInstr::BinOp { lhs, rhs, .. } => {
                self.visit_operand(lhs);
                self.visit_operand(rhs);
            }
            TacInstr::UnOp { src, .. } => self.visit_operand(src),
            TacInstr::Copy { dst, src } => {
                self.visit_operand(dst);
                self.visit_operand(src);
            }
            TacInstr::CondJump { cond, .. } => self.visit_operand(cond),
            TacInstr::Call { args, .. } => {
                for arg in args {
                    self.visit_operand(arg);
                }
            }
            TacInstr::Return { val } => {
                if let Some(val) = val {
                    self.visit_operand(val);
                }
            }
            TacInstr::Jump { .. } | TacInstr::Label(_) => {}
        }
    }

    fn visit_operand(&mut self, op: &Operand) {
        match op {
            Operand::Const(ConstValue::String(value)) => {
                self.label_for(value);
            }

            Operand::Const(ConstValue::Double(value)) => {
                self.label_for_double(*value);
            }
            Operand::Deref(inner) => self.visit_operand(inner),
            _=> {}
        }
    }

    fn label_for(&mut self, value: &str) -> String {
        if let Some(label) = self.labels.get(value) {
            return label.clone();
        }

        let label = format!(".LC{}", self.entries.len());
        self.entries.push((label.clone(), value.to_string()));
        self.labels.insert(value.to_string(), label.clone());
        label
    }

    fn label_for_double(&mut self, value: f64) -> String{
        let key = value.to_string();
        if let Some(label) = self.double_labels.get(&key){
            return label.clone();
        }

        let label = format!(".LC_DBL{}", self.double_entries.len());
        self.double_entries.push((label.clone(), value));
        self.double_labels.insert(key, label.clone());
        label
    }
}

impl Emitter {
    fn new() -> Self {
        Self { out: String::new() }
    }

    /// Instrucao: indentada em 4 espacos.
    fn insn(&mut self, text: &str) {
        self.out.push_str("    ");
        self.out.push_str(text);
        self.out.push('\n');
    }

    /// Diretiva/rotulo: sem indentacao (ex.: `.text`, `main:`).
    fn raw(&mut self, text: &str) {
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn comment(&mut self, text: &str) {
        self.out.push_str("    # ");
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn blank(&mut self) {
        self.out.push('\n');
    }

    fn append_str(&mut self, text: &str) {
        self.out.push_str(text);
    }

    fn into_string(self) -> String {
        self.out
    }
}

/// Emite o assembly de um programa TAC completo, prefixando a diretiva de
/// secao `.text`.
pub fn emit_program(prog: &TacProgram) -> EmitResult<String> {
    let strings = StringPool::collect(prog);
    let mut em = Emitter::new();

    if !strings.entries.is_empty() || !strings.double_entries.is_empty() {
        em.raw(".section .rodata");

        for (label, value) in &strings.entries {
            em.raw(&format!("{label}:"));
            em.raw(&format!("    .asciz {}", escape_asm_string(value)));
        }

        for (label, value) in &strings.double_entries{
            em.raw(&format!("{label}:"));
            em.raw(&format!("   .double {value}"));
        }
        em.blank();
    }
    em.raw(".text");
    for func in &prog.functions {
        em.blank();
        em.append_str(&emit_function(func, &strings)?);
    }
    // Marca a stack como nao-executavel em formatos ELF. Essa secao nao
    // existe no COFF usado pelo MinGW e tornaria o assembly invalido la.
    em.blank();
    #[cfg(not(target_os = "windows"))]
    em.raw(".section .note.GNU-stack,\"\",@progbits");
    Ok(em.into_string())
}

/// Emite o assembly de uma unica funcao: directiva `.globl`, rotulo,
/// prologue, corpo e epilogue.
fn emit_function(func: &TacFunction, strings: &StringPool) -> EmitResult<String> {
    let mut em = Emitter::new();
    em.comment(&format!("function {}", func.name));
    em.raw(&format!(".globl {}", func.name));
    em.raw(&format!("{}:", func.name));

    let frame = build_frame(func);

    // Prologue
    em.insn("pushq %rbp");
    em.insn("movq %rsp, %rbp");
    let frame_size = frame.frame_size();
    if frame_size > 0 {
        em.insn(&format!("subq ${frame_size}, %rsp"));
    }

    // Spill dos argumentos recebidos em registrador para seus slots locais.
    for (index, name) in func.params.iter().enumerate() {
        if let Some(reg) = abi::arg_register(index) {
            let offset = frame
                .offset_of(&SlotKey::Var(name.clone()))
                .expect("slot do parametro deve estar alocado");
            em.insn(&format!("movq %{reg}, {offset}(%rbp)"));
        }
    }

    // Corpo
    let epilogue_label = format!(".L_{}_epilogue", func.name);
    for instr in &func.instrs {
        emit_instr(&mut em, instr, &frame, &func.name, &epilogue_label, strings)?;
    }

    // Epilogue (alvo de todos os `return`). Caso a funcao nao tenha `return`
    // explicito, a queda natural chega aqui e retorna o que estiver em %rax.
    em.raw(&format!("{epilogue_label}:"));
    em.insn("movq %rbp, %rsp");
    em.insn("popq %rbp");
    em.insn("ret");

    // Aplica a otimizacao peephole sobre o assembly gerado, linha a linha,
    // ate atingir ponto fixo.
    let raw_asm = em.into_string();
    let mut instrs: Vec<String> = raw_asm.lines().map(|s| s.to_string()).collect();
    let peephole = PeepholePass::new();
    peephole.run(&mut instrs);

    Ok(instrs.join("\n"))
}

/// Constroi o stack frame pre-escaneando todas as instrucoes para alocar um
/// slot para cada temp/variavel e mapear os parametros para suas posicoes.
///
/// Variaveis listadas em `func.var_sizes` (hoje, structs locais) recebem um
/// bloco contiguo do tamanho indicado em vez do slot escalar padrao de 8
/// bytes; demais variaveis e temporarios usam sempre 8 bytes.
fn build_frame(func: &TacFunction) -> Frame {
    let mut frame = Frame::new();

    let size_of = |name: &str| func.var_sizes.get(name).copied().unwrap_or(8);

    for (index, name) in func.params.iter().enumerate() {
        let key = SlotKey::Var(name.clone());
        match abi::arg_register(index) {
            Some(_) => {
                frame.allocate_local_sized(key, size_of(name));
            }
            None => {
                // Argumento passado via stack do chamador: ja esta disponivel
                // em offset positivo a partir de %rbp.
                frame.set_offset(key, abi::stack_arg_offset(index));
            }
        }
    }

    for instr in &func.instrs {
        for key in slot_keys_of(instr) {
            // Nao realoca stack-args (offsets positivos) que ja foram mapeados.
            if frame.offset_of(&key).is_some() {
                continue;
            }
            let size = match &key {
                SlotKey::Var(name) => size_of(name),
                SlotKey::Temp(_) => 8,
            };
            frame.allocate_local_sized(key, size);
        }
    }

    frame
}

/// Coleta todas as chaves de slot referenciadas por uma instrucao (dst,
/// fontes e operandos de leitura).
fn slot_keys_of(instr: &TacInstr) -> Vec<SlotKey> {
    let mut keys = Vec::new();

    let consider = |keys: &mut Vec<SlotKey>, op: &Operand| {
        if let Some(key) = SlotKey::from_operand(op) {
            keys.push(key);
        }
    };

    match instr {
        TacInstr::BinOp { dst, lhs, rhs, .. } => {
            keys.push(SlotKey::Temp(dst.0));
            consider(&mut keys, lhs);
            consider(&mut keys, rhs);
        }
        TacInstr::UnOp { dst, src, .. } => {
            keys.push(SlotKey::Temp(dst.0));
            consider(&mut keys, src);
        }
        TacInstr::Copy { dst, src } => {
            consider(&mut keys, dst);
            consider(&mut keys, src);
        }
        TacInstr::CondJump { cond, .. } => consider(&mut keys, cond),
        TacInstr::Call { dst, args, .. } => {
            if let Some(dst) = dst {
                keys.push(SlotKey::Temp(dst.0));
            }
            for arg in args {
                consider(&mut keys, arg);
            }
        }
        TacInstr::Return { val } => {
            if let Some(val) = val {
                consider(&mut keys, val);
            }
        }
        TacInstr::Jump { .. } | TacInstr::Label(_) => {}
    }

    keys
}

fn emit_instr(
    em: &mut Emitter,
    instr: &TacInstr,
    frame: &Frame,
    func_name: &str,
    epilogue_label: &str,
    strings: &StringPool,
) -> EmitResult<()> {
    match instr {
        TacInstr::Label(label) => {
            em.raw(&format!("{}:", local_label(func_name, label)));
            Ok(())
        }
        TacInstr::Jump { label } => {
            em.insn(&format!("jmp {}", local_label(func_name, label)));
            Ok(())
        }
        TacInstr::CondJump {
            cond,
            then_label,
            else_label,
        } => {
            load_op(em, frame, cond, "rax", strings, &Type::Int)?;
            em.insn("testq %rax, %rax");
            em.insn(&format!("jne {}", local_label(func_name, then_label)));
            em.insn(&format!("jmp {}", local_label(func_name, else_label)));
            Ok(())
        }
        TacInstr::Copy { dst, src, ty } => {
            let reg = if matches!(ty, Type::Double) { "xmm0" } else { "rax" };
            load_op(em, frame, src, reg, strings, ty)?;
            store_op(em, frame, dst, reg, strings, ty)?;
            Ok(())
        }
        TacInstr::BinOp { dst, op, lhs, rhs, ty } => emit_binop(em, op, lhs, rhs, *dst, frame, strings, ty),
        TacInstr::UnOp { dst, op, src, ty } => emit_unop(em, op, src, *dst, frame, strings, ty),
        TacInstr::Call { dst, fn_name, args } => emit_call(em, fn_name, args, *dst, frame, strings),
        TacInstr::Return { val, ty } => {
            if let Some(val) = val {
                // 1. Resolve o tipo (se for None, assume Int)
                let resolved_ty = ty.as_ref().unwrap_or(&Type::Int);
                // 2. Escolhe o registrador de retorno correto
                let reg = if matches!(resolved_ty, Type::Double) { "xmm0" } else { "rax" };
                // 3. Carrega o valor para o registrador
                load_op(em, frame, val, reg, strings, resolved_ty)?;
            }
            em.insn(&format!("jmp {epilogue_label}"));
            Ok(())
        }
    }
}

fn emit_binop(
    em: &mut Emitter,
    op: &BinOp,
    lhs: &Operand,
    rhs: &Operand,
    dst: crate::ir::tac::TempId,
    frame: &Frame,
    strings: &StringPool,
    ty: &Type, // <-- Faltava isto
) -> EmitResult<()> {
    if matches!(op, BinOp::And | BinOp::Or) {
        emit_logical(em, matches!(op, BinOp::Or), lhs, rhs, dst, frame, strings)?;
        return Ok(());
    }

    if matches!(ty, Type::Double) {
        load_op(em, frame, lhs, "xmm0", strings, ty)?;
        load_op(em, frame, rhs, "xmm1", strings, ty)?;

        match op {
            BinOp::Add => em.insn("addsd %xmm1, %xmm0"),
            BinOp::Sub => em.insn("subsd %xmm1, %xmm0"),
            BinOp::Mul => em.insn("mulsd %xmm1, %xmm0"),
            BinOp::Div => em.insn("divsd %xmm1, %xmm0"),
            BinOp::Less => emit_comparison_double(em, "seta"),
            BinOp::Greater => emit_comparison_double(em, "setb"),
            BinOp::Leq => emit_comparison_double(em, "setae"),
            BinOp::Geq => emit_comparison_double(em, "setbe"),
            BinOp::Eq => emit_comparison_double(em, "sete"),
            BinOp::Neq => emit_comparison_double(em, "setne"),
            _ => return Err(codegen_error("Operacao double nao suportada", Some("binop"))),
        }
        
        let is_relational = matches!(op, BinOp::Less | BinOp::Greater | BinOp::Leq | BinOp::Geq | BinOp::Eq | BinOp::Neq);
        if is_relational {
            store_op(em, frame, &Operand::Temp(dst), "rax", strings, &Type::Int)?;
        } else {
            store_op(em, frame, &Operand::Temp(dst), "xmm0", strings, ty)?;
        }
        
        return Ok(());
    }

   
    load_op(em, frame, lhs, "rax", strings, ty)?;
    load_op(em, frame, rhs, "rcx", strings, ty)?;

    match op {
        BinOp::Add => em.insn("addq %rcx, %rax"),
        BinOp::Sub => em.insn("subq %rcx, %rax"),
        BinOp::Mul => em.insn("imulq %rcx, %rax"),
        BinOp::Div => {
            em.insn("cqto");
            em.insn("idivq %rcx");
        }
        BinOp::Mod => {
            em.insn("cqto");
            em.insn("idivq %rcx");
            em.insn("movq %rdx, %rax");
        }
        BinOp::BitAnd => em.insn("andq %rcx, %rax"),
        BinOp::BitOr => em.insn("orq %rcx, %rax"),
        BinOp::BitXor => em.insn("xorq %rcx, %rax"),
        BinOp::Shl => em.insn("shlq %cl, %rax"),
        BinOp::Shr => em.insn("sarq %cl, %rax"),
        BinOp::Less => emit_comparison(em, "setl"),
        BinOp::Greater => emit_comparison(em, "setg"),
        BinOp::Leq => emit_comparison(em, "setle"),
        BinOp::Geq => emit_comparison(em, "setge"),
        BinOp::Eq => emit_comparison(em, "sete"),
        BinOp::Neq => emit_comparison(em, "setne"),
        BinOp::And | BinOp::Or => unreachable!(),
    }

    store_op(em, frame, &Operand::Temp(dst), "rax", strings, ty)?;
    Ok(())
}

fn emit_comparison(em: &mut Emitter, setcc: &str) {
    em.insn("cmpq %rcx, %rax");
    em.insn(&format!("{setcc} %al"));
    em.insn("movzbq %al, %rax");
}

fn emit_comparison_double(em: &mut Emitter, setcc: &str) {
    em.insn("ucomisd %xmm1, %xmm0");
    em.insn(&format!("{setcc} %al"));
    em.insn("movzbq %al, %rax");
}

fn emit_logical(
    em: &mut Emitter,
    is_or: bool,
    lhs: &Operand,
    rhs: &Operand,
    dst: crate::ir::tac::TempId,
    frame: &Frame,
    strings: &StringPool,
) -> EmitResult<()> {

    let ty = &Type::Int;
    // Normaliza lhs para 0/1 em %rdx.
    load_op(em, frame, lhs, "rax", strings, ty)?;
    em.insn("testq %rax, %rax");
    em.insn("setne %al");
    em.insn("movzbq %al, %rax");
    em.insn("movq %rax, %rdx");

    // Normaliza rhs para 0/1 em %rax.
    load_op(em, frame, rhs, "rax", strings, ty)?;
    em.insn("testq %rax, %rax");
    em.insn("setne %al");
    em.insn("movzbq %al, %rax");

    if is_or {
        em.insn("orq %rdx, %rax");
    } else {
        em.insn("andq %rdx, %rax");
    }

    store_op(em, frame, &Operand::Temp(dst), "rax", strings)?;
    Ok(())
}

fn emit_unop(
    em: &mut Emitter,
    op: &UnOp,
    src: &Operand,
    dst: crate::ir::tac::TempId,
    frame: &Frame,
    strings: &StringPool,
    ty: &Type,
) -> EmitResult<()> {
    // `&x` precisa do *endereco* do slot de `src`, nao do seu valor: nao
    // passa por `load_op` (que faria `movq slot(%rbp), %reg`, carregando o
    // conteudo em vez do endereco).
    if matches!(op, UnOp::AddrOf) {
        let key = SlotKey::from_operand(src).ok_or_else(|| {
            codegen_error(
                "endereco-de (&) requer uma variavel ou temporario com slot",
                Some("unop"),
            )
        })?;
        let offset = frame
            .offset_of(&key)
            .expect("operando de & deve ter slot alocado no frame");
        em.insn(&format!("leaq {offset}(%rbp), %rax"));
        store_op(em, frame, &Operand::Temp(dst), "rax", strings)?;
        return Ok(());
    }
    if matches!(ty, Type::Double) {
        return Err(codegen_error(
            "Operacoes unarias ainda nao suportadas para double",
            Some("unop"),
        ));
    }

    load_op(em, frame, src, "rax", strings, ty)?;
    match op {
        UnOp::Neg => em.insn("negq %rax"),
        UnOp::BitNot => em.insn("notq %rax"),
        UnOp::Not => {
            em.insn("testq %rax, %rax");
            em.insn("sete %al");
            em.insn("movzbq %al, %rax");
        }
        UnOp::Deref => {
            em.insn("movq (%rax), %rax");
        }
        UnOp::AddrOf => unreachable!("tratado antes do load_op acima"),
    }
    store_op(em, frame, &Operand::Temp(dst), "rax", strings)?;
    Ok(())
}

fn emit_call(
    em: &mut Emitter,
    fn_name: &str,
    args: &[Operand],
    dst: Option<crate::ir::tac::TempId>,
    frame: &Frame,
    strings: &StringPool,
) -> EmitResult<()> {
    // Argumentos alem de `MAX_REG_ARGS` vao para a stack do chamador, na
    // ordem inversa (o primeiro arg de stack fica no topo, mais proximo do
    // endereco de retorno), espelhando `abi::stack_arg_offset`.
    let stack_arg_count = args.len().saturating_sub(abi::MAX_REG_ARGS);
    // Mantem a stack 16-alinhada no `call`: cada push usa 8 bytes, entao uma
    // quantidade impar de argumentos de stack precisa de 8 bytes de padding.
    let padding = if stack_arg_count % 2 == 1 { 8 } else { 0 };
    if padding > 0 {
        em.insn(&format!("subq ${padding}, %rsp"));
    }
    let stack_args = &args[args.len().min(abi::MAX_REG_ARGS)..];
    for arg in stack_args.iter().rev() {
        load_op(em, frame, arg, "rax", strings)?;
        em.insn("pushq %rax");
    }

    for (index, arg) in args.iter().take(abi::MAX_REG_ARGS).enumerate() {
        let reg = abi::arg_register(index).expect("index < MAX_REG_ARGS sempre tem registrador");
        load_op(em, frame, arg, "rax", strings, &Type::Int)?;
        em.insn(&format!("movq %rax, %{reg}"));
    }

    em.insn(&format!("call {fn_name}"));

    let cleanup = (stack_arg_count as i64) * 8 + padding;
    if cleanup > 0 {
        em.insn(&format!("addq ${cleanup}, %rsp"));
    }

    if let Some(dst) = dst {
        store_op(em, frame, &Operand::Temp(dst), "rax", strings, &Type::Int)?;
    }
    Ok(())
}

/// Carrega `op` para o registrador nomeado (ex.: "rax", "rcx").
fn load_op(
    em: &mut Emitter,
    frame: &Frame,
    op: &Operand,
    reg: &str,
    strings: &StringPool,
    ty: &Type,
) -> EmitResult<()> {
    let mov_insn = if matches!(ty, Type::Double) {"movsd"} else {"movq"};
    match op {
        Operand::Const(ConstValue::String(value)) => {
            let label = strings
                .labels
                .get(value)
                .expect("string literal deve ter sido coletada");
            em.insn(&format!("leaq {label}(%rip), %{reg}"));
            Ok(())
        }
        Operand::Const(ConstValue::Double(value)) =>{
            let label = strings
                .double_labels
                .get(&value.to_string())
                .expect("double literal deve ter sido coletado");

                em.insn(&format!("movsd {label}(%rip), %{reg}"));
                Ok(())
        }
        Operand::Const(value) => {
            em.insn(&format!("movq ${}, %{reg}", const_immediate(value)?));
            Ok(())
        }
        Operand::Temp(temp) => {
            let offset = frame
                .offset_of(&SlotKey::Temp(temp.0))
                .expect("temp sem slot alocado");
            em.insn(&format!("movq {offset}(%rbp), %{reg}"));
            Ok(())
        }
        Operand::Var(name) => {
            let offset = frame
                .offset_of(&SlotKey::Var(name.clone()))
                .expect("var sem slot alocado");
            em.insn(&format!("movq {offset}(%rbp), %{reg}"));
            Ok(())
        }
        Operand::Deref(inner) => {
            // `%r11` e scratch/caller-saved e nao e usado como `reg` por
            // nenhum chamador de `load_op`/`store_op` neste backend, entao e
            // seguro usa-lo aqui para materializar o ponteiro antes do deref.
            load_op(em, frame, inner, DEREF_SCRATCH_REG, strings)?;
            em.insn(&format!("movq (%{DEREF_SCRATCH_REG}), %{reg}"));
            Ok(())
        }
    }
}

/// Armazena o registrador nomeado em `op` (que deve ser temp, var ou deref).
fn store_op(
    em: &mut Emitter,
    frame: &Frame,
    op: &Operand,
    reg: &str,
    strings: &StringPool,
    ty: &Type,
) -> EmitResult<()> {
    let mov_insn = if matches!(ty, Type::Double) {"movsd"} else {"movq"};
    if let Operand::Deref(inner) = op {
        load_op(em, frame, inner, DEREF_SCRATCH_REG, strings, &Type::Int)?;
        em.insn(&format!("movq %{reg}, (%{DEREF_SCRATCH_REG})"));
        return Ok(());
    }

    let offset = match op {
        Operand::Temp(temp) => frame
            .offset_of(&SlotKey::Temp(temp.0))
            .expect("temp sem slot alocado"),
        Operand::Var(name) => frame
            .offset_of(&SlotKey::Var(name.clone()))
            .expect("var sem slot alocado"),
        Operand::Const(_) => {
            return Err(codegen_error(
                "nao e possivel armazenar em uma constante",
                Some("store"),
            ))
        }
        Operand::Deref(_) => unreachable!("tratado antes do match acima"),
    };
    em.insn(&format!("movq %{reg}, {offset}(%rbp)"));
    Ok(())
}

fn const_immediate(value: &ConstValue) -> EmitResult<String> {
    match value {
        ConstValue::Int(v) => Ok(v.to_string()),
        ConstValue::Char(c) => Ok((*c as i64).to_string()),
        ConstValue::Double(_) => Err(codegen_error(
            "codegen de double nao suportado neste backend",
            Some("const"),
        )),
        ConstValue::String(_) => unreachable!("string literals are emitted through rodata"),
    }
}

fn codegen_error(message: &str, instruction: Option<&str>) -> CodegenError {
    CodegenError {
        message: message.to_string(),
        instruction: instruction.map(str::to_string),
    }
}

fn escape_asm_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c.is_ascii_graphic() || c == ' ' => out.push(c),
            c => out.push_str(&format!("\\x{:02x}", c as u32)),
        }
    }
    out.push('"');
    out
}

fn local_label(func_name: &str, label: &LabelId) -> String {
    format!(".L_{func_name}_L{}", label.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::tac::{ConstValue, Operand, TacFunction, TacInstr, TempId};

    fn func(name: &str, params: Vec<&str>, instrs: Vec<TacInstr>) -> TacFunction {
        TacFunction {
            name: name.to_string(),
            params: params.into_iter().map(String::from).collect(),
            instrs,
            var_sizes: Default::default(),
        }
    }

    fn asm_simple_return_const() -> TacFunction {
        func(
            "main",
            Vec::new(),
            vec![TacInstr::Return {
                val: Some(Operand::Const(ConstValue::Int(42))),
            }],
        )
    }

    #[test]
    fn emit_function_prologue_pushes_rbp_and_sets_frame() {
        let out = emit_function(&asm_simple_return_const(), &StringPool::default()).unwrap();

        assert!(out.contains("pushq %rbp"));
        assert!(out.contains("movq %rsp, %rbp"));
        assert!(out.contains("ret"));
    }

    #[test]
    fn emit_function_declares_global_symbol() {
        let out = emit_function(&asm_simple_return_const(), &StringPool::default()).unwrap();

        assert!(out.contains(".globl main"));
        assert!(out.contains("main:\n"));
    }

    #[test]
    fn return_const_loads_immediate_into_rax() {
        let out = emit_function(&asm_simple_return_const(), &StringPool::default()).unwrap();

        assert!(out.contains("movq $42, %rax"));
    }

    #[test]
    fn binary_add_emits_load_add_store() {
        let f = func(
            "add",
            vec!["a", "b"],
            vec![
                TacInstr::BinOp {
                    dst: TempId(0),
                    op: BinOp::Add,
                    lhs: Operand::Var("a".to_string()),
                    rhs: Operand::Var("b".to_string()),
                },
                TacInstr::Return {
                    val: Some(Operand::Temp(TempId(0))),
                },
            ],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("movq %rdi, -8(%rbp)")); // spill arg a
        assert!(out.contains("movq %rsi, -16(%rbp)")); // spill arg b
        assert!(out.contains("addq %rcx, %rax"));
    }

    #[test]
    fn division_uses_cqto_and_idivq() {
        let f = func(
            "divmod",
            Vec::new(),
            vec![
                TacInstr::BinOp {
                    dst: TempId(0),
                    op: BinOp::Div,
                    lhs: Operand::Const(ConstValue::Int(10)),
                    rhs: Operand::Const(ConstValue::Int(3)),
                },
                TacInstr::Return {
                    val: Some(Operand::Temp(TempId(0))),
                },
            ],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("cqto"));
        assert!(out.contains("idivq %rcx"));
    }

    #[test]
    fn modulo_moves_rdx_into_rax() {
        let f = func(
            "mod",
            Vec::new(),
            vec![TacInstr::BinOp {
                dst: TempId(0),
                op: BinOp::Mod,
                lhs: Operand::Const(ConstValue::Int(10)),
                rhs: Operand::Const(ConstValue::Int(3)),
            }],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("movq %rdx, %rax"));
    }

    #[test]
    fn less_than_emits_setl() {
        let f = func(
            "less",
            Vec::new(),
            vec![TacInstr::BinOp {
                dst: TempId(0),
                op: BinOp::Less,
                lhs: Operand::Const(ConstValue::Int(1)),
                rhs: Operand::Const(ConstValue::Int(2)),
            }],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("cmpq %rcx, %rax"));
        assert!(out.contains("setl %al"));
        assert!(out.contains("movzbq %al, %rax"));
    }

    #[test]
    fn call_sets_up_arg_registers() {
        let f = func(
            "caller",
            Vec::new(),
            vec![
                TacInstr::Call {
                    dst: Some(TempId(0)),
                    fn_name: "soma".to_string(),
                    args: vec![
                        Operand::Const(ConstValue::Int(2)),
                        Operand::Const(ConstValue::Int(3)),
                    ],
                },
                TacInstr::Return {
                    val: Some(Operand::Temp(TempId(0))),
                },
            ],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("movq %rax, %rdi"));
        assert!(out.contains("movq %rax, %rsi"));
        assert!(out.contains("call soma"));
    }

    #[test]
    fn call_with_seven_args_pushes_one_stack_arg() {
        let args = (1..=7)
            .map(|n| Operand::Const(ConstValue::Int(n)))
            .collect();
        let f = func(
            "caller7",
            Vec::new(),
            vec![
                TacInstr::Call {
                    dst: Some(TempId(0)),
                    fn_name: "sum7".to_string(),
                    args,
                },
                TacInstr::Return {
                    val: Some(Operand::Temp(TempId(0))),
                },
            ],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("subq $8, %rsp"));
        assert!(out.contains("pushq %rax"));
        assert!(out.contains("call sum7"));
        assert!(out.contains("addq $16, %rsp"));
    }

    #[test]
    fn call_with_eight_args_needs_no_padding() {
        let args = (1..=8)
            .map(|n| Operand::Const(ConstValue::Int(n)))
            .collect();
        let f = func(
            "caller8",
            Vec::new(),
            vec![TacInstr::Call {
                dst: Some(TempId(0)),
                fn_name: "sum8".to_string(),
                args,
            }],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(!out.contains("subq $8, %rsp"));
        assert!(out.contains("addq $16, %rsp"));
    }

    #[test]
    fn call_with_two_args_emits_no_stack_cleanup() {
        let out = emit_function(
            &func(
                "caller2",
                Vec::new(),
                vec![TacInstr::Call {
                    dst: Some(TempId(0)),
                    fn_name: "soma".to_string(),
                    args: vec![
                        Operand::Const(ConstValue::Int(1)),
                        Operand::Const(ConstValue::Int(2)),
                    ],
                }],
            ),
            &StringPool::default(),
        )
        .unwrap();

        assert!(!out.contains("pushq %rax"));
        assert!(!out.contains("addq $16, %rsp"));
    }

    #[test]
    fn cond_jump_uses_test_and_jne() {
        let f = func(
            "cond",
            Vec::new(),
            vec![
                TacInstr::CondJump {
                    cond: Operand::Const(ConstValue::Int(1)),
                    then_label: LabelId(0),
                    else_label: LabelId(1),
                },
                TacInstr::Label(LabelId(0)),
                TacInstr::Return {
                    val: Some(Operand::Const(ConstValue::Int(1))),
                },
                TacInstr::Label(LabelId(1)),
                TacInstr::Return {
                    val: Some(Operand::Const(ConstValue::Int(0))),
                },
            ],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("testq %rax, %rax"));
        assert!(out.contains("jne .L_cond_L0"));
        assert!(out.contains("jmp .L_cond_L1"));
        assert!(out.contains(".L_cond_L0:"));
        assert!(out.contains(".L_cond_L1:"));
    }

    #[test]
    fn epilogue_label_is_emitted_once() {
        let out = emit_function(&asm_simple_return_const(), &StringPool::default()).unwrap();

        assert_eq!(out.matches(".L_main_epilogue:").count(), 1);
    }

    #[test]
    fn emit_program_prepends_text_section() {
        let prog = TacProgram {
            functions: vec![asm_simple_return_const()],
        };

        let out = emit_program(&prog).unwrap();

        assert!(out.starts_with(".text"));
        assert!(out.contains(".globl main"));
    }

    #[test]
    fn logical_and_normalizes_operands() {
        let f = func(
            "land",
            Vec::new(),
            vec![TacInstr::BinOp {
                dst: TempId(0),
                op: BinOp::And,
                lhs: Operand::Const(ConstValue::Int(1)),
                rhs: Operand::Const(ConstValue::Int(0)),
            }],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("setne %al"));
        assert!(out.contains("andq %rdx, %rax"));
    }

    #[test]
    fn logical_or_emits_orq() {
        let f = func(
            "lor",
            Vec::new(),
            vec![TacInstr::BinOp {
                dst: TempId(0),
                op: BinOp::Or,
                lhs: Operand::Const(ConstValue::Int(1)),
                rhs: Operand::Const(ConstValue::Int(0)),
            }],
        );

        let out = emit_function(&f, &StringPool::default()).unwrap();

        assert!(out.contains("orq %rdx, %rax"));
    }
}
