use crusty::analyser::analyse;
use crusty::common::errors::render::{bold, red, yellow};
use crusty::common::errors::report::{Report, ToReport};
use crusty::common::errors::types::Severity;
use crusty::common::input::source::SourceFile;
use crusty::lexer::scanner::Scanner;
use crusty::parser::Parser;
use std::env;
use std::path::PathBuf;
use std::process::exit;

/// Opções de diagnóstico passadas via linha de comando.
#[derive(Default, Clone, Copy)]
struct DiagnosticsConfig {
    /// `--Werror`: trata todos os warnings como erros (faz a compilação falhar).
    werror: bool,
    /// `--no-warnings`: suprime a exibição de warnings.
    no_warnings: bool,
}

/// Ponto de entrada: faz o parse das flags, decide entre modo interativo
/// (sem arquivo) ou compilação de arquivo.
fn main() -> std::io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let mut config = DiagnosticsConfig::default();
    let mut file: Option<String> = None;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--Werror" => config.werror = true,
            "--no-warnings" => config.no_warnings = true,
            s if s.starts_with("--") => {
                eprintln!("unknown flag: {}", s);
                eprintln!("Usage: crusty [--Werror] [--no-warnings] [script]");
                exit(64);
            }
            s => {
                if file.is_some() {
                    eprintln!("Usage: crusty [--Werror] [--no-warnings] [script]");
                    exit(64);
                }
                file = Some(s.to_string());
            }
        }
    }

    match file {
        None => {
            if let Err(e) = run_prompt() {
                report_and_exit(e);
            }
        }
        Some(path) => {
            if let Err(e) = run_file(&path, &config) {
                report_and_exit(e);
            }
        }
    }
    Ok(())
}

/// Erro retornado por `run()` quando o scanner produz diagnósticos.
#[derive(Debug)]
struct DiagnosticError {
    count: usize,
}

impl ToReport for DiagnosticError {
    fn to_report(&self) -> Report {
        Report::new(&format!("compilation failed with {} error(s)", self.count))
    }
}

/// Modo REPL interativo; ainda não implementado.
fn run_prompt() -> Result<(), Box<dyn ToReport>> {
    todo!()
}

/// Executa o scanner e parser sobre o `SourceFile`, imprime tokens e AST.
fn run(source: SourceFile, config: &DiagnosticsConfig) -> Result<(), Box<dyn ToReport>> {
    let mut scanner = Scanner::new(source);
    scanner.scan();

    let token_count = scanner.tokens.len();
    println!("=== Tokens ({token_count}) ===");
    for token in &scanner.tokens {
        let lexeme = &scanner.src.source.as_str()[token.span.start..token.span.end];
        let kind_str = format!("{:?}", token.kind);
        println!(
            "  [{:3}:{:<3}]  {:<35} {:?}",
            token.line, token.col, kind_str, lexeme
        );
    }

    let diag_count = scanner.diagnostics.len();
    if diag_count > 0 {
        eprintln!("\n=== Diagnostics ({diag_count}) ===");
        for diagnostic in &scanner.diagnostics {
            print_report(&diagnostic.to_report(), Severity::Error);
        }
    } else {
        println!("\n=== Diagnostics (0) ===");
    }

    if diag_count > 0 {
        return Err(Box::new(DiagnosticError { count: diag_count }));
    }

    let mut parser = Parser::new(scanner.tokens);
    let program = match parser.parse_program() {
        Ok(p) => {
            println!("\n=== AST ({}) ===", p.decls.len());
            for decl in &p.decls {
                println!("{:#?}", decl);
            }
            p
        }
        Err(errors) => {
            let count = errors.len();
            eprintln!("\n=== Syntax Errors ({count}) ===");
            for e in &errors {
                print_report(&e.to_report(), Severity::Error);
            }
            return Err(Box::new(DiagnosticError { count }));
        }
    };

    let diagnostics = analyse(&program);
    let (errors, warnings): (Vec<_>, Vec<_>) =
        diagnostics.iter().partition(|d| d.is_error());

    let warn_count = warnings.len();
    if warn_count > 0 && !config.no_warnings {
        eprintln!("\n=== Warnings ({warn_count}) ===");
        for w in &warnings {
            print_report(&w.to_report(), Severity::Warning);
        }
    }

    if !errors.is_empty() {
        let count = errors.len();
        eprintln!("\n=== Semantic Errors ({count}) ===");
        for e in &errors {
            print_report(&e.to_report(), Severity::Error);
        }
        return Err(Box::new(DiagnosticError { count }));
    }

    // `--Werror`: warnings são tratados como erros e falham a compilação.
    if config.werror && warn_count > 0 {
        return Err(Box::new(DiagnosticError { count: warn_count }));
    }

    Ok(())
}

fn print_report(report: &Report, severity: Severity) {
    let label = match severity {
        Severity::Error => red(&bold("error")),
        Severity::Warning => yellow(&bold("warning")),
    };
    eprintln!("  {}: {}", label, report.message);
    if let Some(span) = &report.span {
        eprintln!("    --> {}:{}", span.line, span.column_start);
    }
    for label in &report.labels {
        eprintln!("    | {}", label.message);
    }
    if let Some(help) = &report.help {
        eprintln!("    = help: {}", help);
    }
}

/// Lê o arquivo no caminho informado e delega a execução para `run`.
fn run_file(path: &str, config: &DiagnosticsConfig) -> Result<(), Box<dyn ToReport>> {
    let source = SourceFile::from_path(PathBuf::from(path))?;
    run(source, config)?;
    Ok(())
}

/// Imprime o `Report` de erro no stderr de forma estruturada e encerra o processo com código 74.
fn report_and_exit(e: Box<dyn ToReport>) {
    let report = e.to_report();

    eprintln!("--- ERROR ---");
    eprintln!("Message: {}", report.message);

    if let Some(sys) = report.system {
        eprintln!("System Info: {}", sys);
    }

    if let Some(help) = report.help {
        eprintln!("Help: {}", help);
    }

    std::process::exit(74);
}
