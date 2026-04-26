# Crusty — Compilador C em Rust

Projeto da disciplina de Compiladores 1. Implementa um compilador para um subconjunto da linguagem C, escrito em Rust.

## Estrutura do projeto

```
src/
├── lexer/       Análise léxica — transforma código-fonte em tokens
├── parser/      Análise sintática — constrói a AST via Pratt parsing
├── analyser/    Análise semântica — verificação de tipos e escopo (em desenvolvimento)
├── codegen/     Geração de código — LLVM IR / Assembly (em desenvolvimento)
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

O compilador imprime os tokens reconhecidos e eventuais diagnósticos de erro.

> O modo REPL interativo (sem argumentos) ainda não está implementado.

## Testes

### Todos os testes unitários

```bash
cargo test
```

### Filtrar por módulo

```bash
cargo test lexical      # testes do scanner/lexer (21 casos)
cargo test parser       # testes do parser / AST (8 casos)
cargo test literals     # testes de literais numéricos (4 casos)
cargo test token        # testes de tokens individuais (2 casos)
cargo test source       # testes de SourceFile e spans (12 casos)
cargo test lexer_file   # testes do scanner lendo arquivos (7 casos)
cargo test ast_errors   # testes de erros de AST (4 casos)
```

### Com saída detalhada

```bash
cargo test -- --nocapture
```

### Módulos de teste

| Arquivo                       | Cobertura                                      | Testes |
|-------------------------------|------------------------------------------------|--------|
| `src/tests/lexical_test.rs`   | Scanner: operadores, palavras-chave, literais  | 21     |
| `src/tests/source_test.rs`    | `SourceFile`, `ByteSpan`, posicionamento       | 12     |
| `src/tests/lexer_file_test.rs`| Scanner sobre arquivos reais                   | 7      |
| `src/tests/parser_test.rs`    | Parser / construção de AST                     | 8      |
| `src/tests/literals_test.rs`  | Literais inteiros, floats, strings             | 4      |
| `src/tests/ast_errors.rs`     | Diagnósticos e erros de AST                    | 4      |
| `src/tests/token_test.rs`     | `Token` e `TokenKind`                          | 2      |

## Contribuidores

| Nome                  | GitHub / contato                    |
|-----------------------|-------------------------------------|
| Lucas Andrade Zanetti | [@Bappoz](https://github.com/Bappoz) |
| Gustavo               | [@guxvr](https://github.com/guxvr)  |
| Hugo Freitas Silva    | [@HugoFreitass](https://github.com/HugoFreitass) |
| Matheus Lemes         | [@matheuslemesam](https://github.com/matheuslemesam) |
| Philipe Caetano       | philipe2015amancio@hotmail.com      |
