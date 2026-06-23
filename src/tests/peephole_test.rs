use crate::codegen::last::peephole::PeepholePass;

// Função auxiliar para rodar o Peephole rápido nos testes
fn run_peephole(instrs: Vec<&str>) -> Vec<String> {
    let mut asm: Vec<String> = instrs.into_iter().map(|s| s.to_string()).collect();
    let pass = PeepholePass::new();
    pass.run(&mut asm);
    asm
}

#[test]
fn test_remove_add_sub_zero() {
    // Padrão 3: Soma ou subtração por zero deve sumir
    let asm = run_peephole(vec![
        "addq $0, %rax",
        "subq $0, %rcx",
        "movq %rax, %rbx"
    ]);
    assert_eq!(asm, vec!["movq %rax, %rbx"]);
}

#[test]
fn test_remove_redundant_mov() {
    // Padrões 1 e 2: Mov de A pra B seguido de B pra A
    let asm = run_peephole(vec![
        "movq %rax, %rbx",
        "movq %rbx, %rax", // Esse tem que sumir
        "ret"
    ]);
    assert_eq!(asm, vec!["movq %rax, %rbx", "ret"]);
}

#[test]
fn test_remove_jump_to_next_line() {
    // Padrão 5: Pulo para a linha imediatamente abaixo
    let asm = run_peephole(vec![
        "jmp .L_main_L1",
        ".L_main_L1:",
        "ret"
    ]);
    assert_eq!(asm, vec![".L_main_L1:", "ret"]);
}

#[test]
fn test_optimize_mul_power_of_two() {
    // Padrão 4: Multiplicação por 8 (2^3) deve virar shlq $3
    let asm = run_peephole(vec![
        "movq $8, %rcx",
        "imulq %rcx, %rax"
    ]);
    assert_eq!(asm, vec!["    shlq $3, %rax"]);
}

#[test]
fn test_optimize_cmp_zero() {
    // Padrão 6: Comparar com 0 deve virar testq
    let asm = run_peephole(vec![
        "movq $0, %rcx",
        "cmpq %rcx, %rax"
    ]);
    assert_eq!(asm, vec!["    testq %rax, %rax"]);
}