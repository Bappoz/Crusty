antes de qualquer mudança, quero que voce analise o que foi feito do analiasdor semantico ate o momento. Vou enviar as issues ja fechadas e seus respectivos PRs

[SEMANTIC 00] Implementar a estrutura de dados da tabela de símbolos com escopo léxico

## Tarefas
- [x] Criar struct `Symbol { name, ty, mutable, decl_span }`
- [x] Criar struct `SymbolTable { scopes: Vec<HashMap<String, Symbol>> }`
- [x] Implementar `enter_scope()`, `exit_scope()`
- [x] Implementar `declare()` (erro se redeclarado no mesmo escopo)
- [x] Implementar `lookup()` percorrendo do escopo mais interno
- [x] Implementar `lookup_current_scope()`

## Testes
```rust
#[test] fn inner_scope_shadows_outer() { … }
#[test] fn redeclaration_in_same_scope_errors() { … }
#[test] fn lookup_returns_none_for_undeclared() { … }
#[test] fn exit_scope_removes_declarations() { … }
```
This pull request introduces significant improvements to the parser and semantic analysis infrastructure, with a focus on handling `sizeof` expressions, type casting, and introducing a robust symbol table for semantic checks. It also adds comprehensive tests to ensure correctness of parsing, symbol table behavior, and special character handling. The main changes are grouped below
### Parser and AST Enhancements

* Added a new `Expr::SizeofType(QualifierType, Span)` variant to the AST to distinguish between `sizeof(type)` and `sizeof(expr)`, and updated the parser logic to correctly parse both forms. 
* Removed unnecessary expression terminator checks in the parser, simplifying expression parsing and error handling. 

### Symbol Table and Semantic Analysis

* Introduced a `symbol_table` module implementing a scoped symbol table with support for declaration, lookup, shadowing, and redeclaration error detection. 
* Extended `SemanticErrorKind` with a `Redeclaration` variant and improved error reporting for redeclaration cases. 

### Testing Improvements

* Added comprehensive tests for the symbol table, covering lookups, shadowing, redeclaration, scope management, and qualifiers. 
* Enhanced parser tests to verify correct parsing of cast expressions, `sizeof` in both forms, and binary operations following these constructs. 
* Added new tests for UTF-8 character boundary handling and memory-mapped file source input.

---

[SEMANTIC 02] Type checking de variáveis locais e resolução de identificadores

## Descrição:
Ao visitar Stmt::VarDecl, o analyser deve: (1) checar se o tipo é válido, (2) registrar o símbolo no escopo atual, (3) checar compatibilidade de tipos se há inicializador. Ao visitar Expr::Ident, deve fazer lookup e emitir UndefinedVariable se não encontrar.

## Acceptance criteria:

- Stmt::VarDecl → declare(Symbol { ... }) na scope atual, erro em redeclaração
- Expr::Ident → lookup(name), emite UndefinedVariable se None
- Atribuição: const não pode ser reatribuído → novo SemanticErrorKind::AssignToConst
- Testes: uso de variável não declarada, redeclaração no mesmo escopo, assign a const

---

feat(analyser): type checking de variáveis e resolução de identificadores (issue #12)


This pull request introduces a semantic analysis phase to the compiler, adding a `SemanticAnalyser` that performs semantic checks on the AST, accumulates errors, and integrates these checks into the main compilation pipeline. It also expands the error types and reporting, and provides a comprehensive suite of tests for semantic analysis.

**Semantic Analysis Implementation:**

* Added a new `SemanticAnalyser` in `src/analyser/semantic.rs` that traverses the AST, checks for semantic errors such as use of undeclared variables, redeclarations, and assignments to constants, and accumulates diagnostics. Also provides type inference for literals and basic expressions.
* Exposed the `analyse` function and `SemanticAnalyser` struct in `src/analyser/mod.rs` for use in other modules.

**Error Handling and Reporting:**

* Extended the `SemanticErrorKind` enum to include a new `AssignToConst` variant for assignment to constant variables, and updated error reporting to provide user-friendly diagnostics and suggestions. 

**Integration with Compiler Pipeline:**

* Integrated semantic analysis into the main compilation flow in `src/main.rs`, running semantic checks after parsing and before code generation, and reporting all semantic errors found. 

**Testing:**

* Added a new `semantic_test` module with extensive unit tests for the semantic analyser, covering symbol registration, error accumulation, scoping, assignment to constants, and type inference. 

---

feat: Implementei a estrutura de dados da tabela de simbolos

This pull request introduces significant improvements to the parser and semantic analysis infrastructure, with a focus on handling `sizeof` expressions, type casting, and introducing a robust symbol table for semantic checks. It also adds comprehensive tests to ensure correctness of parsing, symbol table behavior, and special character handling. The main changes are grouped below
### Parser and AST Enhancements

* Added a new `Expr::SizeofType(QualifierType, Span)` variant to the AST to distinguish between `sizeof(type)` and `sizeof(expr)`, and updated the parser logic to correctly parse both forms. 
* Removed unnecessary expression terminator checks in the parser, simplifying expression parsing and error handling. 

### Symbol Table and Semantic Analysis

* Introduced a `symbol_table` module implementing a scoped symbol table with support for declaration, lookup, shadowing, and redeclaration error detection. 
* Extended `SemanticErrorKind` with a `Redeclaration` variant and improved error reporting for redeclaration cases. 

### Testing Improvements

* Added comprehensive tests for the symbol table, covering lookups, shadowing, redeclaration, scope management, and qualifiers. 
* Enhanced parser tests to verify correct parsing of cast expressions, `sizeof` in both forms, and binary operations following these constructs. 
* Added new tests for UTF-8 character boundary handling and memory-mapped file source input.

---


[SEMANTIC 07] Análise semântica de structs (acesso a campos)


feat(semantic)/Análise semântica de structs (acesso direto ou por ponteiro a campos) #99

# Descrição
Este PR implementa a verificação estática e análise semântica para acessos a membros de estruturas (`Expr::Member`), cobrindo tanto o acesso direto via ponto (`.`) quanto o acesso via ponteiro por seta (`->`). 

Além disso, integra a validação dos campos consultando a tabela de símbolos de structs (`struct_table`) para garantir a existência dos membros solicitados.

close #99 

## Modificações
- **`src/analyser/analyzer.rs`**: 
  - Adicionada lógica recursiva no `check_expr` para a variante `Expr::Member`.
  - Implementada a bifurcação de tipos no `MemberAccess::Direct`, exigindo `Type::Struct`.
  - Implementada a desestruturação de ponteiros no `MemberAccess::Pointer`, exigindo `Type::Pointer(Box<Type::Struct>)`.
  - Adicionado o escaneamento funcional via iteradores (`.iter().find()`) para validar se o campo existe na struct.
  - Mapeamento temporário de erros semânticos utilizando a variante de fallback `SemanticErrorKind::TypeMismatch`.

- **`src/tests/analyzer_test.rs`**:
  - Criação do arquivo e implementação de testes de unidade cobrindo os critérios de aceitação.

## Critérios de Aceite Validados
- [x] `Expr::Member(expr, Direct, field)` → Checa se `expr` é struct e se o campo existe.
- [x] `Expr::Member(expr, Pointer, field)` → Checa se `expr` é um ponteiro para struct (`*Struct`).
- [x] Suíte de testes validando: acesso correto por ponto, acesso correto por ponteiro e rejeição de campos inexistentes.

## Como Testar
Para rodar a nova suíte de testes de análise semântica estruturada para este nó da AST, execute o comando na raiz do projeto:

```bash
cargo test analyzer_test


---

feat(analyser): type checking de variáveis e resolução de identificadores (issue #12)


This pull request introduces a semantic analysis phase to the compiler, adding a `SemanticAnalyser` that performs semantic checks on the AST, accumulates errors, and integrates these checks into the main compilation pipeline. It also expands the error types and reporting, and provides a comprehensive suite of tests for semantic analysis.

**Semantic Analysis Implementation:**

* Added a new `SemanticAnalyser` in `src/analyser/semantic.rs` that traverses the AST, checks for semantic errors such as use of undeclared variables, redeclarations, and assignments to constants, and accumulates diagnostics. Also provides type inference for literals and basic expressions.
* Exposed the `analyse` function and `SemanticAnalyser` struct in `src/analyser/mod.rs` for use in other modules.

**Error Handling and Reporting:**

* Extended the `SemanticErrorKind` enum to include a new `AssignToConst` variant for assignment to constant variables, and updated error reporting to provide user-friendly diagnostics and suggestions. 

**Integration with Compiler Pipeline:**

* Integrated semantic analysis into the main compilation flow in `src/main.rs`, running semantic checks after parsing and before code generation, and reporting all semantic errors found. 

**Testing:**

* Added a new `semantic_test` module with extensive unit tests for the semantic analyser, covering symbol registration, error accumulation, scoping, assignment to constants, and type inference. 

