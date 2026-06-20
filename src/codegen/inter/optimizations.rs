use std::collections::{HashMap, HashSet};

use crate::{
    common::ast::expr::{BinOp, UnOp},
    ir::tac::{ConstValue, Operand, TacInstr, TempId},
};

// ─── Liveness ────────────────────────────────────────────────────────────────

/// Resultado da análise de vida útil intraprocessual.
///
/// `live_before[i]` contém o conjunto de `TempId` que estão vivos
/// *imediatamente antes* da instrução de índice `i`.
pub struct LivenessInfo {
    pub live_before: Vec<HashSet<TempId>>,
}

/// Calcula `LivenessInfo` para uma sequência plana de instruções TAC.
///
/// Algoritmo: varredura *backward* (de trás para frente).
/// ```text
/// live = {}
/// para i de (n-1) até 0:
///     live_before[i] = live.clone()
///     remover de live: TempId *definido* por instrs[i]
///     adicionar a live: TempId *usados* como operandos de instrs[i]
/// ```
pub fn compute_liveness(instrs: &[TacInstr]) -> LivenessInfo {
    let n = instrs.len();
    let mut live_before = vec![HashSet::new(); n];
    let mut live: HashSet<TempId> = HashSet::new();

    for i in (0..n).rev() {
        live_before[i] = live.clone();

        // Remover o temporário definido pela instrução.
        if let Some(def) = instr_def(&instrs[i]) {
            live.remove(&def);
        }

        // Adicionar todos os temporários usados como operandos.
        for used in instr_uses(&instrs[i]) {
            live.insert(used);
        }
    }

    LivenessInfo { live_before }
}

/// Retorna o `TempId` definido pela instrução, se houver.
///
/// `Operand::Var` **nunca** é retornado aqui — variáveis nomeadas do programa C
/// têm semântica observável e nunca são candidatas à eliminação pelo DCE.
fn instr_def(instr: &TacInstr) -> Option<TempId> {
    match instr {
        TacInstr::BinOp { dst, .. } => Some(*dst),
        TacInstr::UnOp { dst, .. } => Some(*dst),
        TacInstr::Copy {
            dst: Operand::Temp(t),
            ..
        } => Some(*t),
        TacInstr::Call { dst: Some(t), .. } => Some(*t),
        _ => None,
    }
}

/// Retorna todos os `TempId` *usados* como operandos da instrução.
fn instr_uses(instr: &TacInstr) -> Vec<TempId> {
    let mut uses = Vec::new();

    let push = |uses: &mut Vec<TempId>, op: &Operand| {
        if let Operand::Temp(t) = op {
            uses.push(*t);
        }
    };

    match instr {
        TacInstr::BinOp { lhs, rhs, .. } => {
            push(&mut uses, lhs);
            push(&mut uses, rhs);
        }
        TacInstr::UnOp { src, .. } => push(&mut uses, src),
        TacInstr::Copy { src, .. } => push(&mut uses, src),
        TacInstr::CondJump { cond, .. } => push(&mut uses, cond),
        TacInstr::Call { args, .. } => {
            for arg in args {
                push(&mut uses, arg);
            }
        }
        TacInstr::Return { val: Some(v) } => push(&mut uses, v),
        _ => {}
    }

    uses
}

// ─── Constant Folding ────────────────────────────────────────────────────────

/// Avalia em tempo de compilação operações cujos dois operandos são constantes inteiras.
///
/// Retorna `true` se alguma instrução foi alterada.
///
/// **Não** dobra `ConstValue::Double` (evitar divergência de precisão host vs target).
/// **Não** dobra shift com rhs inválido, nem divisão por zero (UB em C).
pub fn constant_fold(instrs: &mut Vec<TacInstr>) -> bool {
    let mut changed = false;

    for instr in instrs.iter_mut() {
        match instr {
            TacInstr::BinOp { dst, op, lhs, rhs } => {
                if let Some(result) = fold_binop(op, lhs, rhs) {
                    *instr = TacInstr::Copy {
                        dst: Operand::Temp(*dst),
                        src: Operand::Const(result),
                    };
                    changed = true;
                }
            }
            TacInstr::UnOp { dst, op, src } => {
                if let Some(result) = fold_unop(op, src) {
                    *instr = TacInstr::Copy {
                        dst: Operand::Temp(*dst),
                        src: Operand::Const(result),
                    };
                    changed = true;
                }
            }
            _ => {}
        }
    }

    changed
}

fn fold_binop(op: &BinOp, lhs: &Operand, rhs: &Operand) -> Option<ConstValue> {
    let (Operand::Const(ConstValue::Int(l)), Operand::Const(ConstValue::Int(r))) = (lhs, rhs)
    else {
        return None;
    };
    let l = *l;
    let r = *r;

    let result = match op {
        // Aritmética
        BinOp::Add => l.checked_add(r)?,
        BinOp::Sub => l.checked_sub(r)?,
        BinOp::Mul => l.checked_mul(r)?,
        BinOp::Div => {
            if r == 0 {
                return None; // divisão por zero — UB, preservar
            }
            l.checked_div(r)?
        }
        BinOp::Mod => {
            if r == 0 {
                return None; // divisão por zero — UB, preservar
            }
            l.checked_rem(r)?
        }

        // Relacionais — resultado é 0 ou 1
        BinOp::Eq => (l == r) as i64,
        BinOp::Neq => (l != r) as i64,
        BinOp::Less => (l < r) as i64,
        BinOp::Greater => (l > r) as i64,
        BinOp::Leq => (l <= r) as i64,
        BinOp::Geq => (l >= r) as i64,

        // Lógicos — ambos constantes, curto-circuito não se aplica
        BinOp::And => ((l != 0) && (r != 0)) as i64,
        BinOp::Or => ((l != 0) || (r != 0)) as i64,

        // Bitwise
        BinOp::BitAnd => l & r,
        BinOp::BitOr => l | r,
        BinOp::BitXor => l ^ r,

        // Shift — UB se rhs < 0 ou rhs >= 64
        BinOp::Shl => {
            if r < 0 || r >= 64 {
                return None;
            }
            l.checked_shl(r as u32)?
        }
        BinOp::Shr => {
            if r < 0 || r >= 64 {
                return None;
            }
            l.checked_shr(r as u32)?
        }
    };

    Some(ConstValue::Int(result))
}

fn fold_unop(op: &UnOp, src: &Operand) -> Option<ConstValue> {
    let Operand::Const(ConstValue::Int(v)) = src else {
        return None;
    };
    let v = *v;

    let result = match op {
        UnOp::Neg => v.checked_neg()?,
        UnOp::BitNot => !v,
        UnOp::Not => (v == 0) as i64,
        // Deref e AddrOf não podem ser dobrados em tempo de compilação
        UnOp::Deref | UnOp::AddrOf => return None,
    };

    Some(ConstValue::Int(result))
}

// ─── Constant Propagation ────────────────────────────────────────────────────

/// Propaga valores constantes conhecidos de temporários para seus usos posteriores.
///
/// Mantém um mapa `TempId → ConstValue`. Ao encontrar `Copy { dst: Temp(t), src: Const(v) }`,
/// registra `t → v`. Ao encontrar operandos que referenciam temporários mapeados,
/// substitui pelo `Const` correspondente. Ao encontrar redefinição não-constante de `t`,
/// invalida a entrada do mapa.
///
/// Retorna `true` se alguma substituição foi feita.
pub fn constant_propagation(instrs: &mut Vec<TacInstr>) -> bool {
    let mut changed = false;
    let mut const_map: HashMap<TempId, ConstValue> = HashMap::new();

    for instr in instrs.iter_mut() {
        // 1. Substituir usos de temporários conhecidos pelos seus valores constantes.
        propagate_uses(instr, &const_map, &mut changed);

        // 2. Atualizar o mapa conforme o efeito da instrução.
        match instr {
            // Copy com fonte constante: registrar o mapeamento.
            TacInstr::Copy {
                dst: Operand::Temp(t),
                src: Operand::Const(v),
            } => {
                const_map.insert(*t, v.clone());
            }
            // Copy com fonte não-constante: invalidar — o temporário passou a
            // ter um valor desconhecido.
            TacInstr::Copy {
                dst: Operand::Temp(t),
                src: _,
            } => {
                const_map.remove(t);
            }
            // Qualquer outra instrução que redefine um temporário invalida a entrada.
            TacInstr::BinOp { dst, .. } | TacInstr::UnOp { dst, .. } => {
                const_map.remove(dst);
            }
            TacInstr::Call { dst: Some(t), .. } => {
                const_map.remove(t);
            }
            _ => {}
        }
    }

    changed
}

fn propagate_uses(
    instr: &mut TacInstr,
    const_map: &HashMap<TempId, ConstValue>,
    changed: &mut bool,
) {
    let subst = |op: &mut Operand, changed: &mut bool| {
        if let Operand::Temp(t) = op {
            if let Some(v) = const_map.get(t) {
                *op = Operand::Const(v.clone());
                *changed = true;
            }
        }
    };

    match instr {
        TacInstr::BinOp { lhs, rhs, .. } => {
            subst(lhs, changed);
            subst(rhs, changed);
        }
        TacInstr::UnOp { src, .. } => subst(src, changed),
        TacInstr::Copy { src, .. } => subst(src, changed),
        TacInstr::CondJump { cond, .. } => subst(cond, changed),
        TacInstr::Call { args, .. } => {
            for arg in args.iter_mut() {
                subst(arg, changed);
            }
        }
        TacInstr::Return { val: Some(v) } => subst(v, changed),
        _ => {}
    }
}

// ─── Dead Code Elimination ───────────────────────────────────────────────────

/// Remove instruções TAC cujo resultado nunca é lido e que não possuem side-effects.
///
/// Uma instrução é eliminável se:
/// 1. Define um `TempId` (não `Var`).
/// 2. Esse `TempId` não está vivo após a instrução (`live_before[i+1]`).
/// 3. Não é `Call`, `Return`, `Jump`, `CondJump`, `Label`,
///    nem `Copy` com destino `Operand::Var`.
///
/// Retorna `true` se alguma instrução foi removida.
pub fn dead_code_eliminate(instrs: &mut Vec<TacInstr>, liveness: &LivenessInfo) -> bool {
    let n = instrs.len();
    let mut keep = vec![true; n];

    for i in 0..n {
        if has_side_effects(&instrs[i]) {
            continue;
        }

        if let Some(def) = instr_def(&instrs[i]) {
            // O algoritmo backward armazena em live_before[i] o conjunto de
            // temporários vivos *antes* de remover o def e adicionar os uses de i.
            // Ou seja, live_before[i] é o live-OUT da instrução i — os temporários
            // vivos logo após a execução de i.
            // Se o def não aparece em live_before[i], ele nunca é lido depois.
            if !liveness.live_before[i].contains(&def) {
                keep[i] = false;
            }
        }
    }

    let before = instrs.len();
    let mut i = 0;
    instrs.retain(|_| {
        let k = keep[i];
        i += 1;
        k
    });

    instrs.len() != before
}

/// Retorna `true` para instruções que não podem ser removidas mesmo se o resultado
/// não for usado — elas têm efeitos colaterais observáveis.
fn has_side_effects(instr: &TacInstr) -> bool {
    matches!(
        instr,
        TacInstr::Call { .. }
            | TacInstr::Return { .. }
            | TacInstr::Jump { .. }
            | TacInstr::CondJump { .. }
            | TacInstr::Label(_)
            | TacInstr::Copy {
                dst: Operand::Var(_),
                ..
            }
    )
}

// ─── Pipeline ────────────────────────────────────────────────────────────────

const MAX_ITER: usize = 10;

/// Executa os passes de otimização sobre as instruções de uma função até ponto fixo.
pub fn optimize_function(instrs: &mut Vec<TacInstr>) {
    for _ in 0..MAX_ITER {
        let mut changed = false;

        changed |= constant_fold(instrs);
        changed |= constant_propagation(instrs);

        let liveness = compute_liveness(instrs);
        changed |= dead_code_eliminate(instrs, &liveness);

        if !changed {
            break;
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::ast::expr::{BinOp, UnOp},
        ir::tac::{ConstValue, LabelId, Operand, TacInstr, TempId},
    };

    fn int(v: i64) -> Operand {
        Operand::Const(ConstValue::Int(v))
    }

    fn temp(n: u32) -> Operand {
        Operand::Temp(TempId(n))
    }

    fn var(name: &str) -> Operand {
        Operand::Var(name.to_string())
    }

    // ── Constant Fold ──

    #[test]
    fn fold_add_two_consts() {
        let mut instrs = vec![TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::Add,
            lhs: int(2),
            rhs: int(3),
        }];
        assert!(constant_fold(&mut instrs));
        assert_eq!(
            instrs[0],
            TacInstr::Copy {
                dst: temp(0),
                src: int(5),
            }
        );
    }

    #[test]
    fn fold_nested_expression() {
        // 2 + 3*4  →  t0=3*4, t1=2+t0
        let mut instrs = vec![
            TacInstr::BinOp {
                dst: TempId(0),
                op: BinOp::Mul,
                lhs: int(3),
                rhs: int(4),
            },
            TacInstr::BinOp {
                dst: TempId(1),
                op: BinOp::Add,
                lhs: int(2),
                rhs: temp(0),
            },
        ];
        // Após fold: t0 = 12, t1 = 2 + t0 (t0 ainda é temp, precisa de propagation)
        constant_fold(&mut instrs);
        assert_eq!(instrs[0], TacInstr::Copy { dst: temp(0), src: int(12) });

        // Após propagation: t1 = 2 + 12
        constant_propagation(&mut instrs);
        // Após segundo fold: t1 = 14
        constant_fold(&mut instrs);
        assert_eq!(instrs[1], TacInstr::Copy { dst: temp(1), src: int(14) });
    }

    #[test]
    fn fold_relational_less() {
        let mut instrs = vec![TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::Less,
            lhs: int(3),
            rhs: int(5),
        }];
        assert!(constant_fold(&mut instrs));
        assert_eq!(instrs[0], TacInstr::Copy { dst: temp(0), src: int(1) });
    }

    #[test]
    fn fold_relational_eq_false() {
        let mut instrs = vec![TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::Eq,
            lhs: int(3),
            rhs: int(5),
        }];
        assert!(constant_fold(&mut instrs));
        assert_eq!(instrs[0], TacInstr::Copy { dst: temp(0), src: int(0) });
    }

    #[test]
    fn fold_bitwise_and() {
        // 0b1010 & 0b1100 = 0b1000 = 8
        let mut instrs = vec![TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::BitAnd,
            lhs: int(0b1010),
            rhs: int(0b1100),
        }];
        assert!(constant_fold(&mut instrs));
        assert_eq!(instrs[0], TacInstr::Copy { dst: temp(0), src: int(8) });
    }

    #[test]
    fn fold_unary_neg() {
        let mut instrs = vec![TacInstr::UnOp {
            dst: TempId(0),
            op: UnOp::Neg,
            src: int(7),
        }];
        assert!(constant_fold(&mut instrs));
        assert_eq!(instrs[0], TacInstr::Copy { dst: temp(0), src: int(-7) });
    }

    #[test]
    fn fold_unary_not() {
        // !0 = 1, !5 = 0
        let mut instrs = vec![
            TacInstr::UnOp { dst: TempId(0), op: UnOp::Not, src: int(0) },
            TacInstr::UnOp { dst: TempId(1), op: UnOp::Not, src: int(5) },
        ];
        constant_fold(&mut instrs);
        assert_eq!(instrs[0], TacInstr::Copy { dst: temp(0), src: int(1) });
        assert_eq!(instrs[1], TacInstr::Copy { dst: temp(1), src: int(0) });
    }

    #[test]
    fn fold_div_by_zero_preserved() {
        let original = TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::Div,
            lhs: int(10),
            rhs: int(0),
        };
        let mut instrs = vec![original.clone()];
        assert!(!constant_fold(&mut instrs));
        assert_eq!(instrs[0], original);
    }

    #[test]
    fn fold_shl_negative_rhs_preserved() {
        let original = TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::Shl,
            lhs: int(1),
            rhs: int(-1),
        };
        let mut instrs = vec![original.clone()];
        assert!(!constant_fold(&mut instrs));
        assert_eq!(instrs[0], original);
    }

    #[test]
    fn fold_shl_rhs_64_preserved() {
        let original = TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::Shl,
            lhs: int(1),
            rhs: int(64),
        };
        let mut instrs = vec![original.clone()];
        assert!(!constant_fold(&mut instrs));
        assert_eq!(instrs[0], original);
    }

    #[test]
    fn fold_double_operand_not_folded() {
        let original = TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::Add,
            lhs: Operand::Const(ConstValue::Double(1.0)),
            rhs: Operand::Const(ConstValue::Double(2.0)),
        };
        let mut instrs = vec![original.clone()];
        assert!(!constant_fold(&mut instrs));
        assert_eq!(instrs[0], original);
    }

    // ── Constant Propagation ──

    #[test]
    fn propagation_simple_chain() {
        // t0 = 5; t1 = t0 + 3  →  t1 = 5 + 3  →  (fold) t1 = 8
        let mut instrs = vec![
            TacInstr::Copy { dst: temp(0), src: int(5) },
            TacInstr::BinOp {
                dst: TempId(1),
                op: BinOp::Add,
                lhs: temp(0),
                rhs: int(3),
            },
        ];
        assert!(constant_propagation(&mut instrs));
        assert_eq!(
            instrs[1],
            TacInstr::BinOp {
                dst: TempId(1),
                op: BinOp::Add,
                lhs: int(5),
                rhs: int(3),
            }
        );
        // Após fold: t1 = 8
        constant_fold(&mut instrs);
        assert_eq!(instrs[1], TacInstr::Copy { dst: temp(1), src: int(8) });
    }

    #[test]
    fn propagation_invalidated_by_redefinition() {
        // t0 = 5; t0 = call f(); t1 = t0 + 1  →  t1 NÃO deve ser dobrado
        let mut instrs = vec![
            TacInstr::Copy { dst: temp(0), src: int(5) },
            TacInstr::Call {
                dst: Some(TempId(0)),
                fn_name: "f".to_string(),
                args: vec![],
            },
            TacInstr::BinOp {
                dst: TempId(1),
                op: BinOp::Add,
                lhs: temp(0),
                rhs: int(1),
            },
        ];
        // propagation não deve substituir t0 na última instrução
        constant_propagation(&mut instrs);
        assert!(matches!(
            &instrs[2],
            TacInstr::BinOp { lhs: Operand::Temp(_), .. }
        ));
    }

    // ── DCE ──

    #[test]
    fn dce_removes_unused_temp() {
        // t0 = 2 + 3  (nunca lido)
        let mut instrs = vec![TacInstr::BinOp {
            dst: TempId(0),
            op: BinOp::Add,
            lhs: int(2),
            rhs: int(3),
        }];
        let liveness = compute_liveness(&instrs);
        assert!(dead_code_eliminate(&mut instrs, &liveness));
        assert!(instrs.is_empty());
    }

    #[test]
    fn dce_preserves_call_even_if_unused() {
        // t0 = call f()  — side-effect, não pode ser removido
        let mut instrs = vec![TacInstr::Call {
            dst: Some(TempId(0)),
            fn_name: "f".to_string(),
            args: vec![],
        }];
        let liveness = compute_liveness(&instrs);
        assert!(!dead_code_eliminate(&mut instrs, &liveness));
        assert_eq!(instrs.len(), 1);
    }

    #[test]
    fn dce_preserves_var_assignment() {
        // x = 10  (Var — semântica observável, nunca eliminar)
        let mut instrs = vec![TacInstr::Copy {
            dst: var("x"),
            src: int(10),
        }];
        let liveness = compute_liveness(&instrs);
        assert!(!dead_code_eliminate(&mut instrs, &liveness));
        assert_eq!(instrs.len(), 1);
    }

    #[test]
    fn dce_keeps_used_temp() {
        // t0 = 5; return t0  →  t0 está vivo, não deve ser removido
        let mut instrs = vec![
            TacInstr::Copy { dst: temp(0), src: int(5) },
            TacInstr::Return { val: Some(temp(0)) },
        ];
        let liveness = compute_liveness(&instrs);
        assert!(!dead_code_eliminate(&mut instrs, &liveness));
        assert_eq!(instrs.len(), 2);
    }

    // ── Pipeline / Ponto Fixo ──

    #[test]
    fn optimize_function_full_pipeline() {
        // int x = 2 + 3 * 4;  (x nunca lido, sem return)
        // t0 = 3 * 4
        // t1 = 2 + t0
        // x = t1
        let mut instrs = vec![
            TacInstr::BinOp {
                dst: TempId(0),
                op: BinOp::Mul,
                lhs: int(3),
                rhs: int(4),
            },
            TacInstr::BinOp {
                dst: TempId(1),
                op: BinOp::Add,
                lhs: int(2),
                rhs: temp(0),
            },
            TacInstr::Copy {
                dst: var("x"),
                src: temp(1),
            },
        ];

        optimize_function(&mut instrs);

        // Após ponto fixo: t0 e t1 eliminados; x = 14
        assert_eq!(instrs.len(), 1);
        assert_eq!(
            instrs[0],
            TacInstr::Copy {
                dst: var("x"),
                src: int(14),
            }
        );
    }

    #[test]
    fn optimize_function_preserves_call_chain() {
        // t0 = call f(); t1 = t0 + 0; return t1
        // t1 = t0 + 0 pode ser simplificado mas t0 tem side-effect → não eliminar call
        let mut instrs = vec![
            TacInstr::Call {
                dst: Some(TempId(0)),
                fn_name: "f".to_string(),
                args: vec![],
            },
            TacInstr::BinOp {
                dst: TempId(1),
                op: BinOp::Add,
                lhs: temp(0),
                rhs: int(0),
            },
            TacInstr::Return { val: Some(temp(1)) },
        ];

        optimize_function(&mut instrs);

        // call f() deve ser preservado
        assert!(instrs
            .iter()
            .any(|i| matches!(i, TacInstr::Call { fn_name, .. } if fn_name == "f")));
    }

    #[test]
    fn optimize_function_side_effect_label_preserved() {
        let mut instrs = vec![
            TacInstr::Label(LabelId(0)),
            TacInstr::Return { val: None },
        ];
        optimize_function(&mut instrs);
        assert_eq!(instrs.len(), 2);
    }
}
