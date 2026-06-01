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
[Codegen]       → (não implementado)
```

## Módulos

| Módulo | Status | Documentação |
|--------|--------|--------------|
| Lexer | Completo | [lexer.md](lexer.md) |
| Parser | Completo | [parser.md](parser.md) |
| Analisador Semântico | Em desenvolvimento | [semantic.md](semantic.md) |
| Geração de código | Não iniciado | — |

## Referências

- [Precedência de Operadores C (C11)](c_operator_precedence.md) — tabela ISO/IEC 9899:2011 com mapeamento para binding powers do Pratt parser

## Repositório

[github.com/Bappoz/crusty](https://github.com/Bappoz/crusty)
