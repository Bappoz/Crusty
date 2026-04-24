# Geração de Código

**Status:** Planejada

Traduzindo a representação intermediária em código de máquina ou assembly para a arquitetura alvo.

!!! note "Fluxograma"
    *(Reservado — IR → Backend → Código Assembly / Binário)*

## Tópicos previstos

- Arquitetura alvo (x86-64 / LLVM IR)
- Seleção de instruções
- Alocação de registradores
- Geração de assembly / bytecode
- Erros de codegen (`CodegenError`)
