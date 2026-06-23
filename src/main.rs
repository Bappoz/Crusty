use crusty::analyser::analyse;
use crusty::codegen::inter::opt::{pipeline_for_level, OptLevel};
use crusty::codegen::inter::Cfg;
use crusty::codegen::last;
use crusty::common::ast::pretty::pretty_program;
use crusty::common::errors::report::{Report, ToReport};
use crusty::common::input::source::SourceFile;
use crusty::ir::lower::lower_program;
use crusty::lexer::scanner::Scanner;
use crusty::parser::Parser;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

fn main() -> std::io::Result<()> {
    let raw: Vec<_> = env::args().collect();
    let args = CliArgs::parse(&raw);

    let file = match &args.input_file {
        Some(f) => f.clone(),
        None => {
            eprintln!("Usage: crusty [flags] <file>");
            eprintln!("Flags:");
            eprintln!("  --dump-tokens          List all tokens emitted by the lexer");
            eprintln!("  --dump-ast             Pretty-print the AST");
            eprintln!("  --dump-ir              Dump TAC IR (not yet implemented)");
            eprintln!("  --only-lex             Stop after lexing");
            eprintln!("  --only-parse           Stop after parsing");
            eprintln!("  --only-semantic        Stop after semantic analysis");
            eprintln!("  -o <file>              Set the output file path");
            eprintln!("  --emit=asm             Stop after emitting x86-64 assembly (.s)");
            eprintln!("  --emit=obj             Stop after assembling an object file (.o)");
            eprintln!("  --emit=exe             Link a runnable executable (default)");
            eprintln!("  -S, --emit-asm         Alias for --emit=asm");
            eprintln!("  -O0|-O1|-O2|-O3        Set optimization level");
            eprintln!("  --opt-level 0|1|2|3    Set optimization level");
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

/// O que o pipeline deve produzir como artefato final.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum EmitKind {
    /// Para após gerar o assembly x86-64 (`.s`).
    Asm,
    /// Monta o assembly em um objeto ELF (`.o`) via `gcc -c`, sem linkar.
    Obj,
    /// Monta e linka um executável ELF rodável (padrão).
    #[default]
    Exe,
}

impl EmitKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "asm" => Some(Self::Asm),
            "obj" => Some(Self::Obj),
            "exe" => Some(Self::Exe),
            _ => None,
        }
    }
}

struct CliArgs {
    input_file: Option<String>,
    output_file: Option<String>,
    dump_tokens: bool,
    dump_ast: bool,
    dump_ir: bool,
    only_lex: bool,
    only_parse: bool,
    only_semantic: bool,
    emit: EmitKind,
    opt_level: OptLevel,
}

impl CliArgs {
    fn parse(args: &[String]) -> Self {
        let mut cli = CliArgs {
            input_file: None,
            output_file: None,
            dump_tokens: false,
            dump_ast: false,
            dump_ir: false,
            only_lex: false,
            only_parse: false,
            only_semantic: false,
            emit: EmitKind::default(),
            opt_level: OptLevel::default(),
        };

        let mut i = 1;
        while i < args.len() {
            let arg = &args[i];

            match arg.as_str() {
                "--dump-tokens" => cli.dump_tokens = true,
                "--dump-ast" => cli.dump_ast = true,
                "--dump-ir" => cli.dump_ir = true,
                "--only-lex" => cli.only_lex = true,
                "--only-parse" => cli.only_parse = true,
                "--only-semantic" => cli.only_semantic = true,
                "-S" | "--emit-asm" => cli.emit = EmitKind::Asm,
                "-o" => {
                    i += 1;
                    let Some(path) = args.get(i) else {
                        eprintln!("error: missing value for -o");
                        exit(64);
                    };
                    cli.output_file = Some(path.clone());
                }
                _ if arg.starts_with("--emit=") => {
                    let value = arg.strip_prefix("--emit=").unwrap();
                    cli.emit = EmitKind::parse(value).unwrap_or_else(|| {
                        eprintln!("error: invalid --emit value: {value} (expected asm|obj|exe)");
                        exit(64);
                    });
                }
                "-O" | "--opt-level" => {
                    i += 1;
                    let Some(level) = args.get(i) else {
                        eprintln!("error: missing value for {arg}");
                        exit(64);
                    };
                    cli.opt_level = OptLevel::parse(level).unwrap_or_else(|| {
                        eprintln!("error: invalid optimization level: {level}");
                        exit(64);
                    });
                }
                _ if arg
                    .strip_prefix("-O")
                    .filter(|level| !level.is_empty())
                    .is_some() =>
                {
                    let level = arg.strip_prefix("-O").unwrap();
                    cli.opt_level = OptLevel::parse(level).unwrap_or_else(|| {
                        eprintln!("error: invalid optimization level: {arg}");
                        exit(64);
                    });
                }
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

            i += 1;
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

#[derive(Debug)]
struct IoError(String);

impl ToReport for IoError {
    fn to_report(&self) -> Report {
        Report::new(&format!("I/O error: {}", self.0))
    }
}

#[derive(Debug)]
struct LinkError(String);

impl ToReport for LinkError {
    fn to_report(&self) -> Report {
        Report::new(&format!("link/assemble error: {}", self.0))
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

    // ── Stage 4: Optimization ────────────────────────────────────────────────
    // A geração de TAC/CFG ainda não está integrada; a etapa de otimização
    // opera sobre uma instância vazia de `Cfg` até essa integração existir.
    let mut cfg = Cfg::new();
    let opt_pipeline = pipeline_for_level(args.opt_level);
    opt_pipeline.run(&mut cfg, 10);

    // ── Stage 5: Code generation (x86-64 / AT&T) ─────────────────────────────
    let tac_program = lower_program(&program).map_err(|e| Box::new(e) as Box<dyn ToReport>)?;
    let asm = last::emit_program(&tac_program).map_err(|e| Box::new(e) as Box<dyn ToReport>)?;

    let output_path = output_path_for(&args.input_file, &args.output_file, args.emit);
    emit_artifact(&asm, &output_path, args.emit)?;
    eprintln!(
        "emitted {}: {}",
        emit_kind_label(args.emit),
        output_path.display()
    );

    Ok(())
}

/// Resolve o caminho de saída final, respeitando `-o` quando fornecido e
/// caindo para um default específico de cada `EmitKind` caso contrário.
fn output_path_for(
    input: &Option<String>,
    output_override: &Option<String>,
    emit: EmitKind,
) -> PathBuf {
    if let Some(path) = output_override {
        return PathBuf::from(path);
    }

    match emit {
        EmitKind::Exe => PathBuf::from("a.out"),
        EmitKind::Asm | EmitKind::Obj => {
            let input = input.clone().unwrap_or_else(|| "crusty.out".to_string());
            let mut path = PathBuf::from(input);
            path.set_extension(if emit == EmitKind::Asm { "s" } else { "o" });
            path
        }
    }
}

fn emit_kind_label(emit: EmitKind) -> &'static str {
    match emit {
        EmitKind::Asm => "assembly",
        EmitKind::Obj => "object file",
        EmitKind::Exe => "executable",
    }
}

/// Escreve o assembly gerado no destino final, invocando `gcc` para montar
/// (`--emit=obj`) ou montar+linkar (`--emit=exe`) quando necessário.
fn emit_artifact(asm: &str, output_path: &Path, emit: EmitKind) -> Result<(), Box<dyn ToReport>> {
    if emit == EmitKind::Asm {
        return std::fs::write(output_path, asm)
            .map_err(|e| Box::new(IoError(e.to_string())) as Box<dyn ToReport>);
    }

    let asm_path = write_temp_asm(asm)?;
    let result = match emit {
        EmitKind::Obj => run_gcc(&["-c", "-x", "assembler-with-cpp"], &asm_path, output_path),
        EmitKind::Exe => run_gcc(&[], &asm_path, output_path),
        EmitKind::Asm => unreachable!(),
    };
    let _ = std::fs::remove_file(&asm_path);
    result
}

/// Grava o assembly em um arquivo temporário único, usado como entrada do `gcc`.
fn write_temp_asm(asm: &str) -> Result<PathBuf, Box<dyn ToReport>> {
    let mut path = env::temp_dir();
    path.push(format!("crusty_{}.s", std::process::id()));
    std::fs::write(&path, asm)
        .map_err(|e| Box::new(IoError(e.to_string())) as Box<dyn ToReport>)?;
    Ok(path)
}

/// Invoca `gcc` com flags extras (ex.: `-c -x assembler-with-cpp` para gerar
/// objeto) sobre `asm_path`, escrevendo o resultado em `output_path`.
fn run_gcc(
    extra_args: &[&str],
    asm_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn ToReport>> {
    let status = Command::new("gcc")
        .args(extra_args)
        .arg(asm_path)
        .arg("-o")
        .arg(output_path)
        .status()
        .map_err(|e| {
            Box::new(LinkError(format!("falha ao invocar gcc: {e}"))) as Box<dyn ToReport>
        })?;

    if !status.success() {
        return Err(Box::new(LinkError(format!(
            "gcc terminou com status {status}"
        ))));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_default_opt_level() {
        let parsed = CliArgs::parse(&args(&["crusty", "main.c"]));
        assert_eq!(parsed.input_file, Some("main.c".to_string()));
        assert_eq!(parsed.opt_level, OptLevel::O0);
    }

    #[test]
    fn parses_short_opt_level_forms() {
        assert_eq!(
            CliArgs::parse(&args(&["crusty", "-O2", "main.c"])).opt_level,
            OptLevel::O2
        );
        assert_eq!(
            CliArgs::parse(&args(&["crusty", "-O", "3", "main.c"])).opt_level,
            OptLevel::O3
        );
    }

    #[test]
    fn parses_long_opt_level_form() {
        let parsed = CliArgs::parse(&args(&["crusty", "--opt-level", "1", "main.c"]));
        assert_eq!(parsed.opt_level, OptLevel::O1);
    }

    #[test]
    fn defaults_to_emit_exe() {
        let parsed = CliArgs::parse(&args(&["crusty", "main.c"]));
        assert_eq!(parsed.emit, EmitKind::Exe);
        assert_eq!(parsed.output_file, None);
    }

    #[test]
    fn parses_output_flag() {
        let parsed = CliArgs::parse(&args(&["crusty", "-o", "prog", "main.c"]));
        assert_eq!(parsed.output_file, Some("prog".to_string()));
        assert_eq!(parsed.input_file, Some("main.c".to_string()));
    }

    #[test]
    fn parses_emit_flag_variants() {
        assert_eq!(
            CliArgs::parse(&args(&["crusty", "--emit=asm", "main.c"])).emit,
            EmitKind::Asm
        );
        assert_eq!(
            CliArgs::parse(&args(&["crusty", "--emit=obj", "main.c"])).emit,
            EmitKind::Obj
        );
        assert_eq!(
            CliArgs::parse(&args(&["crusty", "--emit=exe", "main.c"])).emit,
            EmitKind::Exe
        );
        assert_eq!(
            CliArgs::parse(&args(&["crusty", "-S", "main.c"])).emit,
            EmitKind::Asm
        );
        assert_eq!(
            CliArgs::parse(&args(&["crusty", "--emit-asm", "main.c"])).emit,
            EmitKind::Asm
        );
    }

    #[test]
    fn output_path_defaults_per_emit_kind() {
        let input = Some("foo/main.c".to_string());
        assert_eq!(
            output_path_for(&input, &None, EmitKind::Asm),
            PathBuf::from("foo/main.s")
        );
        assert_eq!(
            output_path_for(&input, &None, EmitKind::Obj),
            PathBuf::from("foo/main.o")
        );
        assert_eq!(
            output_path_for(&input, &None, EmitKind::Exe),
            PathBuf::from("a.out")
        );
    }

    #[test]
    fn output_path_respects_override() {
        let input = Some("main.c".to_string());
        let output = Some("custom_out".to_string());
        for emit in [EmitKind::Asm, EmitKind::Obj, EmitKind::Exe] {
            assert_eq!(
                output_path_for(&input, &output, emit),
                PathBuf::from("custom_out")
            );
        }
    }
}
