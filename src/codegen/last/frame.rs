//! Gerenciamento do stack frame de uma funcao para o backend x86-64.
//!
//! O backend atual usa uma estrategia ingenua porem correta: cada temp e
//! variavel da TAC recebe um slot proprio de 8 bytes na stack. Nao ha
//! alocacao de registradores (spill universal), o que simplifica a selecao de
//! instrucoes e mantem a correcao enquanto o alocador (`reg_alloc`) nao esta
//! integrado ao pipeline de TAC.
//!
//! Layout do frame (referenciado a partir de `%rbp`):
//!
//! ```text
//!   alto  +-----------+
//!         | arg stack |  argumentos do chamador (acima de rbp)
//!         | ret addr  |
//!         | saved rbp | <- %rbp
//!    -8   | slot 0    |   primeira var/temp local
//!   -16   | slot 1    |
//!         | ...       |
//!   baixo +-----------+ <- %rsp  (apos `subq $frame_size, %rsp`)
//! ```
//!
//! O tamanho do frame e sempre alinhado em 16 bytes para manter o alinhamento
//! exigido pelo System V AMD64 ABI no ponto de `call`.

use std::collections::HashMap;

use crate::ir::tac::Operand;

/// Chave que identifica unicamente um valor residente no frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SlotKey {
    Temp(u32),
    Var(String),
}

impl SlotKey {
    /// Mapeia um `Operand` para sua chave de slot. Constantes nao tem slot
    /// (sao emitidas como imediato) e retornam `None`. `Deref` nao tem slot
    /// proprio: o slot relevante e o do ponteiro que ele indireciona.
    pub fn from_operand(op: &Operand) -> Option<Self> {
        match op {
            Operand::Temp(temp) => Some(Self::Temp(temp.0)),
            Operand::Var(name) => Some(Self::Var(name.clone())),
            Operand::Const(_) => None,
            Operand::Deref(inner) => Self::from_operand(inner),
        }
    }
}

/// Stack frame de uma unica funcao.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Mapeia cada chave de slot para seu offset relativo a `%rbp`.
    offsets: HashMap<SlotKey, i64>,
    /// Proximo offset negativo a ser usado para um novo slot local.
    next_local_offset: i64,
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

impl Frame {
    /// Cria um frame vazio. O primeiro slot local alocado comeca em `-8`
    /// (ou em `-tamanho`, arredondado para 8, se maior que 8 bytes).
    pub fn new() -> Self {
        Self {
            offsets: HashMap::new(),
            next_local_offset: 0,
        }
    }

    /// Aloca (se ainda nao existir) e retorna o offset de `key`, reservando
    /// exatamente um slot de 8 bytes. Equivalente a
    /// `allocate_local_sized(key, 8)` — usado por temporarios e variaveis
    /// escalares/ponteiro, que sempre cabem em 8 bytes neste backend.
    pub fn allocate_local(&mut self, key: SlotKey) -> i64 {
        self.allocate_local_sized(key, 8)
    }

    /// Aloca (se ainda nao existir) um bloco contiguo de `size` bytes
    /// (arredondado para multiplo de 8) e retorna o offset do seu primeiro
    /// byte (o endereco mais baixo do bloco). Usado para variaveis cujo
    /// valor nao cabe em 8 bytes, como structs: campos sao acessados como
    /// `offset_of(var) + offset_do_campo`.
    ///
    /// Aloca sempre decrementando primeiro o cursor de offset, depois
    /// atribuindo: isso preserva a sequencia historica de offsets
    /// (-8, -16, -24, ...) para alocacoes de 8 bytes, e generaliza de forma
    /// continua para blocos maiores.
    pub fn allocate_local_sized(&mut self, key: SlotKey, size: i64) -> i64 {
        if let Some(&offset) = self.offsets.get(&key) {
            return offset;
        }
        let aligned_size = align_up(size.max(1), 8);
        self.next_local_offset -= aligned_size;
        let offset = self.next_local_offset;
        self.offsets.insert(key, offset);
        offset
    }

    /// Fixa um offset explicito para `key` (usado para argumentos recebidos
    /// via stack do chamador, que vivem em offsets positivos).
    pub fn set_offset(&mut self, key: SlotKey, offset: i64) {
        self.offsets.insert(key, offset);
    }

    /// Retorna o offset de `key`, se existir.
    pub fn offset_of(&self, key: &SlotKey) -> Option<i64> {
        self.offsets.get(key).copied()
    }

    /// Numero de slots locais alocados (offsets negativos).
    pub fn local_slot_count(&self) -> usize {
        self.offsets.values().filter(|&&off| off < 0).count()
    }

    /// Tamanho total do frame em bytes, alinhado em 16.
    ///
    /// Retorna 0 quando nao ha slots locais (funcao "leaf" sem frame).
    /// Calculado a partir do cursor de alocacao (`next_local_offset`), e
    /// nao de `local_slot_count() * 8`: blocos maiores que 8 bytes (ex.:
    /// structs via `allocate_local_sized`) ocupam mais de um "slot" de
    /// contabilidade, mas devem ser contados pelo numero real de bytes.
    pub fn frame_size(&self) -> i64 {
        align_up(-self.next_local_offset, 16)
    }
}

fn align_up(value: i64, alignment: i64) -> i64 {
    debug_assert!(alignment > 0);
    (value + alignment - 1) / alignment * alignment
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::tac::TempId;

    #[test]
    fn allocate_local_assigns_distinct_negative_offsets() {
        let mut frame = Frame::new();
        let a = frame.allocate_local(SlotKey::Var("x".to_string()));
        let b = frame.allocate_local(SlotKey::Temp(TempId(0).0));

        assert_eq!(a, -8);
        assert_eq!(b, -16);
    }

    #[test]
    fn allocate_local_is_idempotent() {
        let mut frame = Frame::new();
        let key = SlotKey::Var("x".to_string());

        let first = frame.allocate_local(key.clone());
        let second = frame.allocate_local(key);

        assert_eq!(first, second);
        assert_eq!(frame.local_slot_count(), 1);
    }

    #[test]
    fn set_offset_for_stack_argument_is_positive() {
        let mut frame = Frame::new();
        frame.set_offset(SlotKey::Var("extra".to_string()), 16);

        assert_eq!(
            frame.offset_of(&SlotKey::Var("extra".to_string())),
            Some(16)
        );
    }

    #[test]
    fn frame_size_aligned_to_16() {
        let mut frame = Frame::new();
        // 1 slot -> 8 bytes crus -> alinhado para 16.
        frame.allocate_local(SlotKey::Var("a".to_string()));
        assert_eq!(frame.frame_size(), 16);

        // 2 slots -> 16 bytes.
        frame.allocate_local(SlotKey::Var("b".to_string()));
        assert_eq!(frame.frame_size(), 16);

        // 3 slots -> 24 bytes -> alinhado para 32.
        frame.allocate_local(SlotKey::Var("c".to_string()));
        assert_eq!(frame.frame_size(), 32);
    }

    #[test]
    fn empty_frame_has_zero_size() {
        let frame = Frame::new();
        assert_eq!(frame.frame_size(), 0);
        assert_eq!(frame.local_slot_count(), 0);
    }

    #[test]
    fn slot_key_from_operand_skips_constants() {
        assert_eq!(
            SlotKey::from_operand(&Operand::Temp(TempId(3))),
            Some(SlotKey::Temp(3))
        );
        assert_eq!(
            SlotKey::from_operand(&Operand::Var("y".to_string())),
            Some(SlotKey::Var("y".to_string()))
        );
        assert_eq!(
            SlotKey::from_operand(&Operand::Const(crate::ir::tac::ConstValue::Int(7))),
            None
        );
    }
}
