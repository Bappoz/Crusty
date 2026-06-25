# Crusty — Compilador de C

O **Crusty** é um compilador de C desenvolvido como projeto acadêmico na disciplina de Compiladores 1.
Seu objetivo é transformar código-fonte C em código executável passando por todas as fases clássicas de um compilador moderno.

## Pipeline de Compilação

| # | Fase | Status |
|---|------|--------|
| 01 | [Análise Léxica](lexical-analysis.md) | Concluída |
| 02 | [Análise Sintática](syntax-analysis.md) | Concluída |
| 03 | [Análise Semântica](semantic-analysis.md) | Concluída |
| 04 | [Representação Intermediária](intermediate-representation.md) | Concluída |
| 05 | [Otimização](optimization.md) | Concluída |
| 06 | [Geração de Código](code-generation.md) | Concluída |

## Estrutura do Projeto

```
src/
├── main.rs         # Ponto de entrada (CLI, pipeline integration)
├── lib.rs          # Declaração do pacote crusty
├── lexer/          # Análise Léxica (scanner, tokens, regras)
│   ├── scanner.rs
│   ├── tokens/
│   └── rules/
├── parser/         # Análise Sintática (Pratt parser, AST)
│   ├── parser.rs
│   ├── precedence.rs
│   └── rules/
├── analyser/       # Análise Semântica (tabela de símbolos, tipos)
│   ├── semantic.rs
│   └── symbol_table.rs
├── ir/             # Representação Intermediária (TAC, lowering, CFG)
│   ├── tac.rs
│   ├── lower.rs
│   └── cfg.rs
├── codegen/        # Geração de Código
│   ├── inter/      # Otimizador (DCE, propagation, constant folding, etc)
│   ├── last/       # Backend final (assembly x86-64, stack frames)
│   └── reg_alloc.rs
├── common/         # Estruturas compartilhadas
│   ├── ast/        # AST: decl, expr, stmt
│   ├── errors/
│   ├── input/
│   ├── utils/
│   └── builtins.rs # Funções embutidas (malloc, free, printf, etc)
└── tests/          # Testes unitários e de integração
```

## Tecnologias

| Componente | Tecnologia | Justificativa |
|------------|-----------|---------------|
| Linguagem | Rust | Segurança de memória e sistema de tipos expressivo |
| Leitura de arquivo | memmap2 | Memory-mapped I/O para arquivos grandes |
| Testes | Rust built-in | 165 testes unitários com `#[test]` |

## Recuperação de Erros

O Crusty adota a estratégia de *error recovery* em cada fase: ao encontrar um erro, o compilador
registra o diagnóstico e continua processando o restante do código. Isso permite que múltiplos
erros sejam reportados em uma única execução.
