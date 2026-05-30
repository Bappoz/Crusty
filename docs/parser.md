# Parser

## Visão Geral

O parser consome a sequência de tokens produzida pelo lexer e constrói uma AST (Abstract Syntax Tree). A entrada é `Vec<Token>` e a saída é `Result<Program, Vec<CompilerError>>`.

O `Program` é simplesmente `Vec<Decl>` — uma lista de declarações globais. Erros sintáticos são acumulados: ao encontrar um erro, o parser sincroniza no próximo ponto seguro e continua tentando parsear o restante do arquivo.

---

## Estrutura do Parser

```rust
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<CompilerError>,
}
```

API interna:

| Método | Comportamento |
|---|---|
| `peek()` | Token atual sem avançar |
| `peek_kind()` | `TokenKind` do token atual |
| `peek_at(n)` | Token em `pos + n` sem avançar (lookahead arbitrário) |
| `peek_next()` | Atalho para `peek_at(1)` |
| `advance()` | Consome token atual, retorna referência |
| `check(kind)` | `true` se discriminante bate (ignora payload) |
| `match_kind(kind)` | Consome e retorna `true` se bate, senão `false` |
| `expect(kind, msg)` | Consome ou emite `SyntaxError` com `expected` e `found` |
| `is_at_end()` | `true` se token atual é `Eof` |

---

## Recuperação de Erros

Quando `parse_global_item()` falha, o erro é empurrado para `diagnostics` e `synchronize()` é chamado. A sincronização avança tokens descartando tudo até encontrar `;` ou `}` (consumindo o delimitador), e então tenta o próximo item global.

Isso permite que um arquivo com múltiplos erros produza todos os diagnósticos de uma vez, em vez de parar no primeiro.

---

## Hierarquia de Parse

```
parse_program()
└── parse_global_item()          ← loop até EOF
    ├── parse_struct_decl()      ← se: struct Name {
    ├── parse_function_decl()    ← se: tipo nome (
    └── parse_global_var_decl()  ← senão: variável global

parse_function_decl()
└── parse_block()
    └── parse_stmt()             ← loop até }
        ├── parse_var_decl()
        ├── parse_block()        ← recursivo
        ├── if / while / for / do-while / switch
        ├── return / break / continue
        └── parse_expr() + ;

parse_expr(min_bp)               ← Pratt parser
├── parse_prefix_expr()
└── loop:
    ├── try_parse_postfix()
    └── infix com binding power
```

---

## Declarações Globais

### Dispatcher (`parse_global_item`)

O dispatcher usa lookahead para decidir o tipo de declaração sem consumir tokens:

```
peek[0] == Struct
  && peek[1] == Identifier
  && peek[2] == LeftBrace      →  parse_struct_decl()

senão: parse_type() + nome
  peek == LeftParen             →  parse_function_decl()
  senão                         →  parse_global_var_decl()
```

O lookahead de 2 tokens para struct (via `peek_at(2)`) é necessário para distinguir `struct Point { ... }` (definição) de `struct Point p` (uso como tipo).

### Struct (`parse_struct_decl`)

```
struct Nome {
    tipo campo;
    tipo campo;
    ...
};
```

Produz `Decl::StructDecl(nome, Vec<(QualifierType, String)>, span)`.

### Função (`parse_function_decl`)

```
tipo nome ( [void | tipo param (, tipo param)*] ) { corpo }
```

Parâmetros `void` são tratados como lista vazia. Produz `Decl::Function(retorno, nome, params, stmts, span)`.

### Variável Global (`parse_global_var_decl`)

```
tipo nome [N]* [= expr] ;
```

Após tipo e nome, chama `parse_array_suffix()` para capturar dimensões de array, depois o inicializador opcional. Produz `Decl::GlobalVar(qty, nome, init, span)`.

---

## Tipos (`parse_type`)

O parse de tipos segue esta sequência:

1. **Qualificadores** — loop que consome `const` e `unsigned` em qualquer ordem
2. **Tipo base** — um dos: `int`, `long`, `short`, `char`, `float`, `double`, `void`, `struct Nome`
3. **Ponteiros** — loop que consome `*`, envolvendo o tipo em `Type::Pointer(Box::new(ty))` a cada iteração

```
const unsigned int **  →  QualifierType {
                              ty: Pointer(Pointer(Int)),
                              is_const: true,
                              is_unsigned: true,
                          }
```

### Sufixo de Array (`parse_array_suffix`)

Chamado após o nome da variável (não após o tipo), porque em C o `[N]` vem depois do identificador:

```
int arr[10][20]
      ^^^^^^^^^  sufixo — envolve o tipo em Array(Array(Int))
```

O tamanho é consumido como expressão mas **não é armazenado** no AST (`Type::Array` carrega apenas o tipo do elemento).

---

## Statements (`parse_stmt`)

O dispatcher reconhece o tipo pelo token atual:

| Token | Statement |
|---|---|
| `{` | `Stmt::Block` |
| `return` | `Stmt::Return(Option<Expr>)` |
| `break` | `Stmt::Break` |
| `continue` | `Stmt::Continue` |
| `if` | `Stmt::If(cond, then, Option<else>)` |
| `while` | `Stmt::While(cond, body)` |
| `do` | `Stmt::DoWhile(cond, body)` |
| `for` | `Stmt::For(init, cond, inc, body)` |
| `switch` | `Stmt::Switch(expr, cases)` |
| `starts_type()` | `Stmt::VarDecl(qty, nome, init)` |
| qualquer outro | `Stmt::ExprStmt(expr)` |

### For

O `for` recebe tratamento especial para o inicializador:

```
for ( init ; cond ; inc ) body
      ^^^^
      pode ser:
        - declaração de variável (tipo presente)
        - expressão seguida de ;
        - vazio (só ;)
```

### Switch

Cases são coletados em loop. Cada `SwitchCase` acumula statements até encontrar outro `case`, `default` ou `}`:

```rust
struct SwitchCase {
    label: SwitchLabel,    // Case(Expr) | Default
    stmts: Vec<Stmt>,
    span: Span,
}
```

---

## Expressões — Pratt Parser

O Pratt parser é o algoritmo central para expressar precedência sem criar uma gramática recursiva de dezenas de níveis. Ele opera com **binding powers** (potências de ligação) em vez de regras gramaticais explícitas por precedência.

### Conceito de Binding Power

Cada operador tem dois valores:

- **Left binding power (lbp)** — quão forte o operador puxa o token à sua esquerda
- **Right binding power (rbp)** — quão forte puxa o token à sua direita

A função `infix_binding_power(op)` retorna `(lbp, rbp, is_ternary)`:

| Operadores | lbp / rbp |
|---|---|
| `=`, `+=`, `-=`, ... (atribuição) | 1 / 1 (associativo à direita) |
| `\|\|` | 2 / 3 |
| `&&` | 4 / 5 |
| `\|` | 6 / 7 |
| `^` | 8 / 9 |
| `&` | 10 / 11 |
| `==`, `!=` | 12 / 13 |
| `<`, `>`, `<=`, `>=` | 14 / 15 |
| `<<`, `>>` | 16 / 17 |
| `+`, `-` | 18 / 19 |
| `*`, `/`, `%` | 20 / 21 |

Operadores unários prefix têm binding power fixo 30.

### Algoritmo `parse_expr(min_bp)`

```
lhs = parse_prefix_expr()

loop:
    1. tenta parse_postfix(lhs)  → se consumiu algo, volta ao topo
    2. lê operador infix atual
    3. busca (lbp, rbp) para o operador
    4. se lbp < min_bp → para (o operador pertence ao contexto acima)
    5. consome operador
    6. rhs = parse_expr(rbp)     → recursão com o rbp como novo mínimo
    7. constrói nó Binary / Assign / CompoundAssign
```

O `min_bp` cresce a cada nível recursivo, garantindo que operadores de menor precedência não "roubem" operandos de operadores de maior precedência.

**Exemplo:** `1 + 2 * 3`

```
parse_expr(0)
  lhs = Literal(1)
  operador '+' → lbp=18, rbp=19
  18 >= 0 → consome '+'
  rhs = parse_expr(19)
    lhs = Literal(2)
    operador '*' → lbp=20, rbp=21
    20 >= 19 → consome '*'
    rhs = parse_expr(21)
      lhs = Literal(3)
      sem mais operadores → retorna Literal(3)
    retorna Binary(2, Mul, 3)
  retorna Binary(1, Add, Binary(2, Mul, 3))
```

### Prefix Expressions

Parseadas por `parse_prefix_expr()` antes do loop Pratt:

| Token | Nó produzido |
|---|---|
| `IntLiteral(v)` | `Expr::Literal(Literal::Int(v))` |
| `FloatLiteral(v)` | `Expr::Literal(Literal::Double(v))` |
| `StringLiteral(v)` | `Expr::Literal(Literal::String(v))` |
| `CharLiteral(v)` | `Expr::Literal(Literal::Char(v))` |
| `Identifier(n)` | `Expr::Ident(n)` |
| `-`, `!`, `~`, `*`, `&` | `Expr::Unary(op, inner)` |
| `++`, `--` | `Expr::Prefix(op, inner)` |
| `(tipo)` | `Expr::Cast(qty, inner)` |
| `sizeof(tipo)` | `Expr::SizeofType(qty)` |
| `sizeof expr` | `Expr::Sizeof(inner)` |
| `(expr)` | agrupamento — retorna a expr interna |

**Distinção cast vs. agrupamento:** ao ver `(`, o parser faz lookahead para verificar se o próximo token começa um tipo (`starts_type()`). Se sim, trata como cast; caso contrário, como agrupamento.

**Distinção `sizeof(tipo)` vs. `sizeof expr`:** verifica se `(` é seguido de `starts_type()`. Se sim, consome tipo e fecha `)`; senão, parseia expressão normal.

### Postfix Expressions

`try_parse_postfix(lhs)` retorna `true` se consumiu algo:

| Token | Nó produzido |
|---|---|
| `++` | `Expr::Postfix(PostfixOp::Inc, lhs)` |
| `--` | `Expr::Postfix(PostfixOp::Dec, lhs)` |
| `[` | `Expr::Index(lhs, idx)` — consome `expr ]` |
| `(` | `Expr::Call(lhs, args)` — consome `args )` |
| `.` | `Expr::Member(lhs, Direct, campo)` |
| `->` | `Expr::Member(lhs, Pointer, campo)` |

Postfix tem maior precedência efetiva que qualquer infix — o loop de postfix é tentado antes do loop infix a cada iteração.

---

## AST — Nós Produzidos

### Decl

```
Decl::Function(retorno, nome, params, stmts, span)
Decl::GlobalVar(qty, nome, Option<init>, span)
Decl::StructDecl(nome, campos, span)
```

### Stmt

```
Stmt::Block(stmts, span)
Stmt::VarDecl(qty, nome, Option<init>, span)
Stmt::ExprStmt(expr, span)
Stmt::Return(Option<expr>, span)
Stmt::If(cond, then, Option<else>, span)
Stmt::While(cond, body, span)
Stmt::DoWhile(cond, body, span)
Stmt::For(Option<init>, Option<cond>, Option<inc>, body, span)
Stmt::Switch(expr, cases, span)
Stmt::Break(span)
Stmt::Continue(span)
```

### Expr

```
Expr::Literal(Literal, span)
Expr::Ident(String, span)
Expr::Binary(lhs, BinOp, rhs, span)
Expr::Unary(UnOp, inner, span)
Expr::Prefix(PrefixOp, inner, span)
Expr::Postfix(PostfixOp, inner, span)
Expr::Assign(lhs, rhs, span)
Expr::CompoundAssign(BinOp, lhs, rhs, span)
Expr::Call(callee, args, span)
Expr::Index(arr, idx, span)
Expr::Cast(QualifierType, inner, span)
Expr::Sizeof(inner, span)
Expr::SizeofType(QualifierType, span)
Expr::Ternary(cond, then, else, span)
Expr::Member(obj, MemberAccess, campo, span)
```

### Type

```
Type::Int | Long | Short | Char | Float | Double | Void
Type::Pointer(Box<Type>)
Type::Array(Box<Type>)      ← tamanho não armazenado
Type::Struct(String)        ← referência por nome

QualifierType {
    ty: Type,
    is_const: bool,
    is_unsigned: bool,
}
```
