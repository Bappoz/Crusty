use crusty::analyser::analyse;
use crusty::codegen::inter::opt::{pipeline_for_level, OptLevel};
use crusty::codegen::inter::Cfg;
use crusty::common::errors::report::{Report, ToReport};
use crusty::common::input::source::SourceFile;
use crusty::lexer::scanner::Scanner;
use crusty::parser::Parser;
use std::env;
use std::path::PathBuf;

/// Ponto de entrada: decide entre modo interativo (sem args) ou compilação de arquivo (1 arg).
fn main() -> std::io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let options = match parse_args(&args) {
        Ok(options) => options,
        Err(e) => report_and_exit(Box::new(e)),
    };

    match options.script {
        None => {
            if let Err(e) = run_prompt() {
                report_and_exit(e);
            }
        }
        Some(script) => {
            if let Err(e) = run_file(&script, options.opt_level) {
                report_and_exit(e);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompileOptions {
    script: Option<String>,
    opt_level: OptLevel,
}

#[derive(Debug)]
struct CliError {
    message: String,
}

impl ToReport for CliError {
    fn to_report(&self) -> Report {
        Report::new(&self.message)
            .with_help("Usage: crusty [-O0|-O1|-O2|-O3] [--opt-level 0|1|2|3] [script]")
    }
}

fn parse_args(args: &[String]) -> Result<CompileOptions, CliError> {
    let mut opt_level = OptLevel::default();
    let mut script = None;
    let mut i = 1;

    while i < args.len() {
        let arg = &args[i];

        if let Some(level) = arg.strip_prefix("-O").filter(|level| !level.is_empty()) {
            opt_level = OptLevel::parse(level).ok_or_else(|| CliError {
                message: format!("invalid optimization level: {arg}"),
            })?;
        } else if arg == "-O" || arg == "--opt-level" {
            i += 1;
            let Some(level) = args.get(i) else {
                return Err(CliError {
                    message: format!("missing value for {arg}"),
                });
            };
            opt_level = OptLevel::parse(level).ok_or_else(|| CliError {
                message: format!("invalid optimization level: {level}"),
            })?;
        } else if arg.starts_with('-') {
            return Err(CliError {
                message: format!("unknown option: {arg}"),
            });
        } else if script.is_none() {
            script = Some(arg.clone());
        } else {
            return Err(CliError {
                message: "expected at most one input script".to_string(),
            });
        }

        i += 1;
    }

    Ok(CompileOptions { script, opt_level })
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
fn run(source: SourceFile, opt_level: OptLevel) -> Result<(), Box<dyn ToReport>> {
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
            print_report(&diagnostic.to_report());
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
                print_report(&e.to_report());
            }
            return Err(Box::new(DiagnosticError { count }));
        }
    };

    let semantic_errors = analyse(&program);
    let sem_count = semantic_errors.len();
    if sem_count > 0 {
        eprintln!("\n=== Semantic Errors ({sem_count}) ===");
        for e in &semantic_errors {
            print_report(&e.to_report());
        }
        return Err(Box::new(DiagnosticError { count: sem_count }));
    }

    let mut cfg = Cfg::new();
    let opt_pipeline = pipeline_for_level(opt_level);
    opt_pipeline.run(&mut cfg, 10);

    Ok(())
}

fn print_report(report: &Report) {
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

/// Lê o arquivo no caminho informado e delega a execução para `run`.
fn run_file(path: &str, opt_level: OptLevel) -> Result<(), Box<dyn ToReport>> {
    let source = SourceFile::from_path(PathBuf::from(path))?;
    run(source, opt_level)?;
    Ok(())
}

/// Imprime o `Report` de erro no stderr de forma estruturada e encerra o processo com código 74.
fn report_and_exit(e: Box<dyn ToReport>) -> ! {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_default_options() {
        let parsed = parse_args(&args(&["crusty", "main.c"])).unwrap();

        assert_eq!(
            parsed,
            CompileOptions {
                script: Some("main.c".to_string()),
                opt_level: OptLevel::O0,
            }
        );
    }

    #[test]
    fn parses_short_opt_level_forms() {
        assert_eq!(
            parse_args(&args(&["crusty", "-O2", "main.c"]))
                .unwrap()
                .opt_level,
            OptLevel::O2
        );
        assert_eq!(
            parse_args(&args(&["crusty", "-O", "3", "main.c"]))
                .unwrap()
                .opt_level,
            OptLevel::O3
        );
    }

    #[test]
    fn parses_long_opt_level_form() {
        let parsed = parse_args(&args(&["crusty", "--opt-level", "1", "main.c"])).unwrap();

        assert_eq!(parsed.opt_level, OptLevel::O1);
    }
}
