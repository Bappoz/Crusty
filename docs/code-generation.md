# Geração de Código

**Status:** Concluída

Traduzindo a representação intermediária em código de máquina ou assembly para a arquitetura alvo.

!!! note "Fluxograma"
    *(Reservado — IR → Backend → Código Assembly / Binário)*

## Visão Geral

A etapa final do compilador (*backend*) é responsável por traduzir o Código de Três Endereços (TAC) para código assembly x86-64 (utilizando sintaxe AT&T) que respeita a convenção de chamada System V AMD64 ABI. Toda a implementação dessa etapa reside em `src/codegen/last/`.

A abordagem adotada neste projeto é chamada de **geração "ingênua"** (naive generation). Como o compilador ainda não conta com um alocador universal de registradores (*register allocator* integrado à TAC), optou-se por utilizar o modelo de alocação em pilha (*spill* universal): todas as variáveis locais e temporários residem em posições fixas na *stack* (pilha). Os registradores da CPU (como `%rax` e `%rcx`) são utilizados apenas como memória rascunho (*scratch*) durante as operações.

---

## Estrutura do Módulo (`src/codegen/last/`)

O módulo foi dividido em diferentes arquivos para organizar as responsabilidades da emissão de código:

### 1. Convenção de Chamada (`abi.rs`)
Define e mapeia as regras do System V AMD64 ABI para a passagem de parâmetros de funções.
- Argumentos passados via registradores (em ordem): `%rdi`, `%rsi`, `%rdx`, `%rcx`, `%r8`, `%r9`.
- O retorno das funções é sempre armazenado em `%rax`.
- Argumentos excedentes são buscados na pilha em *offsets* positivos calculados a partir do *frame pointer* (`%rbp`).

### 2. Gerenciamento do Stack Frame (`frame.rs`)
Responsável pela administração do espaço de memória local (frame de pilha) que cada função precisa.
- Todos os identificadores (sejam variáveis do programa C mapeadas por `Var` ou temporários gerados internamente mapeados por `Temp`) ganham um offset fixo negativo relativo a `%rbp`.
- Há suporte para **tipos de tamanho variável** (como `structs` locais), em que o método `allocate_local_sized` reserva blocos de memória contíguos, com o tamanho devido, na pilha.
- Garante o **alinhamento de 16 bytes** no momento de preparar as chamadas de função (`call`), que é uma restrição obrigatória de ABI da arquitetura x86-64 do Linux e afins.

### 3. Seleção de Instruções (`x86_64.rs`)
Ponto central que traduz o TAC, instrução por instrução, em *strings* textuais com os mnemônicos do assembly AT&T.
- **Globais e Strings Literais:** Variáveis globais (inicializadas ou não) e strings literais do programa caem perfeitamente nas seções designadas (`.data`, `.rodata` e `.bss`). O acesso ocorre sempre relativo ao ponteiro de instrução (RIP-relative).
- **Prologues e Epilogues:** Para funções, é emitida a sequência tradicional de salvaguarda de frame (`pushq %rbp; movq %rsp, %rbp`) seguida do decréscimo de `%rsp` condizente ao tamanho necessário alocado. Os registradores que carregavam os argumentos também sofrem um *spill* para suas posições de memória na stack. No término, restaura a configuração e envia um `ret`.
- **Tratamento Efetivo:** As operações aritméticas, ramificações condicionais ou atribuições simples retiram os valores em memória alocando `%rax` e `%rcx`. Para acessos indiretos em blocos (`Operand::Deref`), um registrador *caller-saved* próprio e intocado — o `%r11` — é o manipulador de ponteiro.

### 4. Peephole Optimization (`peephole.rs`)
Um passo de otimização de "buraco da fechadura" que interage diretamente sobre o texto *assembly* cru gerado (sem o conhecimento da árvore original) e realiza correções rápidas focando em eficiência, que a varredura "ingênua" acima deixou passar:
- **Movimentações Redundantes:** Remove padrões em que o gerador carrega e regrava dados entre as mesmas fontes e destinos inúteis (ex: `movq A, B` e depois `movq B, A`).
- **Nulidade Matemática:** Corta cálculos de adições e subtrações inúteis baseados em `$0`.
- **Simplificação de Flags:** O salto que necessitava ler `$0` primeiro no registrador passa a checar o testamento natural via bit logic (`cmpq` para `testq`).
- **Otimização de Multiplicação:** Identifica valores cujas constantes na compilação representem potência de base 2 (`$2`, `$4`, `$8`) e efetiva *shifts* lógicos à esquerda (`shlq`) no lugar da custosa multiplicação (`imulq`).
- **Jumps Supérfluos:** Apaga pulos incondicionais (`jmp .L_label`) se a próxima linha declarada contígua for justamente a *label* solicitada.

---

## Processo de Montagem/Linkagem

A integração presente na Interface por Linha de Comando (CLI) suporta o gerenciamento ponta a ponta. Atualmente invocando a via `gcc -x assembler-with-cpp`, ela permite:
- Despejar somente o código fonte literal *assembly* (`--emit=asm`).
- Montar as instruções num módulo e produzir um objeto bruto ELF/COFF (`--emit=obj`).
- Gerar o executável nativo do sistema do usuário, completando o ciclo (`--emit=exe`, padrão da aplicação).
