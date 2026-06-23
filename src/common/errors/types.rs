use crate::common::errors::error_data::Span;
use crate::common::errors::report::{Report, ToReport};

#[derive(Debug)]
pub enum CompilerError {
    Lexical(LexicalError),
    Syntax(SyntaxError),
    Semantic(SemanticError),
    Intermediate(IntermediateError),
    Optimization(OptimizationError),
    Codegen(CodegenError),
}

impl ToReport for CompilerError {
    /// Delega a conversão para o `Report` específico de cada variante de erro do compilador.
    fn to_report(&self) -> Report {
        match self {
            CompilerError::Lexical(e) => e.to_report(),
            CompilerError::Syntax(e) => e.to_report(),
            CompilerError::Semantic(e) => e.to_report(),
            CompilerError::Intermediate(e) => e.to_report(),
            CompilerError::Optimization(e) => e.to_report(),
            CompilerError::Codegen(e) => e.to_report(),
        }
    }
}

/// Nível de severidade de um diagnóstico.
/// `Error` impede a compilação; `Warning` apenas sinaliza um problema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Diagnóstico unificado: pode ser um erro (impede a compilação) ou um aviso.
/// O analisador semântico produz `Vec<Diagnostic>` misturando ambos.
#[derive(Debug)]
pub enum Diagnostic {
    Error(CompilerError),
    Warning(CompilerWarning),
}

impl Diagnostic {
    /// Retorna a severidade deste diagnóstico.
    pub fn severity(&self) -> Severity {
        match self {
            Diagnostic::Error(_) => Severity::Error,
            Diagnostic::Warning(_) => Severity::Warning,
        }
    }

    /// Retorna `true` quando este diagnóstico impede a compilação.
    pub fn is_error(&self) -> bool {
        matches!(self, Diagnostic::Error(_))
    }

    /// Retorna `true` quando este diagnóstico é apenas um aviso.
    pub fn is_warning(&self) -> bool {
        matches!(self, Diagnostic::Warning(_))
    }
}

impl ToReport for Diagnostic {
    /// Delega a conversão para o `Report` do erro ou aviso subjacente.
    fn to_report(&self) -> Report {
        match self {
            Diagnostic::Error(e) => e.to_report(),
            Diagnostic::Warning(w) => w.to_report(),
        }
    }
}

/// Categoria de aviso do compilador. Apenas semânticos por enquanto.
#[derive(Debug)]
pub enum CompilerWarning {
    Semantic(SemanticWarning),
}

impl ToReport for CompilerWarning {
    fn to_report(&self) -> Report {
        match self {
            CompilerWarning::Semantic(w) => w.to_report(),
        }
    }
}

#[derive(Debug)]
pub enum LexicalErrorKind {
    /// Caractere que o lexer não reconhece (ex: `@`, `$`)
    InvalidChar(char),
    /// `/*` aberto mas nunca fechado com `*/`
    UnclosedBlockComment,
    /// `(`, `[` ou `{` aberto mas nunca fechado
    UnclosedDelimiter(char),
    /// `)`, `]` ou `}` encontrado sem par de abertura correspondente
    UnexpectedClosingDelimiter(char),
    /// String ou char literal não fechada (ex: `"hello` sem `"`)
    UnterminatedLiteral(String),
    /// Dígito inválido 8 e 9 para Octal
    InvalidOctalDigit(char),
}

#[derive(Debug)]
pub struct LexicalError {
    pub span: Span,
    pub kind: LexicalErrorKind,
}

impl ToReport for LexicalError {
    /// Converte o erro léxico em `Report` com mensagem, span e sugestão de correção específicos ao tipo.
    fn to_report(&self) -> Report {
        match &self.kind {
            LexicalErrorKind::InvalidChar(c) => Report::new("invalid character")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("'{}' nao e valido", c))
                .with_help("Remova ou substitua o caractere."),

            LexicalErrorKind::UnclosedBlockComment => Report::new("unclosed block comment")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), "comentario de bloco nao fechado".to_string())
                .with_help("Adicione '*/' para fechar o comentario."),

            LexicalErrorKind::UnclosedDelimiter(c) => Report::new("unclosed delimiter")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("'{}' nao foi fechado", c))
                .with_help("Adicione o delimitador de fechamento correspondente."),

            LexicalErrorKind::UnexpectedClosingDelimiter(c) => Report::new("unexpected closing delimiter")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("'{}' nao tem par de abertura", c))
                .with_help("Remova o delimitador ou adicione o par de abertura correspondente."),

            LexicalErrorKind::UnterminatedLiteral(lit) => Report::new("unterminated literal")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("literal '{}' nao foi terminada", lit))
                .with_help("Feche a string ou char corretamente."),

            LexicalErrorKind::InvalidOctalDigit(c) => Report::new("invalid octal digit")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("'{}' nao e um digito octal valido", c))
                .with_help("Numeros que comecam com '0' sao tratados como octais. Use apenas digitos de 0 a 7."),
        }
    }
}

#[derive(Debug)]
pub struct SyntaxError {
    pub span: Span,
    pub expected: String,
    pub found: String,
}

impl ToReport for SyntaxError {
    /// Converte o erro sintático em `Report` indicando o token esperado versus o encontrado.
    fn to_report(&self) -> Report {
        Report::new("syntax error")
            .with_span(self.span.clone())
            .with_label(
                self.span.clone(),
                format!("esperado '{}', encontrado '{}'", self.expected, self.found),
            )
            .with_help(&format!("talvez você quis usar: '{}'", self.expected))
    }
}

/*
 * Erros semanticos geralmente envolvem:
 *      - variável não declarada
 *      - tipo imcompátivel
 *      - função inexistente
 *      - uso incorreto de simbolos
*/

#[derive(Debug)]
pub enum SemanticErrorKind {
    UndefinedVariable(String),
    Redeclaration(String),
    ReturnInVoid,
    TypeMismatch {
        expected: String,
        found: String,
    },
    ArityMismatch {
        expected: usize,
        found: usize,
    },
    CallNonFunction(String),
    UndefinedStruct(String),
    FieldNotFound {
        struct_name: String,
        field_name: String,
    },
    AssignToConst(String),
    /// Protótipo e definição da função têm assinaturas diferentes.
    PrototypeMismatch {
        name: String,
        expected: String,
        found: String,
    },
    /// Protótipo declarado mas nunca implementado.
    PrototypeMissingBody(String),
    InvalidIndexType {
        found: String,
    },
    NotIndexable {
        found: String,
    },
    /// Expressão do `switch` não é de tipo inteiro.
    InvalidSwitchType {
        found: String,
    },
    /// `break` usado fora de loop ou `switch`.
    BreakOutsideLoop,
    /// `continue` usado fora de loop.
    ContinueOutsideLoop,
}

#[derive(Debug)]
pub struct SemanticError {
    pub span: Span,
    pub kind: SemanticErrorKind,
}

impl ToReport for SemanticError {
    /// Converte o erro semântico em `Report` descrevendo variável indefinida ou incompatibilidade de tipos.
    fn to_report(&self) -> Report {
        match &self.kind {
            SemanticErrorKind::UndefinedVariable(var) => Report::new("variable not defined")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("'{}' nao existe", var))
                .with_help("declare a variavel antes de usar"),
            SemanticErrorKind::Redeclaration(name) => Report::new("Redeclaration error")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("'{}' já foi declarado neste escopo", name),
                )
                .with_help("use um nome diferente ou remova a declaração duplicada"),
            SemanticErrorKind::ReturnInVoid => Report::new("return in void function")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    "função void não pode retornar um valor".to_string(),
                ),
            SemanticErrorKind::TypeMismatch { expected, found } => Report::new("type error")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("esperado: '{}', encontrado: '{}'", expected, found),
                ),
            SemanticErrorKind::ArityMismatch { expected, found } => Report::new("arity mismatch")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("expected {} args, found {}", expected, found),
                ),
            SemanticErrorKind::CallNonFunction(name) => Report::new("call on non-function")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("'{}' is not callable", name)),
            SemanticErrorKind::UndefinedStruct(name) => Report::new("undefined struct")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("struct '{}' nao foi declarada", name),
                )
                .with_help("declare a struct antes de usar"),
            SemanticErrorKind::FieldNotFound {
                struct_name,
                field_name,
            } => Report::new("field not found")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("campo '{}' nao existe em '{}'", field_name, struct_name),
                ),
            SemanticErrorKind::AssignToConst(name) => Report::new("assignment to const")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("'{}' é const e não pode ser reatribuído", name),
                )
                .with_help("remova o qualificador const ou use uma variável mutável"),
            SemanticErrorKind::PrototypeMismatch {
                name,
                expected,
                found,
            } => Report::new("prototype mismatch")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!(
                        "definição de '{}' diverge do protótipo: esperado '{}', encontrado '{}'",
                        name, expected, found
                    ),
                )
                .with_help("ajuste a assinatura da função para corresponder ao protótipo"),
            SemanticErrorKind::PrototypeMissingBody(name) => {
                Report::new("prototype without definition")
                    .with_span(self.span.clone())
                    .with_label(
                        self.span.clone(),
                        format!("'{}' foi declarada mas nunca definida", name),
                    )
                    .with_help("adicione a implementação da função ou remova o protótipo")
            }
            SemanticErrorKind::InvalidIndexType { found } => Report::new("invalid index type")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("índice deve ser inteiro, encontrado '{}'", found),
                )
                .with_help("use um tipo inteiro (int, long, short, char) como índice"),
            SemanticErrorKind::NotIndexable { found } => Report::new("not indexable")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("'{}' não é indexável (esperado array ou ponteiro)", found),
                ),
            SemanticErrorKind::InvalidSwitchType { found } => {
                Report::new("invalid switch expression type")
                    .with_span(self.span.clone())
                    .with_label(
                        self.span.clone(),
                        format!(
                            "expressão do switch deve ser inteira, encontrado '{}'",
                            found
                        ),
                    )
                    .with_help("use int, char, short, long ou enum como discriminante do switch")
            }
            SemanticErrorKind::BreakOutsideLoop => Report::new("break outside loop or switch")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    "'break' só pode ser usado dentro de loop ou switch".to_string(),
                )
                .with_help("mova o 'break' para dentro de um for, while, do-while ou switch"),
            SemanticErrorKind::ContinueOutsideLoop => Report::new("continue outside loop")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    "'continue' só pode ser usado dentro de loop".to_string(),
                )
                .with_help("mova o 'continue' para dentro de um for, while ou do-while"),
        }
    }
}

/*
 * Avisos semânticos são problemas que não impedem a compilação, mas devem ser
 * sinalizados ao usuário. Exemplos:
 *      - variável declarada mas nunca lida (unused-variable)
 *      - variável possivelmente usada sem inicialização (uninitialized)
 */

#[derive(Debug)]
pub enum SemanticWarningKind {
    /// Variável declarada mas nunca lida em nenhum ponto do escopo.
    UnusedVariable(String),
    /// Variável lida antes de receber qualquer inicializador ou atribuição.
    MayBeUninitialized(String),
    /// Função não-void sem `return` detectado no caminho de saída principal.
    MissingReturn(String),
}

#[derive(Debug)]
pub struct SemanticWarning {
    pub span: Span,
    pub kind: SemanticWarningKind,
}

impl ToReport for SemanticWarning {
    /// Converte o aviso semântico em `Report` com mensagem, span e sugestão específicos.
    fn to_report(&self) -> Report {
        match &self.kind {
            SemanticWarningKind::UnusedVariable(var) => Report::new("unused variable")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("variavel '{}' declarada mas nunca lida", var),
                )
                .with_help("remova a declaracao ou use a variavel"),

            SemanticWarningKind::MayBeUninitialized(var) => {
                Report::new("may be used uninitialized")
                    .with_span(self.span.clone())
                    .with_label(
                        self.span.clone(),
                        format!("variavel '{}' pode ser usada sem inicializacao", var),
                    )
                    .with_help("inicialize a variavel antes de usa-la")
            }
            SemanticWarningKind::MissingReturn(fn_name) => Report::new("missing return")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!(
                        "função '{}' não-void pode encerrar sem retornar valor",
                        fn_name
                    ),
                )
                .with_help("adicione um 'return <expr>;' ao final da função"),
        }
    }
}

/*
   Nessa etapa do compilador será lidado com :
       - problemas na geração da IR
       - Inconsistência de nós
       - Variáveis temporárias inválidas
*/

#[derive(Debug)]
pub struct IntermediateError {
    pub message: String,
    pub instruction: Option<String>,
}

impl ToReport for IntermediateError {
    /// Converte o erro de IR em `Report` indicando a instrução problemática e a causa da falha.
    fn to_report(&self) -> Report {
        let mut report = Report::new("IR error");
        if let Some(instr) = &self.instruction {
            report = report.with_label(
                Span {
                    line: 0,
                    end_line: 0,
                    column_start: 0,
                    column_end: 0,
                },
                format!("na instrucao '{}'", instr),
            );
        }
        report.with_help(&self.message)
    }
}

/*
    Erros nessa fase geralemente envolvem:
        -perda de informação
        - transformações inválidas
        - divisão por zero detectada em otimização
        - falha em simplificação

    EXEMPLO: [Optimization Error] Erro na otimização (Constant Folding): divisão por zero detectada
*/

#[derive(Debug)]
pub struct OptimizationError {
    pub message: String,
    pub pass: String,
}

impl ToReport for OptimizationError {
    /// Converte o erro de otimização em `Report` identificando o passo (pass) e o motivo da falha.
    fn to_report(&self) -> Report {
        Report::new(&format!("Error na otimizacao ({})", self.pass)).with_help(&self.message)
    }
}

/*
    Aqui geralmente apresenta erros como :
        - Registradores insuficientes
        - instrucoes inválidas
        - erro de arquitetura alvo

    EXEMPLO: [CodeGen Error] instrução 'MOV' falhou no registrador 'R1'
*/

#[derive(Debug)]
pub struct CodegenError {
    pub message: String,
    pub instruction: Option<String>,
}

impl ToReport for CodegenError {
    /// Converte o erro de geração de código em `Report` com detalhes da instrução e registrador envolvidos.
    fn to_report(&self) -> Report {
        let mut report = Report::new("code generation");
        if let Some(instr) = &self.instruction {
            report = report.with_help(&format!("invalid register in instruction '{}'", instr));
        }
        report.with_help(&format!("detalhe: '{}'", self.message))
    }
}
