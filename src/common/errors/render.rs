// Rendering simple colors for display in error handlings

use crate::common::errors::{error_data::Source, report::Report};

fn red(s: &str) -> String {
    format!("\x1b[31m{}\x1b[0m", s)
}

fn green(s: &str) -> String {
    format!("\x1b[32m{}\x1b[0m", s)
}

fn bold(s: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", s)
}

pub fn render(report: &Report, source: &Source) {
    println!("{}: {}", red(&bold("error")), report.message);

    if let Some(span) = &report.span {
        println!(" --> {}:{}", source.filename, span.line);
        println!("  |");

        if let Some(line) = source.get_lines(span.line) {
            println!("{:2} | {}", span.line, line);
            let mut indicator = String::new();
            for _ in 0..span.column_start {
                indicator.push(' ');
            }
            for _ in span.column_start..span.column_end {
                indicator.push('^');
            }

            println!("  | {}", red(&indicator));
        }
    }

    for label in &report.labels {
        println!("  = {}", label.message);
    }
    if let Some(help) = &report.help {
        println!("  = help: {}", green(help));
    }
}
