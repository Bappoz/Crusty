use crusty::common::errors::report::ToReport;
use crusty::common::input::source::SourceFile;
use crusty::lexer::scanner::Scanner;
use std::env;
use std::path::PathBuf;
use std::process::exit;

/// Ponto de entrada: decide entre modo interativo (sem args) ou compilação de arquivo (1 arg).
fn main() -> std::io::Result<()> {
    let args: Vec<_> = env::args().collect();
    match args.len() {
        1 => {
            if let Err(e) = run_prompt() {
                report_and_exit(e);
            }
        }
        2 => {
            if let Err(e) = run_file(&args[1]) {
                report_and_exit(e);
            }
        }
        _ => {
            eprintln!("Usage: crusty [script]");
            exit(64);
        }
    }
    Ok(())
}

/// Modo REPL interativo; ainda não implementado.
fn run_prompt() -> Result<(), Box<dyn ToReport>> {
    todo!()
}

/// Executa o scanner sobre o `SourceFile` e imprime os tokens produzidos e eventuais diagnósticos.
fn run(source: SourceFile) -> Result<(), Box<dyn ToReport>> {
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
            let report = diagnostic.to_report();
            eprintln!("  error: {}", report.message);
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
    } else {
        println!("\n=== Diagnostics (0) ===");
    }
    Ok(())
}

/// Lê o arquivo no caminho informado e delega a execução para `run`.
fn run_file(path: &str) -> Result<(), Box<dyn ToReport>> {
    let source = SourceFile::from_path(PathBuf::from(path))?;
    run(source)?;
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
