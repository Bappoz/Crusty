# Crusty — Compilador C em Rust

Projeto da disciplina de Compiladores 1. Implementa um compilador para um subconjunto da linguagem C, escrito em Rust, com backend nativo x86-64 (System V ABI, Linux).

## Estágio atual

| Fase | Status |
|------|--------|
| Análise léxica | Completo |
| Análise sintática | Completo |
| Análise semântica | Completo |
| IR (TAC) | Completo |
| Otimizações (CSE, DCE, constant folding, copy propagation, LICM, inlining) | Completo |
| Geração de código x86-64 | Completo para tipos inteiros, ponteiros, structs, arrays, globais e `double` (registradores XMM) |

### Limitações conhecidas

- **`float` não tem codegen.** `double` já é suportado pelo backend x86-64 (registradores XMM, issue [#172](https://github.com/Bappoz/Crusty/issues/172)), mas `float` ainda não — o analisador semântico aceita e tipa `float`, mas o backend ainda não emite o código correspondente. Programas que usam `float` falham com `error: code generation` no estágio final; use `double` no lugar.
- O modo REPL interativo (executar `crusty` sem argumentos) não está implementado.
- `--dump-ir` ainda não imprime a IR (placeholder).

Fora isso, o pipeline completo (lexer → parser → análise semântica → IR → otimizações → assembly x86-64 → executável ELF via `gcc`) funciona ponta a ponta para um subconjunto relevante de C: tipos inteiros, `char` e `double`, ponteiros, structs, arrays de tamanho fixo, enums, typedefs, variáveis globais, todas as estruturas de controle (`if`/`while`/`do-while`/`for`/`switch`) e chamadas de função.

## Estrutura do projeto

```
src/
├── lexer/       Análise léxica — transforma código-fonte em tokens
├── parser/      Análise sintática — constrói a AST via Pratt parsing
├── analyser/    Análise semântica — tabela de símbolos, escopos, verificação de tipos
├── ir/          Geração e lowering da IR intermediária (TAC)
├── codegen/     Geração de código — otimizações sobre TAC (inter/) e backend x86-64 (last/)
├── common/      Estruturas compartilhadas: AST, erros, spans, utilitários
├── examples/    Arquivos .c de exemplo usados em testes e demonstrações
└── tests/       Testes unitários por módulo
tests/           Testes de integração e smoke tests (ponta a ponta, com gcc)
docs/            Documentação técnica de cada fase do compilador
```

## Começando

Instruções completas de instalação e configuração do ambiente (Rust, `gcc`, verificação de toolchain) estão em [INSTALL.md](INSTALL.md).

Resumo rápido:

```bash
rustup update stable
cargo build --release
cargo run --release -- src/examples/hello_world.c
./hello_world
```

## Uso

```bash
crusty [flags] <arquivo>
```

Principais flags (lista completa em `crusty` sem argumentos):

| Flag | Efeito |
|---|---|
| `--dump-tokens` | Lista os tokens emitidos pelo lexer |
| `--dump-ast` | Imprime a AST |
| `--only-lex` / `--only-parse` / `--only-semantic` | Para o pipeline no estágio indicado |
| `-S`, `--emit-asm` / `--emit=asm` | Para após emitir o assembly x86-64 (`.s`) |
| `--emit=obj` | Para após montar o objeto (`.o`), sem linkar |
| `--emit=exe` | Monta e linka um executável ELF rodável (padrão) |
| `-o <arquivo>`, `--out-dir <dir>`, `--out-name <nome>` | Controlam o caminho/nome de saída |
| `-O0`\|`-O1`\|`-O2`\|`-O3`, `--opt-level <n>` | Nível de otimização aplicado à IR |

Exemplo gerando e executando um binário:

```bash
cargo run --release -- src/examples/simple.c -o /tmp/simple
/tmp/simple; echo "exit: $?"
```

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

**IR e otimizações**
- Lowering de AST para TAC (Three-Address Code), incluindo arrays fixos, structs e globais
- Pipeline de otimização configurável por nível (`-O0`..`-O3`): constant folding, common subexpression elimination, dead code elimination, copy propagation, loop-invariant code motion, inlining

**Backend x86-64**
- Convenção de chamada System V ABI (inteiros/ponteiros em `rdi`..`r9`/`rax`, `double` em `xmm0`..`xmm7`)
- Endereço de variáveis, indexação de array, acesso a membro de struct (`.`, `->`), address-of/deref
- Aritmética, comparações e literais de `double` via registradores XMM (`addsd`/`subsd`/`mulsd`/`divsd`/`ucomisd`)
- `sizeof` em tempo de compilação
- Variáveis globais com acesso RIP-relative
- Peephole optimizer sobre o assembly emitido
- Emissão de `.s`, montagem de `.o` e link de executável via `gcc`

## Testes

Cobertura completa (testes unitários e testes com arquivos `.c` reais, executados de ponta a ponta) está documentada em [TESTER.md](TESTER.md).

Resumo rápido:

```bash
cargo test --all          # ~361 testes (unitários + integração + smoke e2e)
cargo clippy -- -D warnings
cargo fmt --check
```

## Documentação técnica

- [Lexer](docs/lexer.md) — scanner, tokens, erros léxicos
- [Parser](docs/parser.md) — Pratt parser, AST, recuperação de erros
- [Analisador Semântico](docs/semantic.md) — tabela de símbolos, verificação de tipos
- [Precedência de Operadores C](docs/c_operator_precedence.md) — tabela C11 e mapeamento para binding powers
- [INSTALL.md](INSTALL.md) — como preparar o ambiente e compilar o projeto
- [TESTER.md](TESTER.md) — como rodar e interpretar todos os testes

## Contribuidores

| Nome | GitHub / contato |
|---|---|
| Lucas Andrade Zanetti | [@Bappoz](https://github.com/Bappoz) |
| Gustavo Xavier | [@guxvr](https://github.com/guxvr) |
| Hugo Freitas Silva | [@HugoFreitass](https://github.com/HugoFreitass) |
| Matheus Lemes | [@matheuslemesam](https://github.com/matheuslemesam) |
| Philipe Caetano | philipe2015amancio@hotmail.com |
