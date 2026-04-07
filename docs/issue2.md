## Issue 2 – Definição de Token e Scanner

### 1. Token canônico

- Foi criado o enum `TokenKind` em `src/common/token.rs`, cobrindo:
	- Palavras reservadas: `If`, `While`, `For`, `Loop`, `Else`, `Match`, `Let`, `Fn`, `Struct`, `Enum`, `Impl`, `Trait`, `Pub`, `Mod`, `Use`, `Const`, `Static`, `Int`, `Return`.
	- Operadores: `Plus`, `Minus`, `Star`, `Slash`, `EqualEqual`, `BangEqual`, `LessEqual`, `GreaterEqual`, `Less`, `Greater`, `Equal`.
	- Delimitadores: `LeftParen`, `RightParen`, `LeftBrace`, `RightBrace`, `Semicolon`, `Comma`.
	- Literais: `IntLiteral`, `FloatLiteral`, `StringLiteral`, `CharLiteral`.
	- Outros: `Identifier`, `Eof`, `Unknown`.
- Foi criada a struct `Token` em `src/common/token.rs`:
	- `pub struct Token { kind: TokenKind, lexeme: String, line: usize, col: usize }`.
	- Derivações: `#[derive(Debug, Clone, PartialEq)]`.
- Implementado `Display` para `Token`, exibindo apenas o `lexeme`.
- Adicionados testes unitários em `src/common/token.rs`:
	- `token_display_shows_lexeme` garante que `to_string()` devolve o lexema.
	- `token_kind_eq_works` verifica igualdade entre variantes de `TokenKind`.
- O módulo foi exposto em `src/common/mod.rs` via `pub mod token;`.

### 2. Integração com lexer e parser

- Em `src/lexer/token.rs`:
	- Reexportamos o tipo canônico: `pub use crate::common::token::{Token, TokenKind};`.
- Em `src/lexer/mod.rs`:
	- Mantidos `pub mod scanner;` e `pub mod token;`.
	- Reexportados `Token` e `TokenKind` para facilitar o uso: `pub use crate::common::token::{Token, TokenKind};`.
- Em `src/parser/mod.rs`:
	- Importados os tipos canônicos: `use crate::common::token::{Token, TokenKind};`, preparando o parser para trabalhar sobre a mesma definição de token.

### 3. Implementação do Scanner

- Em `src/lexer/scanner.rs` foi implementado o `Scanner`, responsável por transformar o código-fonte em `Vec<Token>`:
	- `Scanner::new(source: &str)` inicializa o scanner com: `source: Vec<char>`, `tokens: Vec<Token>`, `start`, `current`, `line`, `col`.
	- `scan_tokens(self) -> Vec<Token>`:
		- Varre o código até o fim chamando `scan_token`.
		- Ao final, adiciona um token `TokenKind::Eof`.
- Principais operações internas:
	- Navegação: `is_at_end`, `advance`, `peek`, `peek_next`.
	- Emissão de token: `add_token(kind: TokenKind)` monta o `lexeme` a partir do intervalo `[start, current)`.
- Regras de lexing implementadas em `scan_token`:
	- Ignora espaços em branco (`' '`, `'\r'`, `'\t'`) e conta linhas em `'\n'`.
	- Reconhece delimitadores: `(`, `)`, `{`, `}`, `;`, `,`.
	- Reconhece operadores simples e compostos:
		- `+`, `-`, `*`, `/`.
		- `=`, `==`, `!`, `!=`, `<`, `<=`, `>`, `>=`.
	- Literais:
		- Strings: `string_literal()` lê até a próxima `"`, marcando `TokenKind::StringLiteral`.
		- Números: `number()` lê inteiros e, se houver ponto seguido de dígitos, classifica como `FloatLiteral`, senão `IntLiteral`.
	- Identificadores e palavras reservadas:
		- `identifier()` consome `{letras, dígitos, '_'}` e faz o mapeamento:
			- `"if"`, `"while"`, `"for"`, `"loop"`, `"else"`, `"match"`, `"let"`, `"fn"`, `"struct"`, `"enum"`, `"impl"`, `"trait"`, `"pub"`, `"mod"`, `"use"`, `"const"`, `"static"`, `"int"`, `"return"` → variantes correspondentes de `TokenKind`.
			- Qualquer outro lexema alfabético → `TokenKind::Identifier`.
	- Qualquer caractere não reconhecido cai em `TokenKind::Unknown`.
- Funções auxiliares:
	- `is_identifier_start` e `is_identifier_continue` definem o que é um caractere válido de identificador.
- Bug corrigido:
	- Havia uma definição vazia de `pub struct Scanner {}` no topo de `scanner.rs`, removida para evitar conflito com a estrutura real.

