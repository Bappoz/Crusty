# Rippler — Compilador de C

O **Rippler** é um compilador de C desenvolvido como projeto acadêmico na disciplina de Compiladores 1.
Seu objetivo é transformar código-fonte C em código executável passando por todas as fases clássicas de um compilador moderno.

## Pipeline de Compilação

| # | Fase | Status |
|---|------|--------|
| 01 | [Análise Léxica](lexical-analysis.md) | Concluída |
| 02 | [Análise Sintática](syntax-analysis.md) | Em desenvolvimento |
| 03 | [Análise Semântica](semantic-analysis.md) | Planejada |
| 04 | [Representação Intermediária](intermediate-representation.md) | Planejada |
| 05 | [Geração de Código](code-generation.md) | Planejada |

## Estrutura do Projeto

```
src/
├── lexer/          # Análise Léxica (scanner, tokens, regras)
│   ├── scanner.rs
│   ├── tokens/
│   └── rules/
├── parser/         # Análise Sintática
├── analyser/       # Análise Semântica
├── codegen/        # Geração de Código
└── common/         # Utilitários compartilhados
    ├── errors/
    ├── input/
    └── utils/
```

## Tecnologias

| Componente | Tecnologia | Justificativa |
|------------|-----------|---------------|
| Linguagem | Rust | Segurança de memória e sistema de tipos expressivo |
| Leitura de arquivo | memmap2 | Memory-mapped I/O para arquivos grandes |
| Testes | Rust built-in | Testes unitários com `#[test]` |

## Recuperação de Erros

O Rippler adota a estratégia de *error recovery* em cada fase: ao encontrar um erro, o compilador
registra o diagnóstico e continua processando o restante do código. Isso permite que múltiplos
erros sejam reportados em uma única execução.
