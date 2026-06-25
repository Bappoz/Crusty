# Representação Intermediária

**Status:** Implementada, com limitações conhecidas e integração de otimização parcial

A representação intermediária do Crusty é baseada em TAC (*Three-Address Code*) e fica em
`src/ir/`. Ela faz a ponte entre a AST validada pela análise semântica e o backend x86-64 em
`src/codegen/last/`.

O caminho usado pela CLI hoje é:

```
AST
  ↓
lower_program (src/ir/lower.rs)
  ↓
TacProgram { globals, functions }
  ↓
emit_program (src/codegen/last/x86_64.rs)
  ↓
assembly / objeto / executável
```

Também existem:

- `build_cfg` em `src/ir/cfg.rs`, usado para construir um CFG a partir de uma `TacFunction`.
- `lower_and_optimize` em `src/ir/lower.rs`, que gera TAC e aplica a pipeline de otimização sobre
  `Vec<TacInstr>`.
- `PassManager` em `src/codegen/inter/opt/`, um modelo de otimização paralelo/legado que opera
  sobre `src/codegen/inter::Cfg`, não sobre `TacProgram`.

---

## Three-Address Code (`src/ir/tac.rs`)

O TAC usa temporários (`TempId`), rótulos (`LabelId`) e operandos explícitos para representar
cálculos, desvios, chamadas, retornos e acessos indiretos de memória.

```rust
enum Operand {
    Temp(TempId),
    Var(String),
    Global(String),
    Const(ConstValue),
    Deref(Box<Operand>),
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

`Operand::Var` representa variáveis locais e parâmetros da função atual. `Operand::Global`
representa objetos de duração estática. `Operand::Deref` permite leitura e escrita indireta,
incluindo `*p`, `arr[i]` e acesso a campos calculado por endereço.

O programa TAC é agrupado assim:

```rust
struct TacProgram {
    globals: Vec<TacGlobal>,
    functions: Vec<TacFunction>,
}

struct TacFunction {
    name: String,
    params: Vec<String>,
    instrs: Vec<TacInstr>,
    var_sizes: HashMap<String, i64>,
}

struct TacGlobal {
    name: String,
    size: i64,
    init: Option<ConstValue>,
}
```

`var_sizes` é usado pelo backend para reservar espaço para locais maiores que um slot escalar,
especialmente structs locais e arrays fixos. `TacGlobal` descreve globais inicializados ou zerados.

---

## Lowering da AST (`src/ir/lower.rs`)

O ponto de entrada principal é:

```rust
pub fn lower_program(prog: &Program) -> Result<TacProgram, CodegenError>
```

Durante o lowering, o `Lowerer` mantém contexto de:

- tipos de variáveis locais, parâmetros e globais;
- escopos locais;
- aliases de `typedef`;
- layouts de `struct`;
- tamanhos de variáveis agregadas para o frame do backend.

`lower_function(decl)` existe para baixar uma função isolada em testes ou usos pontuais. Já
`lower_and_optimize(prog)` chama `lower_program` e depois `optimize_function` para cada função,
mas a CLI atualmente chama `lower_program` diretamente.

### Expressões

| Expressão | Tradução atual |
|---|---|
| Literal | `Operand::Const` |
| Identificador local/parâmetro | `Operand::Var` |
| Identificador global | `Operand::Global` |
| Binária / Unária | Emite `BinOp`/`UnOp` em temporário fresco |
| `Assign` | Baixa RHS e emite `Copy` para lvalue suportado |
| `CompoundAssign` | Baixa como `tmp = dst op rhs; dst = tmp` |
| `Prefix`/`Postfix` | Emite incremento/decremento por `BinOp` + `Copy`; postfix preserva o valor antigo |
| `Call` | Baixa argumentos em ordem; aceita apenas callee identificador simples |
| `Ternary` | Expande para `CondJump`, rótulos de then/else e temporário comum |
| `Cast` | Baixa a expressão interna; a anotação de tipo não vira instrução TAC |
| `SizeofType` | Resolve via `type_size` |
| `Sizeof(expr)` | Suporta identificador simples com tipo conhecido |
| `Index` | Calcula endereço base + índice escalado e retorna `Operand::Deref` |
| `Member` (`.`/`->`) | Usa layout de struct para calcular offset do campo e retorna `Operand::Deref` |
| `Unary(Deref)` | Como rvalue, emite `UnOp`; como alvo de atribuição, vira `Operand::Deref` |
| `Unary(AddrOf)` | Emite `UnOp::AddrOf` |

### Comandos

`If`, `While`, `For` e `DoWhile` são traduzidos para rótulos, `CondJump` e `Jump`.
`break` e `continue` usam `ControlLabels` e retornam `CodegenError` quando aparecem fora de contexto
válido.

`Switch` está implementado com cadeia de comparações por `case`, salto para `default` quando existe
e fallthrough real entre casos. `break` dentro do `switch` salta para o rótulo final; `continue`
preserva o alvo do loop externo quando houver.

`VarDecl` registra o tipo no contexto do lowering e emite inicialização quando há initializer.
Declarações globais são baixadas em `TacGlobal`, com inicialização estática literal ou expressão
inteira constante.

### Layout, arrays e structs

O lowering calcula layouts simples de structs declaradas no programa. Campos escalares e ponteiros
têm tamanho conhecido; structs ou arrays aninhados em campos ainda são recusados para evitar layout
incorreto.

Arrays fixos locais entram em `var_sizes` e podem ser lidos/escritos por índice. Arrays globais
ainda não têm armazenamento suportado no lowering de globais.

---

## Grafo de Fluxo de Controle (`src/ir/cfg.rs`)

`build_cfg(&TacFunction) -> Cfg` particiona uma função TAC linear em blocos básicos.

Líderes reconhecidos:

1. A primeira instrução.
2. Toda instrução `Label`.
3. A instrução logo após `Jump` ou `CondJump`.

Cada `BasicBlock` contém `id`, `instrs`, `succs` e `preds`. Os sucessores são derivados da última
instrução do bloco:

- `Jump` aponta para o bloco do rótulo alvo;
- `CondJump` aponta para os blocos `then_label` e `else_label`;
- `Return` não tem sucessor;
- outros casos fazem fallthrough para o próximo bloco.

`Cfg` mantém `entry` e `exit`, além de helpers `predecessors` e `successors`.

---

## Otimização Sobre TAC (`src/codegen/inter/optimizations.rs`)

A pipeline real sobre `TacInstr` é:

```rust
pub fn optimize_function(instrs: &mut Vec<TacInstr>)
```

Ela aplica, até ponto fixo ou até `MAX_ITER = 10`:

1. `constant_fold`
2. `constant_propagation`
3. `dead_code_eliminate`

`constant_fold` dobra operações binárias e unárias com `ConstValue::Int`, preservando divisão/módulo
por zero, shifts inválidos e operações de endereço/deref. `Double` não é dobrado.

`constant_propagation` rastreia temporários definidos por `Copy { dst: Temp, src: Const }` e substitui
usos posteriores até que o temporário seja redefinido.

`dead_code_eliminate` usa `compute_liveness` com consciência de fluxo de controle. A análise divide
a lista em blocos básicos, propaga `live-in`/`live-out` por ponto fixo e remove apenas definições de
temporários sem uso futuro e sem efeitos observáveis. São preservados `Call`, `Return`, desvios,
`Label`, escritas em `Var` e escritas em `Global`.

### Integração atual

`lower_and_optimize` usa essa pipeline, mas `main.rs` ainda não a chama. A CLI gera o backend a partir
de `lower_program`, portanto os executáveis emitidos hoje não passam por `optimize_function`.

---

## PassManager Legado (`src/codegen/inter/opt/`)

`PassManager` opera sobre outro modelo:

```rust
src/codegen/inter::Cfg
src/codegen/inter::Instruction::{Assign, Binary, Nop}
src/codegen/inter::Value::{Int, Temp}
```

Esse modelo é usado por testes unitários e pelo parsing das flags `-O0` a `-O3`, mas a CLI cria um
`Cfg::new()` vazio e roda o pipeline nele. Ou seja: as flags de otimização ainda não otimizam o
`TacProgram` que segue para o backend.

| Nível | Passes registrados |
|---|---|
| `O0` | nenhum |
| `O1` | `constant-fold`, `dead-code-elimination` |
| `O2` | O1 + `copy-propagation`, `common-subexpression-elimination` |
| `O3` | O2 + `loop-invariant-code-motion`, `inlining` |

Estado dos passes:

- `constant-fold`: implementado para `Instruction::Binary`.
- `dead-code-elimination`: remove apenas `Instruction::Nop`.
- `copy-propagation`: local por bloco.
- `common-subexpression-elimination`: local por bloco.
- `loop-invariant-code-motion`: implementado sobre o CFG legado, com detecção de back-edges e
  preheader simples.
- `inlining`: stub; retorna `false`.

---

## Integração com a CLI

`main.rs` aceita:

- `--dump-ir`, mas ele ainda imprime apenas `=== IR (not yet implemented) ===`;
- `-O0|-O1|-O2|-O3` e `--opt-level 0|1|2|3`, mas esses níveis rodam no CFG legado vazio;
- `--emit=asm`, `--emit=obj`, `--emit=exe`, `-S` e `--emit-asm`;
- `--dump-tokens`, `--dump-ast`, `--only-lex`, `--only-parse`, `--only-semantic`.

Após lexing, parsing e análise semântica, a CLI baixa a AST com `lower_program` e chama
`src/codegen/last::emit_program` para gerar assembly x86-64.

---

## Limitações Atuais

!!! warning "Limitações implementadas como erro explícito"
    - chamada por expressão arbitrária, como `(*fp)(...)`, ainda não é suportada no lowering;
    - `sizeof(expr)` só suporta identificadores simples;
    - `sizeof(type)` não cobre `void`, `struct`, `alias`, `function` nem array sem tamanho conhecido;
    - globais com tipo `Array`, `Void`, `Alias` ou `Function` não têm armazenamento suportado;
    - campos de struct com tipo agregado aninhado (`struct`/`array`) não têm layout suportado;
    - atribuição direta entre structs maiores que 8 bytes é recusada; deve ser feita campo a campo;
    - o dump de IR da CLI ainda não mostra o TAC;
    - a pipeline de otimização sobre `Vec<TacInstr>` existe, mas não está conectada ao fluxo padrão da CLI.
