//! Auxiliares da convencao de chamada System V AMD64 ABI.
//!
//! Apenas a parte inteira do ABI esta coberta, que e o escopo do backend
//! atual (variaveis/temps de 64 bits). Pontos flutuantes (XMM) e vector args
//! ficam fora de escopo por enquanto.
//!
//! Resumo do System V AMD64 (inteiros):
//! - Argumentos inteiros: `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9` (depois stack)
//! - Retorno: `rax`
//! - caller-saved: `rax`, `rcx`, `rdx`, `rsi`, `rdi`, `r8`-`r11`
//! - callee-saved: `rbx`, `rbp`, `r12`-`r15`

/// Registradores (64 bits) usados para passar argumentos inteiros, em ordem.
pub const ARG_REGISTERS: [&str; 6] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];

/// Registrador de retorno de valores inteiros.
pub const RETURN_REGISTER: &str = "rax";

/// Quantidade maxima de argumentos passados em registrador (antes da stack).
pub const MAX_REG_ARGS: usize = ARG_REGISTERS.len();

/// Retorna o registrador de argumento para o indice dado, ou `None` se o
/// argumento deve ser passado pela stack (indice >= 6).
pub fn arg_register(index: usize) -> Option<&'static str> {
    ARG_REGISTERS.get(index).copied()
}

/// Offset positivo, a partir de `%rbp`, onde o enesimo argumento passado na
/// stack do chamador fica disponivel dentro da funcao chamada.
///
/// Para `index >= MAX_REG_ARGS`, o argumento chega em `16 + 8 * (index - 6)`
/// bytes acima de `%rbp` (acima do return address e do `%rbp` salvo).
pub fn stack_arg_offset(index: usize) -> i64 {
    debug_assert!(index >= MAX_REG_ARGS);
    16 + 8 * (index - MAX_REG_ARGS) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arg_registers_match_system_v_order() {
        assert_eq!(ARG_REGISTERS, ["rdi", "rsi", "rdx", "rcx", "r8", "r9"]);
    }

    #[test]
    fn arg_register_lookup() {
        assert_eq!(arg_register(0), Some("rdi"));
        assert_eq!(arg_register(5), Some("r9"));
        assert_eq!(arg_register(6), None);
    }

    #[test]
    fn stack_arg_offsets_skip_saved_rbp_and_return_addr() {
        // 16 = return address (8) + saved %rbp (8).
        assert_eq!(stack_arg_offset(6), 16);
        assert_eq!(stack_arg_offset(7), 24);
        assert_eq!(stack_arg_offset(10), 48);
    }
}
