use crusty::common::errors::report::ToReport;
use crusty::common::input::source::SourceFile;
use crusty::lexer::scanner::Scanner;
use std::env;
use std::path::PathBuf;
use std::process::exit;

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

fn run_prompt() -> Result<(), Box<dyn ToReport>> {
    todo!()
}

fn run(source: SourceFile) -> Result<(), Box<dyn ToReport>> {
    let mut scanner = Scanner::new(source);
    scanner.scan();

    for token in &scanner.tokens {
        let lexeme = &scanner.src.source.as_str()[token.span.start..token.span.end];
        println!("{:?} {:?}", token.kind, lexeme);
    }

    if !scanner.diagnostics.is_empty() {
        eprintln!(
            "\n--- {} erro(s) encontrado(s) ---",
            scanner.diagnostics.len()
        );
        for diaginostic in &scanner.diagnostics {
            let report = diaginostic.to_report();
            eprintln!("  {}", report.message);
        }
    }
    Ok(())
}

fn run_file(path: &str) -> Result<(), Box<dyn ToReport>> {
    let source = SourceFile::from_path(PathBuf::from(path))?;
    run(source)?;
    Ok(())
}

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
