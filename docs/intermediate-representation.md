# Representação Intermediária

**Status:** Planejada

Geração de uma forma intermediária independente de arquitetura, ponte entre a AST e o código de máquina.

!!! note "Fluxograma"
    *(Reservado — AST Anotada → Gerador de IR → Instruções IR)*

## Tópicos previstos

- Formato da IR (three-address code / SSA)
- Tradução de expressões, atribuições e controle de fluxo
- Variáveis temporárias e blocos básicos
- Otimizações na IR (constant folding, dead code elimination)
- Erros de IR (`IntermediateError`)
