# Análise Léxica

**Status:** Concluída

A análise léxica (ou *scanning*) é a primeira fase do compilador. Lê o código-fonte caractere
por caractere e agrupa sequências significativas em unidades chamadas **tokens**.

Dado o trecho:

```c
int x = 42 + y;
```

O scanner produz:

| Lexema | Token | Categoria |
|--------|-------|-----------|
| `int` | `Int` | Palavra-chave |
| `x` | `Identifier("x")` | Identificador |
| `=` | `Equal` | Operador |
| `42` | `IntLiteral(42)` | Literal |
| `+` | `Plus` | Operador |
| `y` | `Identifier("y")` | Identificador |
| `;` | `Semicolon` | Pontuação |

---

## Arquitetura do Scanner

O scanner é implementado na struct `Scanner` em `src/lexer/scanner.rs`:

```rust
pub struct Scanner {
    pub src: SourceFile,
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<CompilerError>,
    delimiter_stack: Vec<(char, usize, usize)>,
}
```

### SourceFile e Memory Mapping

A leitura do fonte é feita por `SourceFile` (`src/common/input/source.rs`) via **memory mapping**
(crate `memmap2`), evitando cópias desnecessárias de bytes. Em testes, aceita strings diretas:

```rust
let src = SourceFile::from_path(PathBuf::from("main.c"))?;
let src = SourceFile::from_string("int x = 42;"); // usado em testes
```

O `SourceFile` rastreia três valores de posição:

- `pos` — offset de bytes no buffer UTF-8
- `line` — linha atual (começa em 1)
- `col` — coluna atual (começa em 1)

### ByteSpan e Token

Cada token carrega um `ByteSpan` com os offsets de início e fim, além de `line` e `col`:

```rust
pub struct ByteSpan { pub start: usize, pub end: usize }

pub struct Token {
    pub kind: TokenKind,
    pub span: ByteSpan,
    pub line: usize,
    pub col: usize,
}
```

!!! note "Fluxograma"
    *(Reservado — inserir diagrama das relações entre Scanner, SourceFile, Token e ByteSpan)*

---

## Tipos de Token

Todos os tipos são definidos em `src/lexer/tokens/token_kind.rs`.

### Palavras-chave

| Categoria | Tokens |
|-----------|--------|
| Controle de fluxo | `if` `else` `while` `for` `do` `switch` `case` `default` `break` `continue` `return` |
| Tipos | `int` `char` `float` `double` `void` `struct` `enum` `union` |
| Qualificadores / storage | `const` `static` `extern` `auto` `register` `volatile` `inline` `typedef` `signed` `unsigned` `short` `long` `sizeof` |

### Operadores

| Token | Lexema | Descrição |
|-------|--------|-----------|
| `Plus` | `+` | Adição |
| `PlusPlus` | `++` | Incremento |
| `PlusEqual` | `+=` | Atribuição com adição |
| `Minus` | `-` | Subtração |
| `MinusMinus` | `--` | Decremento |
| `MinusEqual` | `-=` | Atribuição com subtração |
| `Arrow` | `->` | Acesso via ponteiro |
| `Star` | `*` | Multiplicação / dereferência |
| `StarEqual` | `*=` | Atribuição com multiplicação |
| `Slash` | `/` | Divisão |
| `SlashEqual` | `/=` | Atribuição com divisão |
| `Equal` | `=` | Atribuição |
| `EqualEqual` | `==` | Igualdade |
| `BangEqual` | `!=` | Diferença |
| `Less` | `<` | Menor que |
| `LessEqual` | `<=` | Menor ou igual |
| `LessLess` | `<<` | Shift left |
| `LessLessEqual` | `<<=` | Shift left + atribuição |
| `Greater` | `>` | Maior que |
| `GreaterEqual` | `>=` | Maior ou igual |
| `GreaterGreater` | `>>` | Shift right |
| `GreaterGreaterEqual` | `>>=` | Shift right + atribuição |
| `Bang` | `!` | Negação lógica |
| `AndAnd` | `&&` | E lógico |
| `OrOr` | `\|\|` | Ou lógico |
| `Ampersand` | `&` | E bit a bit / endereço |
| `Pipe` | `\|` | Ou bit a bit |
| `Caret` | `^` | XOR bit a bit |
| `Tilde` | `~` | Complemento bit a bit |
| `Percent` | `%` | Módulo |
| `Dot` | `.` | Acesso a membro |

### Delimitadores e Pontuação

| Token | Lexema |
|-------|--------|
| `LeftParen` / `RightParen` | `(` `)` |
| `LeftBrace` / `RightBrace` | `{` `}` |
| `LeftBracket` / `RightBracket` | `[` `]` |
| `Semicolon` | `;` |
| `Comma` | `,` |
| `Colon` | `:` |

### Tokens de valor

| Token | Valor carregado | Exemplos |
|-------|----------------|----------|
| `IntLiteral(i64)` | Inteiro parsed | `42`, `0xFF`, `0755` |
| `FloatLiteral(f64)` | Float parsed | `3.14`, `1e-10` |
| `StringLiteral(String)` | Conteúdo sem aspas | `"hello\n"` |
| `CharLiteral(char)` | Char Rust | `'A'`, `'\n'` |
| `Identifier(String)` | Nome do identificador | `foo`, `_count` |
| `Unknown(char)` | Char inválido | `@`, `$` |
| `Eof` | — | Fim do arquivo |

---

## Processo de Varredura

O método `scan()` é o ponto de entrada. Executa um loop alternando entre pular
espaços/comentários e reconhecer o próximo token:

```rust
pub fn scan(&mut self) -> &[Token] {
    while !self.src.is_at_end() {
        self.skip_whitespaces_and_comments();
        if self.src.is_at_end() { break; }
        self.next_token();
    }

    // Delimitadores ainda na pilha = nunca foram fechados
    let unclosed = self.delimiter_stack.drain(..).collect::<Vec<_>>();
    for (c, line, col) in unclosed {
        self.diagnostics.push(/* UnclosedDelimiter(c) */);
    }

    self.emit_at(TokenKind::Eof, "", self.src.line(), self.src.col());
    &self.tokens
}
```

`next_token()` lê um caractere e despacha pelo `match`:

```rust
fn next_token(&mut self) {
    let (line, col) = (self.src.line(), self.src.col());
    let c = self.src.advance().unwrap();

    match c {
        '0'..='9'              => self.lex_number(c, line, col),
        '"'                    => self.lex_string(line, col),
        '\''                   => self.lex_char(line, col),
        c if is_ident_start(c) => self.lex_identifier(c, line, col),
        '(' | '[' | '{'        => { /* empilha + emite */ }
        ')' | ']' | '}'        => { /* desempilha + emite */ }
        '+' | '-' | '*' | '/' | '=' | '!' | '<' | '>' | '&' | '|'
                               => self.lex_operator(c, line, col),
        ';' | ',' | ':' | '.' | '%' | '^' | '~'
                               => self.emit_at(/* token simples */),
        c                      => self.emit_unknown(c, line, col),
    }
}
```

!!! note "Fluxograma"
    *(Reservado — inserir fluxograma do loop scan() → skip_whitespace() → next_token() → dispatch)*

---

## Espaços em Branco e Comentários

`skip_whitespaces_and_comments()` consome silenciosamente quatro categorias antes de cada token:

- **Espaços** — `' '`, `'\t'`, `'\r'`, `'\n'`
- **Comentários de linha** — `//` até `\n`
- **Comentários de bloco** — `/* ... */`; se chegar ao EOF sem fechar, emite `UnclosedBlockComment`
- **Diretivas de pré-processador** — linhas com `#` são ignoradas integralmente

```rust
fn skip_whitespaces_and_comments(&mut self) {
    loop {
        while matches!(self.src.peek(), Some(' '|'\t'|'\r'|'\n')) {
            self.src.advance();
        }
        if self.src.peek() == Some('/') && self.src.peek_ahead() == Some('/') {
            while !matches!(self.src.peek(), Some('\n') | None) { self.src.advance(); }
            continue;
        }
        if self.src.peek() == Some('/') && self.src.peek_ahead() == Some('*') {
            // consome até '*/' ou EOF com diagnóstico
            continue;
        }
        if self.src.peek() == Some('#') {
            while !matches!(self.src.peek(), Some('\n') | None) { self.src.advance(); }
            continue;
        }
        break;
    }
}
```

---

## Identificadores e Palavras-chave

Um identificador começa com letra ou `_` (`is_ident_start`), seguido de letras, dígitos ou `_`
(`is_ident_continue`). Ao final, `lookup_keyword()` decide se é keyword ou `Identifier`:

```rust
fn lex_identifier(&mut self, first: char, line: usize, col: usize) {
    let mut buf = String::from(first);
    while let Some(c) = self.src.peek() {
        if is_ident_continue(c) { buf.push(c); self.src.advance(); } else { break; }
    }
    let kind = lookup_keyword(&buf).unwrap_or(TokenKind::Identifier(buf.clone()));
    self.emit_at(kind, &buf, line, col);
}
```

!!! tip "Design"
    Cada keyword tem sua própria variante no enum `TokenKind` (ex: `TokenKind::If`).
    Isso elimina comparações de string no parser — o `match arm` já é o tipo exato.

---

## Literais

Implementado em `src/lexer/rules/literals.rs`.

### Inteiras

| Prefixo | Base | Exemplo | Valor |
|---------|------|---------|-------|
| `0x` / `0X` | Hexadecimal (16) | `0xFF` | 255 |
| `0` + dígito octal | Octal (8) | `0755` | 493 |
| nenhum | Decimal (10) | `42` | 42 |

### Float

Se após os dígitos decimais o scanner encontrar `.`, `e` ou `E`, a literal é tratada como float.
Parte fracionária e expoente (com sinal opcional) são consumidos e parseados como `f64`.

| Forma | Exemplo | Token |
|-------|---------|-------|
| Inteiro | `100` | `IntLiteral(100)` |
| Com ponto | `3.14` | `FloatLiteral(3.14)` |
| Com expoente | `1e10` | `FloatLiteral(1e10)` |
| Expoente com sinal | `1.5e-3` | `FloatLiteral(0.0015)` |

### String e Char

Strings delimitadas por `"..."`, chars por `'...'`. Ambos suportam sequências de escape
(`\n`, `\t`, `\\`, etc.) resolvidas por `resolve_escape()`. Se o arquivo terminar antes
do fechamento, emite `UnterminatedLiteral`.

!!! warning "Limitação"
    A versão atual não valida que a literal de char contém exatamente um caractere.
    Chars multi-char como `'ab'` serão tratados na análise semântica.

---

## Operadores

Implementado em `src/lexer/rules/operators.rs`. Usa **lookahead de um caractere** para
distinguir operadores simples de compostos:

```rust
'+' => match self.src.peek() {
    Some('+') => { self.src.advance(); self.emit_at(TokenKind::PlusPlus,  "++", ..); }
    Some('=') => { self.src.advance(); self.emit_at(TokenKind::PlusEqual, "+=", ..); }
    _         =>                       self.emit_at(TokenKind::Plus,       "+",  ..),
},

// Operadores de shift precisam de lookahead duplo
'<' => match self.src.peek() {
    Some('=') => { /* LessEqual */ }
    Some('<') => {
        self.src.advance();
        if self.src.peek() == Some('=') {
            self.src.advance(); // LessLessEqual <<=
        } else {
            // LessLess <<
        }
    }
    _ => { /* Less */ }
},
```

!!! note "Comentários e `/`"
    `//` e `/*` são consumidos em `skip_whitespaces_and_comments()` **antes** de
    `next_token()`. Quando `lex_operator('/')` é chamado, só chegam `/` isolado ou `/=`.

!!! note "Fluxograma"
    *(Reservado — inserir árvore de decisão do lookahead por operador)*

---

## Delimitadores e Pareamento

O scanner rastreia o pareamento de `( ) [ ] { }` com uma **pilha** interna durante a própria
varredura léxica:

- Abertura → empilha `(char, linha, coluna)`
- Fechamento → verifica o topo; se bater, desempilha; se não, emite `UnexpectedClosingDelimiter`
- Fim do arquivo → entradas restantes viram `UnclosedDelimiter`

```rust
'(' => {
    self.delimiter_stack.push(('(', line, col));
    self.emit_at(TokenKind::LeftParen, "(", line, col);
}
')' => {
    if matches!(self.delimiter_stack.last(), Some(('(', _, _))) {
        self.delimiter_stack.pop();
    } else {
        self.emit_unexpected_delimiter(')', line, col);
    }
    self.emit_at(TokenKind::RightParen, ")", line, col);
}
```

!!! note "Por que no léxico?"
    Detectar pareamento no léxico permite mensagens de erro precisas: sabemos exatamente
    a linha e coluna onde o delimitador foi aberto.

---

## Tratamento de Erros

Erros são coletados em `scanner.diagnostics` sem interromper a varredura.

| Variante | Situação | Exemplo |
|----------|----------|---------|
| `InvalidChar(char)` | Char não reconhecido | `@`, `$` |
| `UnclosedBlockComment` | `/*` sem `*/` antes do EOF | `/* comentário...` |
| `UnclosedDelimiter(char)` | Abertura sem fechamento | `(foo + bar` |
| `UnexpectedClosingDelimiter(char)` | Fechamento sem abertura | `x + y)` |
| `UnterminatedLiteral(String)` | String/char sem fechamento | `"hello` |

Cada erro é convertido em um `Report` estruturado via o trait `ToReport`:

```
error: unclosed delimiter
  --> main.c:3:5
   |
 3 |     (x + y
   |     ^ '(' nao foi fechado
   |
   = help: Adicione o delimitador de fechamento correspondente.
```

!!! note "Fluxograma"
    *(Reservado — inserir diagrama de tratamento e recuperação de erros léxicos)*
