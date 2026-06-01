# Análise Sintática

**Status:** Concluída

Consome a sequência de tokens produzida pelo lexer e constrói a Árvore Sintática Abstrata (AST).
A entrada é `Vec<Token>` e a saída é `Program` (uma lista de declarações globais) mais um vetor de diagnósticos.

---

## Estrutura do Parser

```rust
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<CompilerError>,
    type_names: HashSet<String>,   // aliases de typedef registrados
}
```

| Método | Comportamento |
|---|---|
| `peek()` | Token atual sem avançar |
| `peek_at(n)` | Token em `pos + n` (lookahead arbitrário) |
| `advance()` | Consome token atual |
| `expect(kind, msg)` | Consome ou emite `SyntaxError` |
| `starts_type()` | Retorna `true` se o token atual inicia um tipo |

---

## Recuperação de Erros

Quando `parse_global_item()` falha, o erro é acumulado em `diagnostics` e `synchronize()` é chamado.
A sincronização descarta tokens até encontrar `;` ou `}`, depois continua o próximo item global.
Isso permite que um arquivo com múltiplos erros produza todos os diagnósticos de uma vez.

---

## Hierarquia de Parse

```
parse_program()
└── parse_global_item()          ← loop até EOF
    ├── parse_struct_decl()      ← struct Nome {
    ├── parse_enum_decl()        ← enum Nome {
    ├── parse_typedef_decl()     ← typedef tipo Alias;
    ├── parse_function_decl()    ← tipo nome (
    └── parse_global_var_decl()  ← variável global

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

O dispatcher usa lookahead para decidir o tipo de declaração sem consumir tokens:

```
peek[0] == Struct && peek[1] == Ident && peek[2] == {   →  parse_struct_decl()
peek[0] == Enum   && peek[1] == Ident && peek[2] == {   →  parse_enum_decl()
peek[0] == Typedef                                       →  parse_typedef_decl()
senão: parse_type() + nome
  peek == (   →  parse_function_decl()
  senão       →  parse_global_var_decl()
```

### Struct

```c
struct Ponto {
    int x;
    int y;
};
```

Produz `Decl::StructDecl(nome, Vec<(QualifierType, String)>, span)`.

### Enum

```c
enum Cor { VERMELHO, VERDE = 10, AZUL };
```

Produz `Decl::EnumDecl(nome, Vec<(String, Option<Expr>)>, span)`.

### Typedef

```c
typedef unsigned int uint32_t;
```

Produz `Decl::Typedef(QualifierType, alias_nome, span)`.
Após o parse, o alias é registrado em `type_names` para que expressões como `(uint32_t)x` sejam reconhecidas como cast.

### Função

```c
int soma(int a, int b) { return a + b; }
```

Produz `Decl::Function(retorno, nome, params, stmts, span)`.

---

## Tipos (`parse_type`)

O parse de tipos segue três etapas:

1. **Qualificadores** — `const`, `unsigned` em qualquer ordem
2. **Tipo base** — `int`, `long`, `short`, `char`, `float`, `double`, `void`, `struct Nome`, `enum Nome`, ou um alias de typedef
3. **Ponteiros** — loop que consome `*`, envolvendo o tipo em `Pointer(T)`

```
const unsigned int **  →  QualifierType {
                              ty: Pointer(Pointer(Int)),
                              is_const: true, is_unsigned: true
                          }
```

**Sufixo de array** (`parse_array_suffix`) — chamado após o identificador, não após o tipo:

```c
int arr[10][20]
//      ^^^^^^^^^  envolve em Array(Array(Int))
```

---

## Statements

| Token | Statement produzido |
|---|---|
| `{` | `Stmt::Block(stmts)` |
| `return` | `Stmt::Return(Option<Expr>)` |
| `break` | `Stmt::Break` |
| `continue` | `Stmt::Continue` |
| `if` | `Stmt::If(cond, then, Option<else>)` |
| `while` | `Stmt::While(cond, body)` |
| `do` | `Stmt::DoWhile(cond, body)` |
| `for` | `Stmt::For(Option<init>, Option<cond>, Option<inc>, body)` |
| `switch` | `Stmt::Switch(expr, Vec<SwitchCase>)` |
| `starts_type()` | `Stmt::VarDecl(qty, nome, Option<init>)` |
| qualquer outro | `Stmt::ExprStmt(expr)` |

O `for` aceita declaração de variável no `init`:

```c
for (int i = 0; i < n; i++) { ... }
```

---

## Expressões — Pratt Parser

O Pratt parser controla precedência via **binding powers** sem criar regras gramaticais por nível.

### Tabela de binding powers

| Operadores | lbp / rbp |
|---|---|
| `=` `+=` `-=` `*=` `/=` `%=` `&=` `\|=` `^=` `<<=` `>>=` | 1 / 1 (direita) |
| `\|\|` | 4 / 5 |
| `&&` | 6 / 7 |
| `\|` | 8 / 9 |
| `^` | 10 / 11 |
| `&` | 12 / 13 |
| `==` `!=` | 14 / 15 |
| `<` `<=` `>` `>=` | 16 / 17 |
| `<<` `>>` | 18 / 19 |
| `+` `-` | 20 / 21 |
| `*` `/` `%` | 22 / 23 |
| prefix unário, cast | — / 30 |
| postfix `++` `--` `[]` `()` `.` `->` | > 30 |

### Algoritmo

```
parse_expr(min_bp):
  lhs = parse_prefix_expr()
  loop:
    se try_parse_postfix(lhs) → continua
    op = peek()
    (lbp, rbp) = infix_binding_power(op)
    se lbp < min_bp → para
    consome op
    rhs = parse_expr(rbp)
    lhs = Binary/Assign/CompoundAssign(lhs, op, rhs)
  retorna lhs
```

**Exemplo:** `1 + 2 * 3` produz `Binary(1, Add, Binary(2, Mul, 3))` porque `*` tem lbp=22 > rbp=19 de `+`.

### Expressões prefixas

| Token | Nó AST |
|---|---|
| `IntLiteral(v)` | `Literal::Int(v)` |
| `FloatLiteral(v)` | `Literal::Double(v)` |
| `Identifier(n)` | `Expr::Ident(n)` |
| `-` `!` `~` `*` `&` | `Expr::Unary(op, inner)` |
| `++` `--` | `Expr::Prefix(op, inner)` |
| `(tipo) expr` | `Expr::Cast(qty, inner)` |
| `sizeof(tipo)` | `Expr::SizeofType(qty)` |
| `sizeof expr` | `Expr::Sizeof(inner)` |
| `(expr)` | agrupamento |

A distinção cast vs. agrupamento é feita por lookahead: `(` seguido de `starts_type()` → cast.

### Expressões postfixas

| Token | Nó AST |
|---|---|
| `++` `--` | `Expr::Postfix(op, lhs)` |
| `[expr]` | `Expr::Index(lhs, idx)` |
| `(args)` | `Expr::Call(lhs, args)` |
| `.nome` | `Expr::Member(lhs, Direct, campo)` |
| `->nome` | `Expr::Member(lhs, Pointer, campo)` |

---

## AST — Nós Produzidos

### Decl

```
Decl::Function(retorno, nome, params, stmts, span)
Decl::GlobalVar(qty, nome, Option<init>, span)
Decl::StructDecl(nome, campos, span)
Decl::EnumDecl(nome, Vec<(String, Option<Expr>)>, span)
Decl::Typedef(qty, alias_nome, span)
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
Stmt::Switch(expr, Vec<SwitchCase>, span)
Stmt::Break(span)
Stmt::Continue(span)
```

### Expr

```
Expr::Literal(Literal, span)       // Int(i64), Double(f64), Char(char), String(String)
Expr::Ident(String, span)
Expr::Binary(lhs, BinOp, rhs, span)
Expr::Unary(UnOp, inner, span)     // -, !, ~, *, &
Expr::Prefix(PrefixOp, inner, span) // ++, -- prefixo
Expr::Postfix(PostfixOp, inner, span) // ++, -- sufixo
Expr::Assign(lhs, rhs, span)
Expr::CompoundAssign(BinOp, lhs, rhs, span) // +=, -=, ...
Expr::Call(callee, args, span)
Expr::Index(arr, idx, span)
Expr::Cast(QualifierType, inner, span)
Expr::Sizeof(inner, span)
Expr::SizeofType(QualifierType, span)
Expr::Ternary(cond, then, else, span)
Expr::Member(obj, MemberAccess, campo, span) // Direct (.) | Pointer (->)
```

### Type

```
Type::Int | Long | Short | Char | Float | Double | Void
Type::Pointer(Box<Type>)
Type::Array(Box<Type>)     ← tamanho não armazenado
Type::Struct(String)       ← referência por nome
Type::Enum(String)         ← referência por nome
Type::Alias(String)        ← nome de typedef; resolvido pelo analisador semântico

QualifierType { ty: Type, is_const: bool, is_unsigned: bool }
```
