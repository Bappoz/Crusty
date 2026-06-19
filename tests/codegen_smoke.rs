//! Testes de smoke do backend x86-64.
//!
//! Estes testes montam e (quando possivel) executam a saida do codegen com o
//! `gcc` do sistema para garantir que o assembly gerado nao apenas e
//! sintaticamente valido, mas tambem produz o resultado esperado em tempo de
//! execucao. Se o `gcc` nao estiver disponivel no ambiente, os testes sao
//! ignorados (skip) em vez de falhar.

use std::path::PathBuf;
use std::process::Command;

use crusty::codegen::last::emit_program;
use crusty::common::ast::expr::BinOp;
use crusty::ir::tac::{ConstValue, LabelId, Operand, TacFunction, TacInstr, TacProgram, TempId};

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

fn build_soma_program() -> TacProgram {
    let soma = TacFunction {
        name: "soma".to_string(),
        params: vec!["a".to_string(), "b".to_string()],
        instrs: vec![
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
    };

    let main = TacFunction {
        name: "main".to_string(),
        params: Vec::new(),
        instrs: vec![
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
    };

    TacProgram {
        functions: vec![soma, main],
    }
}

fn write_temp_source(name: &str, contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("crusty_smoke_{name}_{}.s", std::process::id()));
    std::fs::write(&path, contents).expect("falha ao escrever arquivo .s temporario");
    path
}

#[test]
fn smoke_assembles_with_gcc() {
    require_gcc!();

    let asm = emit_program(&build_soma_program());
    let source = write_temp_source("assemble", &asm);
    let object = source.with_extension("o");

    let status = Command::new("gcc")
        .args(["-c", "-x", "assembler-with-cpp"])
        .arg(&source)
        .arg("-o")
        .arg(&object)
        .status()
        .expect("falha ao invocar gcc");

    assert!(
        status.success(),
        "gcc nao conseguiu montar a saida do codegen"
    );

    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&object);
}

#[test]
fn smoke_links_and_runs_with_expected_exit_code() {
    require_gcc!();

    let asm = emit_program(&build_soma_program());
    let source = write_temp_source("run", &asm);
    let exe = source.with_extension("bin");

    let link = Command::new("gcc")
        .arg(&source)
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("falha ao invocar gcc");
    assert!(
        link.success(),
        "gcc nao conseguiu linkar a saida do codegen"
    );

    let exit = Command::new(&exe)
        .status()
        .expect("falha ao executar o binario gerado");

    // soma(2, 3) == 5: o valor de retorno de `main` vira o exit code.
    #[cfg(unix)]
    assert_eq!(exit.code(), Some(5), "esperava exit code 5 (soma de 2 + 3)");

    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&exe);
}

#[test]
fn smoke_simple_return_const_runs() {
    require_gcc!();

    let prog = TacProgram {
        functions: vec![TacFunction {
            name: "main".to_string(),
            params: Vec::new(),
            instrs: vec![TacInstr::Return {
                val: Some(Operand::Const(ConstValue::Int(42))),
            }],
        }],
    };

    let asm = emit_program(&prog);
    let source = write_temp_source("const", &asm);
    let exe = source.with_extension("bin");

    let link = Command::new("gcc")
        .arg(&source)
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("falha ao invocar gcc");
    assert!(link.success());

    let exit = Command::new(&exe).status().expect("falha ao executar");

    #[cfg(unix)]
    assert_eq!(exit.code(), Some(42));

    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&exe);
}

#[test]
fn smoke_control_flow_if_else_runs() {
    require_gcc!();

    // main: if (1) return 10; else return 20;  -> espera-se 10.
    let prog = TacProgram {
        functions: vec![TacFunction {
            name: "main".to_string(),
            params: Vec::new(),
            instrs: vec![
                TacInstr::CondJump {
                    cond: Operand::Const(ConstValue::Int(1)),
                    then_label: LabelId(0),
                    else_label: LabelId(1),
                },
                TacInstr::Label(LabelId(0)),
                TacInstr::Return {
                    val: Some(Operand::Const(ConstValue::Int(10))),
                },
                TacInstr::Label(LabelId(1)),
                TacInstr::Return {
                    val: Some(Operand::Const(ConstValue::Int(20))),
                },
            ],
        }],
    };

    let asm = emit_program(&prog);
    let source = write_temp_source("ifelse", &asm);
    let exe = source.with_extension("bin");

    let link = Command::new("gcc")
        .arg(&source)
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("falha ao invocar gcc");
    assert!(link.success());

    let exit = Command::new(&exe).status().expect("falha ao executar");

    #[cfg(unix)]
    assert_eq!(exit.code(), Some(10));

    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&exe);
}
