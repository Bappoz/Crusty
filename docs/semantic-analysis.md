# Análise Semântica

**Status:** Em desenvolvimento

Recebe o `Program` (AST) produzido pelo parser e verifica restrições que a gramática não captura:
variáveis não declaradas, redeclarações, atribuição a `const`, incompatibilidades de tipo,
acesso a campos inexistentes.

---

## Estrutura do Analisador

```rust
struct SemanticAnalyser {
    sym: SymbolTable,
    current_fn_ret: Option<QualifierType>,  // tipo de retorno da função atual
    diagnostics: Vec<CompilerError>,
}
```

O ponto de entrada público é `analyse(prog)`, que retorna `Vec<CompilerError>`.
Erros não interrompem a análise — todos são acumulados.

---

## Tabela de Símbolos

A tabela de símbolos é uma **pilha de escopos**. Cada escopo é um `HashMap<String, Symbol>`.

```rust
struct Symbol {
    name: String,
    ty: QualifierType,
    mutable: bool,      // false para declarações const
    decl_span: Span,
}
```

### API

| Método | Comportamento |
|---|---|
| `enter_scope()` | Empilha novo escopo vazio |
| `exit_scope()` | Desempilha escopo atual |
| `declare(symbol)` | Insere no escopo corrente; erro se duplicado no mesmo escopo |
| `lookup(name)` | Busca do escopo mais interno ao mais externo |
| `register_struct(name, fields)` | Armazena definição de struct |
| `lookup_struct(name)` | Recupera campos de uma struct |
| `register_type_alias(name, qty)` | Armazena alias de typedef |
| `lookup_type_alias(name)` | Recupera o tipo subjacente de um alias |

### Ciclo de vida dos escopos

```
analyse_program()         → escopo global
analyse_decl::Function    → escopo da função + parâmetros
analyse_stmt::Block       → bloco aninhado
analyse_stmt::For         → init pode declarar variável
```

---

## Resolução de Tipos

Antes de declarar um símbolo, o tipo passa por `resolve_type()`, que substitui
`Type::Alias(name)` pelo tipo concreto registrado via typedef. A resolução é recursiva:

```
typedef int myint_t;

myint_t*    →  int*
myint_t[10] →  int[10]
```

---

## Análise de Declarações

| Declaração | Ação |
|---|---|
| `GlobalVar` / `VarDecl` | Resolve tipo, chama `declare` |
| `Function` | `enter_scope`, declara params, analisa body, `exit_scope` |
| `StructDecl` | Chama `register_struct` |
| `EnumDecl` | Declara cada variante como símbolo `const int` |
| `Typedef` | Resolve tipo base, chama `register_type_alias` |

---

## Inferência e Verificação de Tipos

`analyse_expr` analisa recursivamente e **retorna o tipo inferido** da expressão.

### Literais

| Literal | Tipo inferido |
|---|---|
| `IntLiteral` | `int` |
| `FloatLiteral` | `double` |
| `CharLiteral` | `char` |
| `StringLiteral` | `char*` |

### Atribuição

1. Verifica se o LHS é `const` → `AssignToConst`
2. Infere tipos de LHS e RHS
3. Verifica compatibilidade → `TypeMismatch`
4. Retorna tipo do LHS

### Operações Binárias

| Operadores | Regra |
|---|---|
| `+` `-` | numérico OP numérico → promoção; ponteiro ± inteiro → ponteiro |
| `*` `/` | numérico OP numérico → promoção |
| `%` `&` `\|` `^` `<<` `>>` | **inteiro** OP **inteiro** |
| `==` `!=` `<` `>` `<=` `>=` | num↔num ou ptr↔ptr → `int` |
| `&&` `\|\|` | escalar OP escalar → `int` |

**Promoção numérica:** `Double > Float > Long > Int > Short/Char`

### Acesso a Membro (`.`, `->`)

1. Infere o tipo do objeto
2. `.` espera `Struct(name)`; `->` espera `Pointer(Struct(name))`
3. Busca `name` via `lookup_struct` → `UndefinedStruct` se ausente
4. Busca `field_name` nos campos → `FieldNotFound` se ausente

---

## Erros Semânticos

Todos são do tipo `CompilerError::Semantic(SemanticError { span, kind })`:

| Variante | Causa |
|---|---|
| `Redeclaration(name)` | Nome já declarado no escopo atual |
| `UndefinedVariable(name)` | Identificador não encontrado em nenhum escopo |
| `AssignToConst(name)` | Atribuição a variável `const` |
| `TypeMismatch { expected, found }` | Tipos incompatíveis em atribuição ou operação |
| `UndefinedStruct(name)` | Acesso a membro de struct não registrada |
| `FieldNotFound { struct_name, field_name }` | Campo não existe na struct |

---

## Limitações atuais

!!! warning "Ainda não implementado"
    - Verificação de compatibilidade do tipo de retorno de funções
    - Lookup do tipo de retorno em chamadas de função (`Call` retorna `void` sentinela)
    - Verificação de compatibilidade entre ramos `then`/`else` no ternário
    - Aritmética de ponteiro para subtração ponteiro−ponteiro (`ptrdiff_t`)
