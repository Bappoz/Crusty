# Otimização

**Status:** Parcialmente implementada

## Visão Geral

A etapa de otimização em Crusty atua sobre o Código de Três Endereços (TAC - *Three-Address Code*). Seu objetivo é reescrever a representação intermediária gerada pela fase de *lowering* para produzir um código equivalente, porém com execução mais eficiente e enxuta.

A otimização é dividida em duas abordagens no projeto: uma **pipeline principal**, que atua iterativamente de maneira direta sobre `Vec<TacInstr>`, e uma arquitetura baseada no `PassManager`, que utiliza um modelo abstrato legado intra-bloco para orquestrar os diferentes níveis clássicos de otimização.

---

## Pipeline Principal (`src/codegen/inter/optimizations.rs`)

A integração efetiva do módulo com a IR baseada em `TacInstr` acontece através desta pipeline. O ponto de entrada é:

```rust
pub fn optimize_function(instrs: &mut Vec<TacInstr>)
```

Esta função aplica três tipos de passes iterativamente dentro de um loop, repetindo as transformações até atingir um **ponto fixo** — momento em que nenhuma instrução do código sofre modificações.

A ordem de execução de cada ciclo de otimização é:
1. **Constant Folding**
2. **Constant Propagation**
3. **Dead Code Elimination (DCE)**

### Análise de Vivacidade (*Liveness Analysis*)

O passe de eliminação de código morto depende do conhecimento sobre o tempo de vida (*liveness*) das variáveis temporárias. Para garantir que atribuições usadas após ramificações do fluxo de controle (como num `if`/`else`) não sejam acidentalmente excluídas, executa-se a função `compute_liveness(instrs)`.

Ela trabalha da seguinte forma:
1. Identifica blocos básicos dividindo a sequência linear de instruções (pela detecção de líderes, idêntico à construção do CFG em `build_cfg`).
2. Propaga iterativamente conjuntos de variáveis ativas (*live-in* e *live-out*) entre os blocos, resolvendo as equações de fluxo de dados até convergirem (ponto fixo).
3. Produz o mapa exato de temporários vivos por instrução.

### Passes Implementados na Pipeline Principal

- **`constant_fold` (Dobramento de Constantes):**  
  Inspeciona instruções binárias (`BinOp`). Caso os dois operandos sejam do tipo `ConstValue::Int`, efetua a operação matemática na própria compilação e substitui a instrução por um `Copy` da constante resultante. A aritmética conta com checagens seguras (`checked_*`): tentativas de divisão/módulo por zero e deslocamentos de bits inválidos não são "dobrados".
- **`constant_propagation` (Propagação de Constantes):**  
  Rastreia atribuições de literais (`Copy { dst: Temp, src: Const }`). Todos os usos sequenciais daquele temporário são diretamente substituídos pela respectiva constante. Se houver uma redefinição de valor para o `Temp` mapeado, ele é invalidado do cache de propagação.
- **`dead_code_eliminate` (Eliminação de Código Morto):**  
  Desarta cálculos dispensáveis ou não consumidos. Utiliza a informação advinda do `compute_liveness` para verificar se uma instrução de atribuição (`BinOp`, `UnOp` ou `Copy` para um `Temp`) será lida no futuro. Se o destino nunca for acessado, a instrução é excluída. São preservadas ramificações de código (`Jump`, `CondJump`, `Label`), efeitos no programa (`Call`, `Return`) e manipulação de variáveis/memória (locais com `Var`, globais com `Global` e referências/ponteiros com `Deref`).

---

## PassManager e CFG Legado (`src/codegen/inter/opt/`)

A codebase também carrega uma implementação alternativa intra-bloco para os passes, que utilizam tipos legados (`Assign`/`Binary`/`Nop`). O `PassManager` juntamente com o *trait* `OptPass` comandam a orquestração desses passes. 

Eles são selecionados através de uma escala de `OptLevel`, baseada em argumentos vindos da Interface de Linha de Comando (`-O0` a `-O3` / `--opt-level`):

| Nível | Passes Registrados |
|:---:|---|
| **`O0`** | Nenhum passe ativado |
| **`O1`** | `constant-fold` + `dead-code-elimination` |
| **`O2`** | `O1` + `copy-propagation` + `common-subexpression-elimination` |
| **`O3`** | `O2` + `loop-invariant-code-motion` + `inlining` |

### Comportamento e Otimizações de Nível `O2`

Além dos passes de propagação e dobramento usuais (adaptados a este CFG paralelo), o nível `O2` traz transformações ausentes na pipeline baseada no TAC real:
- **`dead-code-elimination`:** Remove diretamente as instruções inoperantes (`Nop`) acumuladas na edição do bloco.
- **`common-subexpression-elimination` (CSE - Eliminação de Subexpressão Comum):** Executado de forma **local/intra-bloco**. O passe lembra qual temporário (`dst`) armazenou o resultado computado para uma operação redundante como `(lhs, op, rhs)`. Redefinir variáveis usadas quebra a redundância, inviabilizando o rastreio daquela subexpressão.

---

## Limitações Conhecidas (Trabalhos Futuros)

!!! warning "Recursos fora do escopo da implementação atual"
    - Integração de `OptLevel` CLI para rodar explicitamente dentro do *Lowering/TAC Main Pipeline* — o PassManager legado encontra-se estagnado perante os stubs não funcionais.
    - Otimizações pesadas atreladas a `O3`: `loop-invariant-code-motion` (Movimentação de Invariantes em Loop) e `inlining` (Inclusão Inline de Funções) respondem `false` (passivos inativos).
    - Análise global para a **CSE**: A eliminação de subexpressão comum no PassManager não propaga árvores entre os dominadores de blocos básicos (Available Expressions / Inter-bloco). A atual verificação é estritamente local (intra-bloco).

