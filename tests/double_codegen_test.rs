//! Smoke tests ponta-a-ponta para o codegen x86-64 de `double` (issue #172).
//!
//! Escopo coberto, conforme delimitado na issue: literais `double`,
//! variaveis locais, aritmetica basica (`+ - * /`), comparacoes e `return`.
//! Argumentos/parametros e retorno de `double` por uma funcao chamada pelo
//! proprio codegen (em vez de por um `main` escrito a mao) permanecem fora
//! de escopo, pois exigiriam estender `codegen/last/abi.rs` para cobrir
//! `xmm0..xmm7` na convencao de chamada — deixado para uma proxima etapa.
//!
//! Como nem toda expressao com `double` pode ser verificada via exit code
//! (o valor de retorno de um processo e sempre truncado a um inteiro de 8
//! bits), os testes usam duas estrategias:
//! - quando o resultado observavel e naturalmente inteiro (comparacoes, que
//!   este backend ja normaliza para 0/1 em `%rax`), o programa inteiro e
//!   gerado por este compilador e o exit code e verificado diretamente;
//! - quando o resultado e um `double` em si (ex.: o proprio criterio de
//!   aceite da issue, `double x = 1.5; return x + 2.5;`), apenas a funcao
//!   `double` e gerada por este compilador (`--emit=obj`); um pequeno
//!   programa C convencional, compilado pelo `gcc` do sistema, chama essa
//!   funcao e verifica o resultado — exercitando o lado "callee" da ABI
//!   (retorno em `%xmm0`) sem depender do lado "caller" deste backend.
//!
//! Se `gcc` nao estiver disponivel no ambiente, os testes sao ignorados
//! (skip) em vez de falhar, espelhando `tests/exe_smoke_test.rs`.

#![cfg_attr(not(unix), allow(unused_variables))]

use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use crusty::analyser::analyse_with_builtins;
use crusty::codegen::last::emit_program;
use crusty::common::input::source::SourceFile;
use crusty::ir::lower::lower_program;
use crusty::lexer::scanner::Scanner;
use crusty::parser::Parser;

fn gcc_available() -> bool {
    Command::new("gcc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Ignora o teste quando nao ha `gcc` no ambiente.
macro_rules! require_gcc {
    () => {
        if !gcc_available() {
            eprintln!("gcc indisponivel: pulando teste de smoke");
            return;
        }
    };
}

/// Roda o pipeline completo (lexer -> parser -> semantic -> IR -> codegen)
/// sobre `source` e retorna o assembly x86-64 gerado. Falha o teste (panic)
/// se qualquer estagio reportar diagnosticos, ja que os fixtures usados aqui
/// sao sempre programas C validos.
fn compile_to_asm(source: &str) -> String {
    let mut scanner = Scanner::new(SourceFile::from_string(source));
    scanner.scan();
    assert!(
        scanner.diagnostics.is_empty(),
        "erros de lexer inesperados: {:?}",
        scanner.diagnostics
    );

    let mut parser = Parser::new(scanner.tokens);
    let program = parser
        .parse_program()
        .unwrap_or_else(|errors| panic!("erros de parser inesperados: {errors:?}"));

    let sem_diagnostics = analyse_with_builtins(&program, scanner.builtins);
    let sem_errors: Vec<_> = sem_diagnostics.iter().filter(|d| d.is_error()).collect();
    assert!(
        sem_errors.is_empty(),
        "erros semanticos inesperados: {sem_errors:?}"
    );

    let tac_program = lower_program(&program).unwrap();
    emit_program(&tac_program).unwrap()
}

/// Compila `source` (C) ate um executavel real via `gcc` e o executa,
/// retornando o `ExitStatus` do processo filho. Limpa os arquivos
/// temporarios (.s e binario) ao final.
fn compile_and_run(name: &str, source: &str) -> ExitStatus {
    let asm = compile_to_asm(source);

    let mut asm_path = std::env::temp_dir();
    asm_path.push(format!(
        "crusty_double_smoke_{name}_{}.s",
        std::process::id()
    ));
    std::fs::write(&asm_path, asm).expect("falha ao escrever .s temporario");
    let exe_path: PathBuf = asm_path.with_extension("bin");

    let link = Command::new("gcc")
        .arg(&asm_path)
        .arg("-o")
        .arg(&exe_path)
        .status()
        .expect("falha ao invocar gcc");
    assert!(
        link.success(),
        "gcc nao conseguiu linkar a saida do codegen"
    );

    let status = Command::new(&exe_path)
        .status()
        .expect("falha ao executar o binario gerado");

    let _ = std::fs::remove_file(&asm_path);
    let _ = std::fs::remove_file(&exe_path);

    status
}

/// Compila uma funcao `double` isolada (`source`) com `--emit=obj`-equivalente
/// (aqui via `emit_program`, montado a `.o` pelo `gcc`), e linka com um
/// pequeno harness C escrito a mao que chama `compute()` e verifica o
/// resultado. Retorna o `ExitStatus` do harness.
fn compile_double_fn_and_check(name: &str, source: &str, harness_body: &str) -> ExitStatus {
    let asm = compile_to_asm(source);

    let dir = std::env::temp_dir();
    let asm_path = dir.join(format!("crusty_double_fn_{name}_{}.s", std::process::id()));
    let obj_path = dir.join(format!("crusty_double_fn_{name}_{}.o", std::process::id()));
    let harness_path = dir.join(format!(
        "crusty_double_harness_{name}_{}.c",
        std::process::id()
    ));
    let exe_path = dir.join(format!(
        "crusty_double_fn_{name}_{}.bin",
        std::process::id()
    ));

    std::fs::write(&asm_path, asm).expect("falha ao escrever .s temporario");

    let assemble = Command::new("gcc")
        .arg("-c")
        .arg(&asm_path)
        .arg("-o")
        .arg(&obj_path)
        .status()
        .expect("falha ao invocar gcc -c");
    assert!(
        assemble.success(),
        "gcc nao conseguiu montar a saida do codegen"
    );

    std::fs::write(&harness_path, harness_body).expect("falha ao escrever harness .c");

    let link = Command::new("gcc")
        .arg(&harness_path)
        .arg(&obj_path)
        .arg("-o")
        .arg(&exe_path)
        .status()
        .expect("falha ao invocar gcc para linkar harness + objeto");
    assert!(
        link.success(),
        "gcc nao conseguiu linkar o harness com o objeto gerado pelo codegen"
    );

    let status = Command::new(&exe_path)
        .status()
        .expect("falha ao executar o binario gerado");

    let _ = std::fs::remove_file(&asm_path);
    let _ = std::fs::remove_file(&obj_path);
    let _ = std::fs::remove_file(&harness_path);
    let _ = std::fs::remove_file(&exe_path);

    status
}

/// Criterio de aceite da issue #172: `double x = 1.5; return x + 2.5;`
/// compila e roda via gcc com resultado correto (4.0).
#[test]
fn smoke_double_literal_local_and_addition_returns_correct_value() {
    require_gcc!();

    let status = compile_double_fn_and_check(
        "literal_add",
        "double compute() { double x = 1.5; return x + 2.5; }",
        r#"
        double compute(void);
        int main() {
            double r = compute();
            return (r == 4.0) ? 0 : 1;
        }
        "#,
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(0));
}

#[test]
fn smoke_double_subtraction_multiplication_division_return_correct_values() {
    require_gcc!();

    let status = compile_double_fn_and_check(
        "arith",
        "double compute() { \
            double a = 10.0; \
            double b = 4.0; \
            double sub = a - b; \
            double mul = sub * 2.0; \
            double div = mul / 3.0; \
            return div; \
        }",
        r#"
        double compute(void);
        int main() {
            double r = compute();
            return (r == 4.0) ? 0 : 1;
        }
        "#,
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(0));
}

#[test]
fn smoke_double_multiple_locals_runs() {
    require_gcc!();

    let status = compile_double_fn_and_check(
        "multi_locals",
        "double compute() { \
            double a = 1.5; \
            double b = 2.25; \
            double c = 0.25; \
            return a + b + c; \
        }",
        r#"
        double compute(void);
        int main() {
            double r = compute();
            return (r == 4.0) ? 0 : 1;
        }
        "#,
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(0));
}

/// Comparacoes entre `double` ja produzem um resultado inteiro (0/1) neste
/// backend, entao podem ser verificadas direto pelo exit code de um `int
/// main()` gerado integralmente por este compilador, sem depender de
/// conversao double<->int (ainda nao suportada).
#[test]
fn smoke_double_less_than_comparison_runs() {
    require_gcc!();

    let status = compile_and_run(
        "double_less_than",
        "int main() { \
            double a = 1.5; \
            double b = 2.5; \
            if (a < b) { return 1; } \
            return 0; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(1));
}

#[test]
fn smoke_double_equality_after_addition_runs() {
    require_gcc!();

    let status = compile_and_run(
        "double_eq_after_add",
        "int main() { \
            double a = 1.5; \
            double b = 2.5; \
            double c = a + b; \
            if (c == 4.0) { return 1; } \
            return 0; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(1));
}

#[test]
fn smoke_double_greater_or_equal_false_branch_runs() {
    require_gcc!();

    let status = compile_and_run(
        "double_geq_false",
        "int main() { \
            double a = 1.0; \
            double b = 9.0; \
            if (a >= b) { return 1; } \
            return 0; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(0));
}

/// Garante que o assembly gerado para `double` realmente usa o caminho de
/// ponto flutuante (registradores `xmm`/instrucoes `sd`) em vez do caminho
/// inteiro (`rax`/`rcx`), o que a issue #172 identificou como o gap real do
/// backend antes desta feature.
#[test]
fn double_codegen_emits_xmm_instructions() {
    let asm = compile_to_asm("double compute() { double x = 1.5; return x + 2.5; }");

    assert!(
        asm.contains("movsd"),
        "esperado mov de double via movsd no assembly gerado:\n{asm}"
    );
    assert!(
        asm.contains("addsd"),
        "esperada soma de double via addsd no assembly gerado:\n{asm}"
    );
    assert!(
        asm.contains(".double"),
        "esperada diretiva .double para os literais double na .rodata:\n{asm}"
    );
}
