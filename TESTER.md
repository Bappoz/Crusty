# TESTER — como testar o Crusty

Este documento cobre todas as formas de testar o compilador: testes unitários, testes de integração com arquivos `.c` reais, e testes de smoke ponta a ponta que montam/linkam/executam o binário gerado via `gcc`.

Pré-requisito: ambiente configurado conforme [INSTALL.md](INSTALL.md) (Rust + `gcc`).

## Visão geral das suítes

| Suíte | Localização | O que verifica | Testes |
|---|---|---|---|
| Testes unitários da lib | `src/tests/*.rs` | Lexer, parser, analisador semântico, codegen, em isolamento | 295 |
| Testes unitários do binário | `src/main.rs` (`#[cfg(test)]`) | Parsing de flags da CLI | 10 |
| `tests/integration_test.rs` | Pipeline lexer→parser→semântico sobre arquivos `.c` reais (válidos e inválidos) | 16 |
| `tests/codegen_smoke.rs` | TAC montado manualmente → assembly → `gcc` → execução, checando exit code | 5 |
| `tests/exe_smoke_test.rs` | Código-fonte C real → pipeline completo → executável ELF → execução | 26 |
| `tests/double_codegen_test.rs` | Codegen de `double`/XMM (assembly emitido e execução real, issue #172) | 7 |
| `tests/licm_test.rs` | Otimização de loop-invariant code motion | 2 |

Total atual: **361 testes**, todos passando em `developer`.

## Rodando tudo

```bash
cargo test --all
```

Isso compila e roda todas as suítes acima, na ordem mostrada por `cargo`.

## Testes unitários (por módulo)

Os testes unitários vivem em `src/tests/` e cobrem cada fase do compilador isoladamente, sem precisar de arquivos externos nem de `gcc`.

```bash
cargo test --lib                # só a suíte unitária da lib (sem integração/smoke)
cargo test lexical               # scanner/lexer: operadores, palavras-chave, literais
cargo test parser_test           # parser / construção de AST
cargo test semantic_test         # verificação de tipos, undefined vars, const
cargo test symbol_test           # tabela de símbolos, escopos, redeclaração
cargo test source                # SourceFile, ByteSpan, posicionamento
cargo test lexer_file            # scanner sobre arquivos reais
cargo test parser_file           # parser sobre arquivos reais
cargo test literals              # literais inteiros, floats, strings
cargo test ast_errors            # diagnósticos e erros de AST
cargo test analyzer_test         # integração léxico → sintático → semântico
cargo test token                 # Token e TokenKind
cargo test codegen_test          # geração de código (unitário)
cargo test peephole_test         # otimizador de assembly (peephole)
cargo test unmap_safe_test       # segurança de unmap/memmap do SourceFile
```

Saída detalhada (não suprime `println!`/`eprintln!` dos testes):

```bash
cargo test -- --nocapture
```

Rodar um único teste pelo nome exato:

```bash
cargo test test_licm_loop_with_invariant
```

## Testes com arquivos `.c` reais

### `tests/integration_test.rs` — front-end completo, sem executar binário

Roda lexer → parser → análise semântica sobre arquivos em `tests/integration/valid/` e `tests/integration/invalid/`, verificando que programas válidos não geram diagnósticos e que programas inválidos geram exatamente o erro esperado (variável não declarada, redeclaração, atribuição a `const`, mismatch de tipo, aridade de chamada, etc).

```bash
cargo test --test integration_test
```

Para adicionar um novo caso: crie um `.c` em `tests/integration/valid/` (deve compilar sem erros) ou `tests/integration/invalid/` (deve falhar com o diagnóstico esperado) e adicione o caso correspondente em `tests/integration_test.rs`.

### `tests/exe_smoke_test.rs` e `tests/codegen_smoke.rs` — ponta a ponta, com execução real

Estes são os testes mais completos: compilam código C real até assembly x86-64, montam e linkam com `gcc` em um executável ELF, executam o binário e verificam o **exit code** (e, quando aplicável, a saída em stdout).

```bash
cargo test --test exe_smoke_test
cargo test --test codegen_smoke
cargo test --test double_codegen_test
```

Se `gcc` não estiver disponível no `PATH`, esses testes são pulados (skip) automaticamente — verifique a saída de `cargo test -- --nocapture` por `gcc indisponivel: pulando teste de smoke` para confirmar.

### Testando manualmente com os arquivos de `src/examples/`

O diretório `src/examples/` contém programas `.c` de exemplo usados como referência/demonstração. Para testar manualmente o pipeline completo sobre um deles:

```bash
# 1. compilar e linkar
cargo run --release -- src/examples/simple.c -o /tmp/simple

# 2. executar e checar o resultado
/tmp/simple; echo "exit code: $?"
```

Para inspecionar estágios intermediários:

```bash
cargo run -- src/examples/simple.c --dump-tokens     # tokens do lexer
cargo run -- src/examples/simple.c --dump-ast        # AST
cargo run -- src/examples/simple.c --emit=asm -o /tmp/simple.s   # assembly x86-64 gerado
```

Exemplos disponíveis e seu status atual:

| Arquivo | Compila e roda? | Observação |
|---|---|---|
| `hello_world.c` | Sim | Imprime `Hello, World!` |
| `simple.c` | Sim | |
| `demo_presentation.c` | Sim | Demo usada na apresentação da disciplina |
| `declarations.c` | Gera assembly, mas não tem `main` | Não é pensado para ser linkado/executado isoladamente |
| `full_code1.c` | Não | Usa `float`, sem codegen ainda (`double` já é suportado, ver issue #172) |
| `operators.c` | Não — nem com `gcc` | Tem statements em escopo global, o que não é C válido (confirmado com `gcc -fsyntax-only`); não é um bug do compilador |

### Testando com seus próprios arquivos `.c`

Qualquer arquivo `.c` válido pode ser usado diretamente:

```bash
cargo run --release -- caminho/para/arquivo.c -o /tmp/saida
/tmp/saida
```

Para depurar um erro de compilação, repita o comando com `--dump-ast` ou `--only-semantic` para isolar em qual fase o problema ocorre.

## Checagens de qualidade (rodadas no CI)

O CI (`.github/workflows/`) roda, nesta ordem, em todo push/PR para `developer` e `master`:

```bash
cargo build --all
cargo test --all
cargo clippy -- -D warnings
cargo fmt --check
```

Rode as quatro localmente antes de abrir um PR — é exatamente o que será verificado automaticamente.

## Cobertura conhecida e limitações dos testes

- `double` tem cobertura dedicada em `tests/double_codegen_test.rs` (issue #172). Não há testes automatizados para `float` em codegen, porque a feature ainda não existe no backend.
- Os smoke tests de execução (`exe_smoke_test.rs`, `codegen_smoke.rs`, `double_codegen_test.rs`) dependem de Linux x86-64 + `gcc`; em outras plataformas eles são pulados, não falham.
