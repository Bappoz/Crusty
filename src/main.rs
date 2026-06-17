use crusty::analyser::analyse;
use crusty::common::ast::pretty::pretty_program;
use crusty::common::errors::report::{Report, ToReport};
use crusty::common::input::source::SourceFile;
use crusty::lexer::scanner::Scanner;
use crusty::parser::Parser;
use std::env;
use std::path::PathBuf;
use std::process::exit;

fn main() -> std::io::Result<()> {
    let raw: Vec<_> = env::args().collect();
    let args = CliArgs::parse(&raw);

    let file = match &args.input_file {
        Some(f) => f.clone(),
        None => {
            eprintln!("Usage: crusty [flags] <file>");
            eprintln!("Flags:");
            eprintln!("  --dump-tokens      List all tokens emitted by the lexer");
            eprintln!("  --dump-ast         Pretty-print the AST");
            eprintln!("  --dump-ir          Dump TAC IR (not yet implemented)");
            eprintln!("  --only-lex         Stop after lexing");
            eprintln!("  --only-parse       Stop after parsing");
            eprintln!("  --only-semantic    Stop after semantic analysis");
            exit(64);
        }
    };

    let source = match SourceFile::from_path(PathBuf::from(&file)) {
        Ok(s) => s,
        Err(e) => report_and_exit(e),
    };

    if let Err(e) = run(source, &args) {
        report_and_exit(e);
    }
    Ok(())
}

// ── CLI arg parsing ──────────────────────────────────────────────────────────

struct CliArgs {
    input_file: Option<String>,
    dump_tokens: bool,
    dump_ast: bool,
    dump_ir: bool,
    only_lex: bool,
    only_parse: bool,
    only_semantic: bool,
}

impl CliArgs {
    fn parse(args: &[String]) -> Self {
        let mut cli = CliArgs {
            input_file: None,
            dump_tokens: false,
            dump_ast: false,
            dump_ir: false,
            only_lex: false,
            only_parse: false,
            only_semantic: false,
        };
        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--dump-tokens" => cli.dump_tokens = true,
                "--dump-ast" => cli.dump_ast = true,
                "--dump-ir" => cli.dump_ir = true,
                "--only-lex" => cli.only_lex = true,
                "--only-parse" => cli.only_parse = true,
                "--only-semantic" => cli.only_semantic = true,
                _ if arg.starts_with("--") => {
                    eprintln!("error: unknown flag '{arg}'");
                    exit(64);
                }
                _ if cli.input_file.is_some() => {
                    eprintln!(
                        "error: multiple input files provided ('{}' and '{arg}')",
                        cli.input_file.unwrap()
                    );
                    exit(64);
                }
                _ => cli.input_file = Some(arg.clone()),
            }
        }
        cli
    }
}

// ── Pipeline ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct DiagnosticError {
    count: usize,
}

impl ToReport for DiagnosticError {
    fn to_report(&self) -> Report {
        Report::new(&format!("compilation failed with {} error(s)", self.count))
    }
}

fn run(source: SourceFile, args: &CliArgs) -> Result<(), Box<dyn ToReport>> {
    // ── Stage 1: Lex ─────────────────────────────────────────────────────────
    let mut scanner = Scanner::new(source);
    scanner.scan();

    if args.dump_tokens {
        dump_tokens(&scanner);
    }

    let lex_errors = scanner.diagnostics.len();
    if lex_errors > 0 {
        eprintln!("\n=== Lex Errors ({lex_errors}) ===");
        for d in &scanner.diagnostics {
            print_report(&d.to_report());
        }
        return Err(Box::new(DiagnosticError { count: lex_errors }));
    }

    if args.dump_ir {
        eprintln!("=== IR (not yet implemented) ===");
    }

    if args.only_lex {
        return Ok(());
    }

    // ── Stage 2: Parse ───────────────────────────────────────────────────────
    let mut parser = Parser::new(scanner.tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(errors) => {
            let count = errors.len();
            eprintln!("\n=== Parse Errors ({count}) ===");
            for e in &errors {
                print_report(&e.to_report());
            }
            return Err(Box::new(DiagnosticError { count }));
        }
    };

    if args.dump_ast {
        dump_ast(&program);
    }

    if args.only_parse {
        return Ok(());
    }

    // ── Stage 3: Semantic ────────────────────────────────────────────────────
    let sem_errors = analyse(&program);
    let sem_count = sem_errors.len();
    if sem_count > 0 {
        eprintln!("\n=== Semantic Errors ({sem_count}) ===");
        for e in &sem_errors {
            print_report(&e.to_report());
        }
        return Err(Box::new(DiagnosticError { count: sem_count }));
    }

    if args.only_semantic {
        return Ok(());
    }

    Ok(())
}

// ── Dump helpers ─────────────────────────────────────────────────────────────

fn dump_tokens(scanner: &Scanner) {
    println!("=== Tokens ({}) ===", scanner.tokens.len());
    for token in &scanner.tokens {
        let lexeme = &scanner.src.source.as_str()[token.span.start..token.span.end];
        let len = token.span.end - token.span.start;
        let col_end = token.col + len.saturating_sub(1);
        let kind_str = format!("{:?}", token.kind);
        println!(
            "[{:3}:{:<3}-{:3}:{:<3}]  {:<35} {:?}",
            token.line, token.col, token.line, col_end, kind_str, lexeme
        );
    }
}

fn dump_ast(program: &crusty::common::ast::ast::Program) {
    println!("=== AST ===");
    print!("{}", pretty_program(program));
}

// ── Error reporting ───────────────────────────────────────────────────────────

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

fn report_and_exit(e: Box<dyn ToReport>) -> ! {
    let report = e.to_report();
    eprintln!("error: {}", report.message);
    if let Some(sys) = report.system {
        eprintln!("system: {}", sys);
    }
    if let Some(help) = report.help {
        eprintln!("help: {}", help);
    }
    exit(74);
}
