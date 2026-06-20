# Representação Intermediária

**Status:** Em desenvolvimento

Geração de uma forma intermediária independente de arquitetura (TAC — *Three-Address Code*),
ponte entre a AST anotada e a geração de código de máquina. A etapa cobre três responsabilidades:
baixar (*lower*) a AST para TAC, montar o grafo de fluxo de controle (CFG) a partir do TAC, e
otimizar o resultado através de um pipeline de passes configurável.

```
AST → Lowerer (src/ir/lower.rs) → TacFunction/TacProgram (src/ir/tac.rs)
                                        │
                                        ▼
                              build_cfg (src/ir/cfg.rs)
```

---

## Three-Address Code (`src/ir/tac.rs`)

Cada instrução TAC tem no máximo um operador e até dois operandos, com o resultado sempre
endereçado por um destino explícito.

```rust
enum Operand {
    Temp(TempId),       // temporário gerado pelo lowering
    Var(String),        // variável do programa fonte
    Const(ConstValue),  // literal (Int, Double, Char, String)
}

enum TacInstr {
    BinOp { dst: TempId, op: BinOp, lhs: Operand, rhs: Operand },
    UnOp  { dst: TempId, op: UnOp, src: Operand },
    Copy  { dst: Operand, src: Operand },
    Jump      { label: LabelId },
    CondJump  { cond: Operand, then_label: LabelId, else_label: LabelId },
    Call      { dst: Option<TempId>, fn_name: String, args: Vec<Operand> },
    Return    { val: Option<Operand> },
    Label(LabelId),
}
```

`TempId` e `LabelId` são gerados por `TempGen`/`LabelGen`, contadores monotônicos
(`fresh()` retorna o próximo id). Uma `TacFunction` agrupa `name`, `params` e a lista linear de
`instrs`; um `TacProgram` agrupa as `TacFunction`s do programa. Todas as instruções e operandos
implementam `Display`, usado para depuração (`t0 = t1 + t2`, `if t3 goto L0 else goto L1`).

---

## Lowering da AST (`src/ir/lower.rs`)

`Lowerer` percorre a AST e empilha instruções TAC em um buffer interno (`Vec<TacInstr>`). A API
pública é `lower_program(prog)` / `lower_function(decl)`, usadas a partir de `lower_expr`/`lower_stmt`
internamente.

### Expressões (`lower_expr` → `Operand`)

| Expressão | Tradução |
|---|---|
| Literal | `Operand::Const` |
| Identificador | `Operand::Var` |
| Binária / Unária | Baixa os operandos, emite `BinOp`/`UnOp` em um temporário fresco |
| `Assign` | Baixa o RHS, emite `Copy` para o destino |
| `CompoundAssign` (`+=`, `-=`, ...) | Baixa como `tmp = dst op rhs; dst = tmp` |
| `Prefix`/`Postfix` (`++x`, `x--`) | Implementados como `BinOp` de `dst ± 1` seguido de `Copy`; o postfix preserva o valor antigo em um temporário antes de atualizar |
| `Call` | Baixa os argumentos em ordem, emite `Call` com destino em temporário fresco |
| `Ternary` | Expandido em `CondJump` + dois blocos rotulados, cada um copiando seu valor para um temporário comum |
| `Cast` | Repassado (a baixa não preserva a anotação de tipo) |

### Comandos (`lower_stmt`)

`If`/`While`/`For`/`DoWhile` são expandidos para `CondJump` + rótulos (`LabelGen::fresh`).
`break`/`continue` resolvem para `Jump` usando os rótulos de controle (`ControlLabels`) propagados
pela pilha de chamadas — por isso `lower_stmt` é correto apenas dentro de loops; fora deles,
`break`/`continue` causam `panic!`.

`emit_jump_unless_terminated` evita emitir um `Jump` redundante quando o bloco já termina em
`Jump`/`Return` (ex.: `if` cujo `then`-branch já tem `return`).

### Limitações atuais

!!! warning "Ainda não implementado"
    - `Expr::Index` (acesso por índice) e `Expr::Member` (`.`/`->`) — `panic!` na baixa
    - `Expr::Sizeof` de expressão (apenas `sizeof(tipo)` é suportado, via `type_size`)
    - `Stmt::Switch` — `panic!` na baixa
    - Chamada por expressão arbitrária (`(*fp)(...)`) — apenas `Expr::Ident` como callee
    - `lower_assignment_target` só aceita `Expr::Ident` como LHS (sem suporte a `*p = x`, `a[i] = x`, `s.campo = x`)
    - `type_size` não cobre `Array`, `Struct`, `Alias` nem `Function` (faltam layout/tamanho completos)

---

## Grafo de Fluxo de Controle (`src/ir/cfg.rs`)

`build_cfg(&TacFunction) -> Cfg` particiona a lista linear de `TacInstr` em blocos básicos pelo
algoritmo clássico de **líderes**:

1. A primeira instrução é sempre líder.
2. O alvo de todo `Jump`/`CondJump` (via `Label`) é líder.
3. A instrução imediatamente após um `Jump`/`CondJump` é líder.

Cada bloco (`BasicBlock { id: BlockId, instrs, succs, preds }`) recebe seus sucessores a partir da
última instrução: `Jump` → um sucessor, `CondJump` → dois, `Return` → nenhum, qualquer outra coisa
→ o próximo bloco em sequência (*fallthrough*). Os predecessores são derivados invertendo os
sucessores. `Cfg` também guarda `entry`/`exit` e implementa `Display` (formato `Bn: instrs / succs: [...]`).

---

## Pipeline de Otimização (`src/codegen/inter/opt/`)

Os passes de otimização operam sobre uma representação de CFG simplificada própria
(`codegen::inter::{Cfg, BasicBlock, Instruction, Value, BinaryOp}`), com apenas `Assign`, `Binary`
e `Nop` como instruções — **ainda não é a mesma estrutura de `src/ir::tac::TacInstr`** descrita
acima; a integração entre o TAC "completo" (lowering) e o CFG de otimização é um trabalho futuro.

```rust
trait OptPass {
    fn name(&self) -> &'static str;
    fn run(&self, cfg: &mut Cfg) -> bool;  // true se alterou o CFG
}
```

`PassManager::run(cfg, max_iter)` executa todos os passes registrados repetidamente até ponto fixo
(nenhum pass reporta mudança) ou até `max_iter` iterações. `OptLevel` (`O0`–`O3`, selecionável via
flags `-O0`..`-O3`/`--opt-level`) define o pipeline:

| Nível | Passes |
|---|---|
| `O0` | nenhum |
| `O1` | constant-fold, dead-code-elimination |
| `O2` | O1 + copy-propagation, common-subexpression-elimination |
| `O3` | O2 + loop-invariant-code-motion, inlining |

### Passes implementados

- **`constant-fold`** — substitui `Binary` por `Assign` quando ambos operandos são `Value::Int`,
  usando aritmética `checked_*` (divisão/módulo por zero não são dobrados).
- **`dead-code-elimination`** — remove instruções `Nop` de cada bloco.
- **`common-subexpression-elimination`** — eliminação **local** (intra-bloco) de subexpressões
  repetidas. Mantém um cache `(lhs, op, rhs) → destino` por bloco; ao repetir uma expressão, remove
  a instrução redundante e propaga (renomeia) usos futuros do destino antigo para o já calculado.
  Qualquer redefinição de variável/temporário invalida as entradas do cache que a referenciam, seja
  como operando ou como destino já calculado.

### Passes ainda não implementados (stubs que retornam `false`)

!!! warning "Ainda não implementado"
    - `copy-propagation`
    - `loop-invariant-code-motion`
    - `inlining`
    - CSE **global** (entre blocos, via análise de *available expressions* com dominadores) — fora
      de escopo do CSE local atual

---

## Integração com a CLI

`main.rs` aceita `-O0|-O1|-O2|-O3` / `--opt-level 0|1|2|3` para selecionar o pipeline. A flag
`--dump-ir` (dump do TAC) está reservada na ajuda da CLI mas **ainda não implementada** — o
lowering e o CFG de otimização não estão conectados ao `main` em nenhum nível ainda; hoje são
exercitados apenas pelos testes unitários de cada módulo.
