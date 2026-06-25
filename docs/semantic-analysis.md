# Análise Semântica

**Status:** Concluída

## Visão Geral

O analisador semântico recebe o `Program` (AST) produzido pelo parser e verifica restrições que a gramática não captura: variáveis não declaradas, redeclarações, atribuição a `const`, incompatibilidades de tipo, acesso a campos inexistentes.

O ponto de entrada público é `analyse(prog)`, que cria um `SemanticAnalyser` e retorna `Vec<CompilerError>`. Erros não interrompem a análise — todos são acumulados para relatório completo.

---

## Estrutura

```rust
struct SemanticAnalyser {
    sym: SymbolTable,
    current_fn_ret: Option<QualifierType>,  // tipo de retorno da função atual
    diagnostics: Vec<CompilerError>,
}
```

O fluxo é:

```
analyse_program(prog)
└── analyse_decl(decl)*
    ├── analyse_stmt(stmt)*     (dentro de Function)
    │   └── analyse_expr(expr)
    └── analyse_expr(expr)      (inicializadores)
```

---

## Tabela de Símbolos (`SymbolTable`)

A tabela de símbolos é uma pilha de escopos. Cada escopo é um `HashMap<String, Symbol>`.

```rust
struct Symbol {
    name: String,
    ty: QualifierType,
    mutable: bool,         // false se declarado com const
    decl_span: Span,
}
```

### API

| Método | Comportamento |
|---|---|
| `enter_scope()` | Empilha um novo escopo vazio |
| `exit_scope()` | Desempilha o escopo atual |
| `declare(symbol)` | Insere no escopo corrente; erro se já existir no mesmo escopo |
| `lookup(name)` | Busca do escopo mais interno ao mais externo |
| `lookup_current_scope(name)` | Busca apenas no escopo corrente |
| `register_struct(name, fields)` | Armazena definição de struct (separado dos símbolos) |
| `lookup_struct(name)` | Recupera os campos de uma struct |
| `register_type_alias(name, qty)` | Armazena alias de typedef |
| `lookup_type_alias(name)` | Recupera o tipo subjacente de um alias |

### Ciclo de vida dos escopos

```
analyse_program       → enter_scope / exit_scope  (escopo global)
analyse_decl::Function → enter_scope / exit_scope  (escopo da função + params)
analyse_stmt::Block   → enter_scope / exit_scope  (bloco aninhado)
analyse_stmt::For     → enter_scope / exit_scope  (init pode declarar variável)
```

---

## Análise de Declarações

### Variável Global / Local (`VarDecl`)

1. Analisa o inicializador (se houver)
2. Resolve aliases de typedef no tipo
3. Chama `declare(symbol)` — emite `Redeclaration` se duplicado no mesmo escopo

### Função

1. `enter_scope`
2. Salva `current_fn_ret` com o tipo de retorno resolvido
3. Declara cada parâmetro no novo escopo
4. Analisa todos os statements do corpo
5. Restaura `current_fn_ret`; `exit_scope`

### Struct

Chama `register_struct(name, fields)` — armazena a definição para uso posterior em acesso a membro.

### Enum

Cada variante é declarada como um símbolo `const int`. Variantes com valor explícito têm o inicializador analisado.

### Typedef

Resolve o tipo-base e chama `register_type_alias(alias, resolved_type)`.

---

## Resolução de Tipos (`resolve_type`)

Antes de declarar um símbolo, o tipo passa por `resolve_type()`, que substitui `Type::Alias(name)` pelo tipo concreto registrado:

```
const unsigned myint_t x
    → resolve_type → const unsigned int x   (se typedef myint_t = int)
```

A resolução é recursiva para ponteiros e arrays:

```
myint_t* p   →   int* p
myint_t[10]  →   int[10]
```

Aliases não encontrados são mantidos como `Type::Alias` (diagnóstico emitido mais tarde quando usados em contexto de tipo).

---

## Análise de Expressões e Inferência de Tipos

`analyse_expr` analisa recursivamente e **retorna o tipo inferido** da expressão. Tipos não resolvíveis retornam `Type::Void` como sentinela.

### Inferência de literais

| Literal | Tipo inferido |
|---|---|
| `IntLiteral(v)` | `int` |
| `FloatLiteral(v)` | `double` |
| `CharLiteral(v)` | `char` |
| `StringLiteral(v)` | `char*` |

### Identificador

Busca na tabela de símbolos. Se não encontrado → `UndefinedVariable(name)`.

### Atribuição

1. Verifica se o LHS é `const` → `AssignToConst(name)`
2. Infere tipos de LHS e RHS
3. Verifica compatibilidade via `types_compatible_for_assign` → `TypeMismatch`
4. Retorna o tipo do LHS

### Operações Binárias

`binary_result_type(lhs_ty, op, rhs_ty)` define as regras:

| Operador | Regra |
|---|---|
| `+`, `-` | numérico OP numérico → promoção; ponteiro ± inteiro → ponteiro |
| `*`, `/` | numérico OP numérico → promoção |
| `%` | **inteiro** OP **inteiro** (float proibido) |
| `&`, `\|`, `^`, `<<`, `>>` | **inteiro** OP **inteiro** |
| `==`, `!=`, `<`, `>`, `<=`, `>=` | num↔num ou ptr↔ptr → `int` |
| `&&`, `\|\|` | escalar OP escalar → `int` |

Promoção numérica: `Double > Float > Long > Int > Short/Char`.

### Acesso a Membro (`.`, `->`)

1. Infere o tipo do objeto
2. Para `.`: espera `Type::Struct(name)` — caso contrário `TypeMismatch`
3. Para `->`: espera `Type::Pointer(Struct(name))` — caso contrário `TypeMismatch`
4. Busca `name` em `lookup_struct` → `UndefinedStruct` se ausente
5. Busca `field_name` nos campos → `FieldNotFound` se ausente
6. Retorna o tipo do campo

### Outros nós

| Nó | Tipo retornado |
|---|---|
| `Unary`, `Prefix`, `Postfix` | mesmo tipo do operando |
| `CompoundAssign` | tipo do LHS |
| `Cast(qty, _)` | `qty` resolvido |
| `Sizeof(_)`, `SizeofType(_)` | `unsigned int` |
| `Call(callee, args)` | `void` (sentinela — lookup de retorno não implementado) |
| `Index(arr, idx)` | tipo do elemento (desreferencia `Array(T)` ou `Pointer(T)`) |
| `Ternary(cond, then, else)` | tipo do ramo `then` |

---

## Erros Semânticos

Todos são do tipo `CompilerError::Semantic(SemanticError { span, kind })`:

| Kind | Causa |
|---|---|
| `Redeclaration(name)` | Nome já declarado no escopo atual |
| `UndefinedVariable(name)` | Identificador não encontrado em nenhum escopo |
| `AssignToConst(name)` | Atribuição a variável declarada com `const` |
| `TypeMismatch { expected, found }` | Tipos incompatíveis em atribuição ou operação binária |
| `UndefinedStruct(name)` | Acesso a membro de struct não registrada |
| `FieldNotFound { struct_name, field_name }` | Campo não existe na struct |

---

## Limitações atuais

!!! warning "Ainda não implementado"
    - Verificação de tipo de retorno de função (`return expr` vs. tipo declarado)
    - Lookup de tipo de retorno em chamadas de função (`Call` retorna `void` sentinela)
    - Verificação de compatibilidade entre ramos `then`/`else` no ternário
    - Aritmética de ponteiro para `Sub` (ponteiro − ponteiro → `ptrdiff_t`)
