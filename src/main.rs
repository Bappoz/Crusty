use std::env;
use std::process::exit;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    /*
        match args.len() {
            1 => run_prompt(),
            2 => run_file(&args[1]),
            _ => {
                eprintln!("Usage: jlox [script]");
                exit(64);
            }
        }
    */
    Ok(())
}
