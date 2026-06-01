# Lexer

## Visão Geral

O lexer transforma o código-fonte em uma sequência de tokens. A entrada é um `SourceFile` — que pode ser um arquivo em disco ou uma string (usado nos testes) — e a saída é `Vec<Token>`, mais um vetor de diagnósticos acumulados em caso de erros.

O ponto de entrada é `Scanner::scan()`, que processa o arquivo inteiro antes de retornar. Erros léxicos não interrompem o processo: são acumulados e o scanner continua tentando produzir mais tokens.

---

## Leitura Caracter a Caracter

O `SourceFile` expõe uma API de cursor:

- `peek()` — lê o próximo char sem avançar
- `peek_ahead()` — lê o char em +2 sem avançar (usado para lookahead duplo)
- `advance()` — consome o char atual e atualiza a posição
- `advance_if(predicate)` — consome somente se o predicado for verdadeiro

Internamente o `SourceFile` rastreia três campos:

| Campo | Descrição |
|---|---|
| `pos: usize` | offset em bytes no arquivo |
| `line: usize` | linha atual (começa em 1, incrementa a cada `\n`) |
| `col: usize` | coluna atual (começa em 1, reseta em `\n`) |

---

## Fluxo do Scanner

`scan()` é um loop que repete `next_token()` até o EOF. A cada iteração:

1. Espaços e comentários são descartados por `skip_whitespaces_and_comments()`
2. `token_start` é capturado — byte offset do início do token
3. O primeiro char é consumido e o scanner despacha para a regra correspondente:

```
char → handler
─────────────────────────────────
0–9         → lex_number()
" "         → lex_string()
' '         → lex_char()
a–z, A–Z, _ → lex_identifier()
operadores  → lex_operator()
delimitadores → push token simples + rastreia na delimiter_stack
outros      → emit_unknown()
```

---

## Rastreamento de Spans

Cada token armazena dois tipos de posição:

- `ByteSpan { start, end }` — offsets brutos em bytes (para fatiar o source)
- `line` e `col` — linha e coluna do início do token (para relatórios de erro)

O `Span` usado nos diagnósticos é derivado depois: `{ line, end_line, column_start, column_end }`.

---

## Comentários e Diretivas de Pré-processador

Tratados em `skip_whitespaces_and_comments()` antes de qualquer token ser produzido:

- `// ...` — consumido até `\n`
- `/* ... */` — consumido até `*/`; se o EOF chegar sem fechar, emite `UnclosedBlockComment`
- `# ...` — diretivas de pré-processador são consumidas até `\n` e **descartadas** completamente (nenhum token é produzido)

---

## Delimitadores Balanceados

O scanner mantém uma `delimiter_stack: Vec<(char, linha, coluna)>`. A cada `(`, `[` ou `{` empilha; a cada `)`, `]` ou `}` desempilha e verifica se o par bate.

| Situação | Erro emitido |
|---|---|
| Fechar sem abrir | `UnexpectedClosingDelimiter(char)` |
| EOF com pilha não vazia | `UnclosedDelimiter(char)` |

Os tokens ainda são produzidos normalmente — o erro é diagnóstico, não fatal.

---

## Literais Numéricos

A função `lex_number()` decide o formato pelo prefixo:

| Prefixo | Formato | Conversão |
|---|---|---|
| `0x` / `0X` | Hexadecimal | `i64::from_str_radix(&buf[2..], 16)` |
| `0` + dígitos | Octal | `i64::from_str_radix(&buf[1..], 8)` |
| Sem prefixo | Decimal | `buf.parse::<i64>()` |

Se um dígito octal inválido (8 ou 9) for encontrado, emite `InvalidOctalDigit(c)`.

**Ponto flutuante:** detectado pela presença de `.` ou `e`/`E` durante o consumo. A fração e o expoente (com `+`/`-` opcional) são acumulados na mesma string e convertidos com `buf.parse::<f64>()`.

**Sufixos** (`u`, `U`, `l`, `L` para inteiros; `f`, `F` para floats) são consumidos e descartados — o AST não distingue por sufixo neste estágio.

---

## Literais de String e Char

**String** (`lex_string()`):
- Delimitada por `"`
- Sequências de escape suportadas: `\n`, `\t`, `\r`, `\\`, `\"`, `\'`, `\0`
- Se encontrar `\n` ou EOF antes do `"` de fechamento → `UnterminatedLiteral`
- Armazenada já com escapes resolvidos: `StringLiteral(String)`

**Char** (`lex_char()`):
- Delimitado por `'`
- Aceita um único char ou uma escape sequence
- Armazenado como `CharLiteral(char)`

---

## Identificadores e Keywords

`lex_identifier()` consome enquanto o char satisfazer `is_ident_continue()` (letras, dígitos, `_`). Em seguida consulta `lookup_keyword()`:

- Se a string for uma keyword conhecida → token de keyword (`If`, `Int`, `While`, etc.)
- Caso contrário → `Identifier(String)`

Keywords reconhecidas: `if`, `else`, `while`, `for`, `do`, `switch`, `case`, `default`, `break`, `continue`, `return`, `int`, `char`, `float`, `double`, `void`, `struct`, `enum`, `union`, `typedef`, `const`, `static`, `extern`, `auto`, `register`, `signed`, `unsigned`, `short`, `long`, `volatile`, `inline`, `sizeof`.

---

## Operadores (Lookahead de 1)

`lex_operator()` usa `peek()` para decidir entre tokens de 1 e 2 (ou 3) chars:

```
'+'  → peek '+'  → PlusPlus
     → peek '='  → PlusEqual
     → PlusPlus  → Plus

'-'  → peek '-'  → MinusMinus
     → peek '='  → MinusEqual
     → peek '>'  → Arrow
     → Minus

'<'  → peek '<'  → peek '='  → LessLessEqual
               → LessLess
     → peek '='  → LessEqual
     → Less
```

O mesmo padrão se aplica a `>`, `=`, `!`, `&`, `|`, `^`, `*`, `/`, `%`.

---

## Categorias de Tokens

```
TokenKind
├── Keywords de controle: If, Else, While, For, Do, Switch, Case, Default,
│                         Break, Continue, Return
├── Keywords de tipo:     Int, Long, Short, Char, Float, Double, Void, Struct,
│                         Enum, Union, Unsigned, Signed, Const, Typedef,
│                         Static, Extern, Auto, Register, Volatile, Inline
├── Operadores:           Plus, Minus, Star, Slash, Percent, PlusPlus,
│                         MinusMinus, PlusEqual, MinusEqual, ... (compostos)
│                         EqualEqual, BangEqual, Less, Greater, LessEqual,
│                         GreaterEqual, AndAnd, OrOr, Bang, Ampersand, Pipe,
│                         Caret, Tilde, LessLess, GreaterGreater, Arrow,
│                         Sizeof, Question, ...
├── Delimitadores:        LeftParen, RightParen, LeftBrace, RightBrace,
│                         LeftBracket, RightBracket, Semicolon, Comma,
│                         Colon, Dot
├── Literais:             IntLiteral(i64), FloatLiteral(f64),
│                         StringLiteral(String), CharLiteral(char)
├── Identificador:        Identifier(String)
├── Erro:                 Unknown(char)
└── Fim:                  Eof
```

---

## Erros Léxicos

Todos são do tipo `CompilerError::Lexical(LexicalError { span, kind })`:

| Kind | Causa |
|---|---|
| `InvalidChar(char)` | Char que não inicia nenhum token válido |
| `UnclosedBlockComment` | EOF dentro de `/* ... */` |
| `UnclosedDelimiter(char)` | EOF com `(`, `[` ou `{` sem fechar |
| `UnexpectedClosingDelimiter(char)` | `)`, `]` ou `}` sem correspondente |
| `UnterminatedLiteral(String)` | String ou char sem fechar |
| `InvalidOctalDigit(char)` | Dígito 8 ou 9 em literal octal |
