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
                                        │
                                        ▼
                   optimize_function (src/codegen/inter/optimizations.rs)
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

O ponto de entrada recomendado para a pipeline de compilação é `lower_and_optimize(prog)`, que
invoca `lower_program` seguido de `optimize_function` para cada função.

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
| `SizeofType` | Resolvido estaticamente via `type_size` para `Operand::Const(Int)` |

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

## Pipeline de Otimização (`src/codegen/inter/optimizations.rs`)

Os passes de otimização operam **diretamente sobre `Vec<TacInstr>`** — a mesma representação
produzida pelo lowering — através das funções de nível de módulo em `optimizations.rs`. Esta é a
integração efetiva entre o TAC gerado por `src/ir` e a etapa de otimização; o modelo de CFG
alternativo em `src/codegen/inter/opt/` (com `Assign`/`Binary`/`Nop`) é utilizado apenas pelos
passes registrados no `PassManager` (nível de abstração legado, não conectado ao pipeline principal).

```rust
// Ponto de entrada da pipeline de otimização sobre TacInstr:
pub fn optimize_function(instrs: &mut Vec<TacInstr>)
```

`optimize_function` executa constant folding, constant propagation e dead-code elimination até
ponto fixo, nessa ordem, repetindo enquanto houver mudanças.

### Análise de Vivacidade (`compute_liveness`)

`compute_liveness(instrs)` constrói um `LivenessInfo` com consciência de fluxo de controle:
internamente divide a lista linear em blocos básicos via `split_into_blocks` (mesmos critérios de
líder de `build_cfg`), propaga conjuntos *live-in*/*live-out* entre blocos por ponto fixo e retorna
os temporários vivos em cada ponto do programa. Isso garante que o DCE preserve definições cujo
valor é consumido apenas após um merge de branches (`if`/`else`).

### Passes implementados (`optimizations.rs`)

- **`constant_fold`** — substitui `BinOp` por `Copy` quando ambos operandos são `ConstValue::Int`,
  usando aritmética `checked_*` (divisão/módulo por zero, shifts negativos ou ≥ 64 bits não são
  dobrados).
- **`constant_propagation`** — rastreia atribuições `Copy { dst: Temp, src: Const }` e substitui
  usos subsequentes do temporário pela constante, invalidando entradas ao redefinir o temporário.
- **`dead_code_eliminate`** — remove `BinOp`/`UnOp`/`Copy` cujo destino é um `Temp` nunca lido
  após o ponto de definição; preserva `Call`, `Return`, `Jump`/`CondJump`, `Label` e atribuições
  para `Var` (efeitos observáveis).

### Passes no `PassManager` (`src/codegen/inter/opt/`)

O `PassManager` e o trait `OptPass` orquestram passes sobre o CFG alternativo. `OptLevel`
(`O0`–`O3`, selecionável via flags `-O0`..`-O3`/`--opt-level`) define o pipeline registrado:

| Nível | Passes |
|---|---|
| `O0` | nenhum |
| `O1` | constant-fold, dead-code-elimination |
| `O2` | O1 + copy-propagation, common-subexpression-elimination |
| `O3` | O2 + loop-invariant-code-motion, inlining |

- **`constant-fold`** (implementado) — mesma semântica de `constant_fold` acima, sobre o modelo de CFG legado.
- **`dead-code-elimination`** (implementado) — remove instruções `Nop` de cada bloco.
- **`copy-propagation`** (implementado) — substitui usos de temporários por suas cópias de constantes dentro de cada bloco.
- **`common-subexpression-elimination`** (implementado) — eliminação **local** (intra-bloco): mantém cache `(lhs, op, rhs) → destino`; ao repetir uma expressão, remove a instrução redundante e renomeia usos futuros. Redefinições invalidam entradas do cache.

!!! warning "Ainda não implementado (stubs que retornam `false`)"
    - `loop-invariant-code-motion`
    - `inlining`
    - CSE **global** (entre blocos, via análise de *available expressions* com dominadores)

---

## Integração com a CLI

`main.rs` aceita `-O0|-O1|-O2|-O3` / `--opt-level 0|1|2|3` para selecionar o pipeline do
`PassManager`. A flag `--dump-ir` (dump do TAC) está reservada na ajuda da CLI mas **ainda não
implementada** — o lowering e o pipeline de otimização sobre `TacInstr` não estão conectados ao
`main` em nenhum nível ainda; hoje são exercitados apenas pelos testes unitários de cada módulo.
O ponto de entrada `lower_and_optimize` em `src/ir/lower.rs` está disponível para uso futuro pela
pipeline principal.
