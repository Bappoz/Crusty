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

#[derive(Debug)]
pub enum LexicalErrorKind {
    /// Caractere que o lexer não reconhece (ex: `@`, `$`)
    InvalidChar(char),
    /// `/*` aberto mas nunca fechado com `*/`
    UnclosedBlockComment,
    /// `(`, `[` ou `{` aberto mas nunca fechado
    UnclosedDelimiter(char),
    /// String ou char literal não fechada (ex: `"hello` sem `"`)
    UnterminatedLiteral(String),
}

#[derive(Debug)]
pub struct LexicalError {
    pub span: Span,
    pub kind: LexicalErrorKind,
}

impl ToReport for LexicalError {
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

            LexicalErrorKind::UnterminatedLiteral(lit) => Report::new("unterminated literal")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("literal '{}' nao foi terminada", lit))
                .with_help("Feche a string ou char corretamente."),
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
    TypeMismatch { expected: String, found: String },
}

#[derive(Debug)]
pub struct SemanticError {
    pub span: Span,
    pub kind: SemanticErrorKind,
}

impl ToReport for SemanticError {
    fn to_report(&self) -> Report {
        match &self.kind {
            SemanticErrorKind::UndefinedVariable(var) => Report::new("variable not defined")
                .with_span(self.span.clone())
                .with_label(self.span.clone(), format!("'{}' nao existe", var))
                .with_help("declare a variavel antes de usar"),
            SemanticErrorKind::TypeMismatch { expected, found } => Report::new("type error")
                .with_span(self.span.clone())
                .with_label(
                    self.span.clone(),
                    format!("esperado: '{}', encontrado: '{}'", expected, found),
                ),
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
    fn to_report(&self) -> Report {
        let mut report = Report::new("IR error");
        if let Some(instr) = &self.instruction {
            report = report.with_label(
                Span {
                    line: 0,
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
    fn to_report(&self) -> Report {
        let mut report = Report::new("code generation");
        if let Some(instr) = &self.instruction {
            report = report.with_help(&format!("invalid register in instruction '{}'", instr));
        }
        report.with_help(&format!("detalhe: '{}'", self.message))
    }
}
