use std::io::{BufRead, BufReader};
use std::process::exit;
use std::{env, fs};
use Rippler::common::errors::report::ToReport;
use Rippler::common::errors::system_error::SystemError;

fn main() -> std::io::Result<()> {
    let path = "src/examples/teste1.txt";
    if let Err(e) = run_file(path) {
        report_and_exit(e);
    }

    /*
     *
     * A logica vai ficar assim, porem usei o jeito acima para testar com um arquivo
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
    */
    Ok(())
}

// Function to run the interactive prompt, returning an error if it fails (Not implemented yet)
fn run_prompt() -> Result<(), Box<dyn ToReport>> {
    todo!()
}

// Function to run the source code, returning an error if it fails
fn run(source: String) -> Result<(), Box<dyn ToReport>> {
    for _ in source.lines() {
        for c in source.chars() {
            println!("Read Char: {}", c);
        }
    }
    Ok(())
}

//  Function to read a file and run its contents, returning an error if the file cannot be read
fn run_file(path: &str) -> Result<(), Box<dyn ToReport>> {
    let source = fs::read_to_string(path).map_err(|e| {
        Box::new(SystemError {
            msg: format!("Could not read file '{}': {}", path, e),
        }) as Box<dyn ToReport>
    })?;

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
