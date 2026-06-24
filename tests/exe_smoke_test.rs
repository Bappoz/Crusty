//! Smoke tests ponta-a-ponta da issue #130: fonte C real passa por todo o
//! pipeline (lexer -> parser -> semantic -> IR -> codegen x86-64) e o
//! assembly resultante e montado/linkado com `gcc` em um executavel ELF
//! real, que e entao executado para checar o exit code.
//!
//! Diferente de `tests/codegen_smoke.rs` (que monta `TacProgram` a mao),
//! aqui o ponto de entrada e codigo-fonte C, exercitando o front-end
//! completo. Se `gcc` nao estiver disponivel no ambiente, os testes sao
//! ignorados (skip) em vez de falhar.

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
/// se qualquer estagio reportar diagnosticos, ja que os fixtures usados
/// aqui sao sempre programas C validos.
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
    asm_path.push(format!("crusty_exe_smoke_{name}_{}.s", std::process::id()));
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

#[test]
fn smoke_minimal_program_exit_0() {
    require_gcc!();

    let status = compile_and_run("exit0", "int main() { return 0; }");

    #[cfg(unix)]
    assert_eq!(status.code(), Some(0));
}

#[test]
fn smoke_minimal_program_exit_42() {
    require_gcc!();

    let status = compile_and_run("exit42", "int main() { return 42; }");

    #[cfg(unix)]
    assert_eq!(status.code(), Some(42));
}

#[test]
fn smoke_local_variable_arithmetic_runs() {
    require_gcc!();

    let status = compile_and_run(
        "arith",
        "int main() { int a = 10; int b = 32; return a + b; }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(42));
}

#[test]
fn smoke_function_call_runs() {
    require_gcc!();

    let status = compile_and_run(
        "call",
        "int soma(int a, int b) { return a + b; } int main() { return soma(19, 23); }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(42));
}

#[test]
fn smoke_blocks_and_local_scopes_run() {
    require_gcc!();

    let status = compile_and_run(
        "blocks",
        "int main() { \
            int total = 0; \
            { int a = 5; total = total + a; } \
            { int b = 7; total = total + b; } \
            return total; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(12));
}

#[test]
fn smoke_if_else_chain_runs() {
    require_gcc!();

    let status = compile_and_run(
        "if_else_chain",
        "int classify(int n) { \
            if (n < 0) { return -1; } \
            else if (n == 0) { return 0; } \
            else { return 1; } \
        } \
        int main() { return classify(-5) + classify(0) + classify(5) + 10; }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(10));
}

#[test]
fn smoke_while_loop_runs() {
    require_gcc!();

    let status = compile_and_run(
        "while",
        "int main() { \
            int i = 0; \
            int total = 0; \
            while (i < 5) { total = total + i; i = i + 1; } \
            return total; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(10));
}

#[test]
fn smoke_for_loop_runs() {
    require_gcc!();

    let status = compile_and_run(
        "for",
        "int main() { \
            int total = 0; \
            for (int i = 0; i < 5; i = i + 1) { total = total + i; } \
            return total; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(10));
}

#[test]
fn smoke_do_while_loop_runs() {
    require_gcc!();

    let status = compile_and_run(
        "do_while",
        "int main() { \
            int i = 0; \
            int total = 0; \
            do { total = total + i; i = i + 1; } while (i < 5); \
            return total; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(10));
}

#[test]
fn smoke_recursive_fibonacci_runs() {
    require_gcc!();

    let status = compile_and_run(
        "fib",
        "int fib(int n) { \
            if (n <= 1) { return n; } \
            return fib(n - 1) + fib(n - 2); \
        } \
        int main() { return fib(10); }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(55));
}

#[test]
fn smoke_switch_with_default_runs() {
    require_gcc!();

    let status = compile_and_run(
        "switch_default",
        "int classify(int n) { \
            int result = 0; \
            switch (n) { \
                case 1: result = 1; break; \
                case 2: result = 2; break; \
                default: result = -1; break; \
            } \
            return result; \
        } \
        int main() { return classify(1) + classify(2) + classify(9) + 100; }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(102));
}

#[test]
fn smoke_address_of_and_deref_read_runs() {
    require_gcc!();

    let status = compile_and_run(
        "addrof_deref_read",
        "int main() { \
            int x = 21; \
            int *p = &x; \
            int y = *p; \
            return y * 2; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(42));
}

#[test]
fn smoke_deref_assignment_writes_through_pointer() {
    require_gcc!();

    let status = compile_and_run(
        "deref_assign",
        "int main() { \
            int x = 10; \
            int *p = &x; \
            *p = 20; \
            return x; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(20));
}

#[test]
fn smoke_deref_compound_assign_through_function_param_runs() {
    require_gcc!();

    let status = compile_and_run(
        "deref_compound",
        "void inc(int *p) { *p = *p + 1; } \
        int main() { \
            int x = 10; \
            inc(&x); \
            inc(&x); \
            return x; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(12));
}

#[test]
fn smoke_deref_increment_operators_run() {
    require_gcc!();

    let status = compile_and_run(
        "deref_incr",
        "int main() { \
            int x = 5; \
            int *p = &x; \
            (*p)++; \
            *p += 10; \
            return x; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(16));
}

#[test]
fn smoke_pointer_index_read_and_write_runs() {
    require_gcc!();

    let status = compile_and_run(
        "pointer_index",
        "int sum_via_index(int *p) { \
            p[0] = 10; \
            return p[0] + 5; \
        } \
        int main() { \
            int x = 1; \
            return sum_via_index(&x); \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(15));
}

#[test]
fn smoke_fixed_array_index_read_and_write_runs() {
    require_gcc!();

    let status = compile_and_run(
        "fixed_array_index",
        "int main() { \
            int arr[3]; \
            arr[0] = 1; \
            arr[1] = 2; \
            arr[2] = 3; \
            return arr[0] + arr[1] + arr[2]; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(6));
}

#[test]
fn smoke_struct_member_read_and_write_runs() {
    require_gcc!();

    let status = compile_and_run(
        "struct_member",
        "struct Point { int x; int y; }; \
        int main() { \
            struct Point p; \
            p.x = 3; \
            p.y = 4; \
            return p.x + p.y; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(7));
}

#[test]
fn smoke_struct_member_via_pointer_arrow_runs() {
    require_gcc!();

    let status = compile_and_run(
        "struct_member_arrow",
        "struct Point { int x; int y; }; \
        void move_point(struct Point *p, int dx, int dy) { \
            p->x = p->x + dx; \
            p->y = p->y + dy; \
        } \
        int main() { \
            struct Point p; \
            p.x = 1; \
            p.y = 2; \
            move_point(&p, 10, 20); \
            return p.x + p.y; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(33));
}

#[test]
fn smoke_struct_larger_than_eight_bytes_runs() {
    require_gcc!();

    let status = compile_and_run(
        "struct_big",
        "struct Big { long a; long b; long c; }; \
        int main() { \
            struct Big big; \
            big.a = 1; \
            big.b = 2; \
            big.c = 3; \
            return big.a + big.b + big.c; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(6));
}

#[test]
fn smoke_sizeof_of_variables_runs() {
    require_gcc!();

    let status = compile_and_run(
        "sizeof_vars",
        "int main() { \
            int x = 7; \
            long l = 3; \
            char c = 'a'; \
            int *p = &x; \
            return sizeof(x) + sizeof(l) + sizeof(c) + sizeof(p); \
        }",
    );

    // sizeof(int) + sizeof(long) + sizeof(char) + sizeof(int*) = 4+8+1+8.
    #[cfg(unix)]
    assert_eq!(status.code(), Some(21));
}

#[test]
fn smoke_switch_fallthrough_runs() {
    require_gcc!();

    let status = compile_and_run(
        "switch_fallthrough",
        "int main() { \
            int n = 2; \
            int total = 0; \
            switch (n) { \
                case 1: total = total + 1; \
                case 2: total = total + 2; \
                case 3: total = total + 3; break; \
                case 4: total = total + 100; \
            } \
            return total; \
        }",
    );

    #[cfg(unix)]
    assert_eq!(status.code(), Some(5));
}
