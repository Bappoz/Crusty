# Crusty — Documentação Técnica

Compilador para um subconjunto da linguagem C, escrito em Rust. Projeto da disciplina de Compiladores 1.

## Pipeline

```
Código-fonte (.c)
    ↓
[Lexer]         → Vec<Token>  +  erros léxicos
    ↓
[Parser]        → Program (AST)  +  erros sintáticos
    ↓
[Analisador]    → diagnósticos semânticos
    ↓
[IR / TAC]      → Three-Address Code  +  otimizações (-O0..-O3)
    ↓
[Codegen]       → assembly x86-64  →  .o / executável ELF via gcc
```

## Módulos

| Módulo | Status | Documentação |
|--------|--------|--------------|
| Lexer | Completo | [lexer.md](lexer.md) |
| Parser | Completo | [parser.md](parser.md) |
| Analisador Semântico | Completo | [semantic.md](semantic.md) |
| IR (TAC) e otimizações | Completo | — |
| Geração de código x86-64 | Completo para tipos inteiros, ponteiros, structs, arrays, globais e `double`; `float` ainda sem codegen ([issue #172](https://github.com/Bappoz/Crusty/issues/172)) | — |

## Referências

- [Precedência de Operadores C (C11)](c_operator_precedence.md) — tabela ISO/IEC 9899:2011 com mapeamento para binding powers do Pratt parser

## Repositório

[github.com/Bappoz/Crusty](https://github.com/Bappoz/Crusty)
