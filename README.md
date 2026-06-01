# Crusty — Compilador C em Rust

Projeto da disciplina de Compiladores 1. Implementa um compilador para um subconjunto da linguagem C, escrito em Rust.

## Estágio atual

| Fase | Status |
|------|--------|
| Análise léxica | Completo |
| Análise sintática | Completo |
| Análise semântica | Em desenvolvimento |
| Geração de código | Não iniciado |

## Estrutura do projeto

```
src/
├── lexer/       Análise léxica — transforma código-fonte em tokens
├── parser/      Análise sintática — constrói a AST via Pratt parsing
├── analyser/    Análise semântica — tabela de símbolos, escopos, verificação de tipos
├── codegen/     Geração de código — esqueleto (não implementado)
├── common/      Estruturas compartilhadas: AST, erros, spans, utilitários
└── tests/       Testes unitários por módulo
```

## Pré-requisitos

- [Rust](https://rustup.rs/) 1.70+

```bash
rustup update stable
```

## Build

```bash
cargo build
```

## Uso

Rodar o compilador sobre um arquivo de entrada:

```bash
cargo run -- <arquivo>
```

Exemplo:

```bash
cargo run -- input.c
```

O compilador imprime os tokens reconhecidos, a AST e eventuais diagnósticos de erro.

> O modo REPL interativo (sem argumentos) ainda não está implementado.

## Funcionalidades implementadas

**Lexer**
- Literais inteiros (decimal, octal, hexadecimal), float, string e char
- Todos os operadores C: aritméticos, relacionais, lógicos, bitwise, atribuição composta, `->`, `++`, `--`
- 30+ palavras-chave: `if`, `while`, `for`, `do`, `switch`, `typedef`, `enum`, `struct`, `sizeof`, etc.
- Rastreamento de spans (linha/coluna) para mensagens de erro precisas
- Recuperação de erros (continua após erro léxico)

**Parser (Pratt)**
- Declarações globais: função, variável, struct, enum, typedef
- Statements: `if`/`else`, `while`, `do-while`, `for`, `switch`/`case`, `return`, `break`, `continue`
- Expressões com precedência completa: cast, `sizeof`, ternário, acesso a membro (`.`, `->`)
- Recuperação de erros (sincroniza em `;` ou `}`)

**Analisador Semântico**
- Tabela de símbolos com escopos aninhados
- Detecção de variáveis não declaradas e redeclarações
- Registro de structs e resolução de campos (`.`, `->`)
- Registro e resolução de aliases de typedef
- Registro de variantes de enum como constantes `int`
- Inferência de tipo para expressões
- Verificação de tipo em atribuições e operações binárias
- Promoção numérica implícita (Double > Float > Long > Int)
- Detecção de atribuição a `const`

## Testes

### Todos os testes unitários

```bash
cargo test
```

### Filtrar por módulo

```bash
cargo test lexical        # testes do scanner/lexer (21 casos)
cargo test parser_test    # testes do parser / AST (76 casos)
cargo test semantic_test  # testes do analisador semântico (21 casos)
cargo test symbol_test    # testes da tabela de símbolos (11 casos)
cargo test analyzer_test  # testes de integração do analisador (3 casos)
cargo test source         # testes de SourceFile e spans (12 casos)
cargo test lexer_file     # testes do scanner lendo arquivos (7 casos)
cargo test parser_file    # testes do parser lendo arquivos (4 casos)
cargo test literals       # testes de literais numéricos (4 casos)
cargo test ast_errors     # testes de erros de AST (4 casos)
cargo test token          # testes de tokens individuais (2 casos)
```

### Com saída detalhada

```bash
cargo test -- --nocapture
```

### Módulos de teste

| Arquivo | Cobertura | Testes |
|---|---|---|
| `src/tests/lexical_test.rs` | Scanner: operadores, palavras-chave, literais | 21 |
| `src/tests/parser_test.rs` | Parser / construção de AST | 76 |
| `src/tests/semantic_test.rs` | Verificação de tipos, undefined vars, const | 21 |
| `src/tests/symbol_test.rs` | Tabela de símbolos, escopos, redeclaração | 11 |
| `src/tests/source_test.rs` | `SourceFile`, `ByteSpan`, posicionamento | 12 |
| `src/tests/lexer_file_test.rs` | Scanner sobre arquivos reais | 7 |
| `src/tests/parser_file_test.rs` | Parser sobre arquivos reais | 4 |
| `src/tests/literals_test.rs` | Literais inteiros, floats, strings | 4 |
| `src/tests/ast_errors.rs` | Diagnósticos e erros de AST | 4 |
| `src/tests/analyzer_test.rs` | Integração léxico → sintático → semântico | 3 |
| `src/tests/token_test.rs` | `Token` e `TokenKind` | 2 |

**Total: 165 testes**

## Documentação técnica

- [Lexer](docs/lexer.md) — scanner, tokens, erros léxicos
- [Parser](docs/parser.md) — Pratt parser, AST, recuperação de erros
- [Analisador Semântico](docs/semantic.md) — tabela de símbolos, verificação de tipos
- [Precedência de Operadores C](docs/c_operator_precedence.md) — tabela C11 e mapeamento para binding powers

## Contribuidores

| Nome | GitHub / contato |
|---|---|
| Lucas Andrade Zanetti | [@Bappoz](https://github.com/Bappoz) |
| Gustavo Xavier | [@guxvr](https://github.com/guxvr) |
| Hugo Freitas Silva | [@HugoFreitass](https://github.com/HugoFreitass) |
| Matheus Lemes | [@matheuslemesam](https://github.com/matheuslemesam) |
| Philipe Caetano | philipe2015amancio@hotmail.com |
