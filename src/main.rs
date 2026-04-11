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
            eprintln!("Usage: jlox [script]");
            exit(64);
        }
    }
    Ok(())
}

// Function to run the interactive prompt, returning an error if it fails (Not implemented yet)
fn run_prompt() -> Result<(), Box<dyn ToReport>> {
    todo!()
}

// Function to run the source code, returning an error if it fails
fn run(source: SourceFile) -> Result<(), Box<dyn ToReport>> {
    let mut scanner = Scanner::new(source);
    let tokens = scanner.scan();

    for token in tokens {
        println!(
            " [{}:{}] {:?} => \"{}\"",
            token.line, token.col, token.kind, token.lexeme,
        );
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

//  Function to read a file and run its contents, returning an error if the file cannot be read
fn run_file(path: &str) -> Result<(), Box<dyn ToReport>> {
    let source = SourceFile::from_path(PathBuf::from(path))?;
    run(source)?;
    Ok(())
}

// Function to report errors and exit the program with a non-zero status code
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

    // No jlox, erros de entrada/arquivo costumam usar o código 66 ou 74
    std::process::exit(74);
}
