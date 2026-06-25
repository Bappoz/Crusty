use std::path::PathBuf;

use crusty::analyser::analyse_with_builtins;
use crusty::common::errors::types::{CompilerError, Diagnostic, SemanticErrorKind};
use crusty::common::input::source::SourceFile;
use crusty::lexer::scanner::Scanner;
use crusty::parser::Parser;

struct CompileResult {
    diagnostics: Vec<Diagnostic>,
}

fn compile_file(rel: &str) -> CompileResult {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    let src = SourceFile::from_path(path)
        .unwrap_or_else(|e| panic!("failed to read fixture '{rel}': {:?}", e.to_report()));

    let mut scanner = Scanner::new(src);
    scanner.scan();
    let lex_diags = std::mem::take(&mut scanner.diagnostics);
    let tokens = std::mem::take(&mut scanner.tokens);

    let mut diagnostics: Vec<Diagnostic> = lex_diags.into_iter().map(Diagnostic::Error).collect();

    let mut parser = Parser::new(tokens);
    match parser.parse_program() {
        Ok(program) => diagnostics.extend(analyse_with_builtins(&program, scanner.builtins)),
        Err(parse_errs) => diagnostics.extend(parse_errs.into_iter().map(Diagnostic::Error)),
    }

    CompileResult { diagnostics }
}

fn has_semantic_error(diags: &[Diagnostic], pred: impl Fn(&SemanticErrorKind) -> bool) -> bool {
    diags.iter().any(|d| match d {
        Diagnostic::Error(CompilerError::Semantic(se)) => pred(&se.kind),
        _ => false,
    })
}

fn compile_valid(name: &str) {
    let result = compile_file(&format!("tests/integration/valid/{name}.c"));
    assert!(
        result.diagnostics.is_empty(),
        "valid program '{name}' should produce zero diagnostics, got: {:#?}",
        result.diagnostics
    );
}

fn assert_invalid(name: &str, pred: impl Fn(&SemanticErrorKind) -> bool, label: &str) {
    let result = compile_file(&format!("tests/integration/invalid/{name}.c"));
    assert!(
        has_semantic_error(&result.diagnostics, pred),
        "invalid program '{name}' should emit {label}, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_hello_world() {
    compile_valid("hello_world");
}

#[test]
fn valid_bool_literals() {
    compile_valid("bool_literals");
}

#[test]
fn valid_arithmetic() {
    compile_valid("arithmetic");
}

#[test]
fn valid_fibonacci() {
    compile_valid("fibonacci");
}

#[test]
fn valid_structs() {
    compile_valid("structs");
}

#[test]
fn valid_pointers() {
    compile_valid("pointers");
}

#[test]
fn valid_typedef() {
    compile_valid("typedef");
}

#[test]
fn valid_enum_usage() {
    compile_valid("enum_usage");
}

#[test]
fn valid_control_flow() {
    compile_valid("control_flow");
}

#[test]
fn invalid_undeclared_var_emits_error() {
    assert_invalid(
        "undeclared_var",
        |k| matches!(k, SemanticErrorKind::UndefinedVariable(_)),
        "UndefinedVariable",
    );
}

#[test]
fn invalid_type_mismatch_assign_emits_error() {
    assert_invalid(
        "type_mismatch_assign",
        |k| matches!(k, SemanticErrorKind::TypeMismatch { .. }),
        "TypeMismatch",
    );
}

#[test]
fn invalid_assign_const_emits_error() {
    assert_invalid(
        "assign_const",
        |k| matches!(k, SemanticErrorKind::AssignToConst(_)),
        "AssignToConst",
    );
}

#[test]
fn invalid_redeclaration_emits_error() {
    assert_invalid(
        "redeclaration",
        |k| matches!(k, SemanticErrorKind::Redeclaration(_)),
        "Redeclaration",
    );
}

#[test]
fn invalid_return_in_void_emits_error() {
    assert_invalid(
        "return_in_void",
        |k| matches!(k, SemanticErrorKind::ReturnInVoid),
        "ReturnInVoid",
    );
}

#[test]
fn invalid_arity_mismatch_emits_error() {
    assert_invalid(
        "arity_mismatch",
        |k| matches!(k, SemanticErrorKind::ArityMismatch { .. }),
        "ArityMismatch",
    );
}

#[test]
fn invalid_call_non_function_emits_error() {
    assert_invalid(
        "call_non_function",
        |k| matches!(k, SemanticErrorKind::CallNonFunction(_)),
        "CallNonFunction",
    );
}
