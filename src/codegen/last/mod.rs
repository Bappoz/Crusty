//! Backend final (last): traducao da IR (TAC) para codigo de maquina alvo.
//!
//! Atualmente o unico alvo implementado e x86-64 (sintaxe AT&T / System V
//! AMD64 ABI). Os modulos seguintes organizam as responsabilidades:
//!
//! - [`abi`]: convencao de chamada System V AMD64.
//! - [`frame`]: gerenciamento do stack frame de cada funcao.
//! - [`x86_64`]: selecao de instrucoes e emissao de assembly.

pub mod abi;
pub mod frame;
pub mod peephole;
pub mod x86_64;

pub use x86_64::emit_program;
