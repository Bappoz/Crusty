## PROJETO DE COMPILADORES 1 


### OBJETIVO DE CADA PASTA

- src/lexer/: Análise Léxica. Contém a lógica para transformar o código-fonte (texto) em uma sequência de tokens.

- src/parser/: Análise Sintática. Define a gramática da linguagem e constrói a Árvore Sintática Abstrata (AST) a partir dos tokens.

- src/analyzer/: Análise Semântica. Realiza a verificação de tipos, checagem de escopo e garante que o código faz sentido logicamente.

- src/codegen/: Geração de Código. Transforma a AST validada em código de baixo nível (Assembly, LLVM IR ou Bytecode).

- src/common/: Estruturas compartilhadas por todo o projeto, como definições de erros, localização de arquivos (linhas/colunas) e utilitários.

- tests/: Testes de integração. Pasta para arquivos de exemplo que testam o fluxo completo do compilador (da entrada ao output final).

- examples/: Arquivos de código escritos na nossa própria linguagem para demonstração e testes rápidos.
