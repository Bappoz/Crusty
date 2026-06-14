// Rendering simple colors for display in error handlings

use crate::common::errors::report::Report;
use crate::common::errors::types::Severity;
use crate::common::errors::{error_data::Source};

/// Envolve a string `s` na sequência ANSI de cor vermelha para exibição no terminal.
pub fn red(s: &str) -> String {
    format!("\x1b[31m{}\x1b[0m", s)
}

/// Envolve a string `s` na sequência ANSI de cor amarela (usada para warnings).
pub fn yellow(s: &str) -> String {
    format!("\x1b[33m{}\x1b[0m", s)
}

/// Envolve a string `s` na sequência ANSI de cor verde para exibição no terminal.
pub fn green(s: &str) -> String {
    format!("\x1b[32m{}\x1b[0m", s)
}

/// Envolve a string `s` na sequência ANSI de negrito para exibição no terminal.
pub fn bold(s: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", s)
}

/// Imprime o `Report` formatado no terminal com localização, setas indicadoras e
/// sugestão de ajuda. A cor do rótulo (`error`/`warning`) segue a `severity`.
pub fn render(report: &Report, source: &Source, severity: Severity) {
    let (label, accent): (String, fn(&str) -> String) = match severity {
        Severity::Error => (red(&bold("error")), red),
        Severity::Warning => (yellow(&bold("warning")), yellow),
    };
    println!("{}: {}", label, report.message);

    if let Some(span) = &report.span {
        if span.end_line == span.line {
            println!(" --> {}:{}", source.filename, span.line);
        } else {
            println!(" --> {}:{}-{}", source.filename, span.line, span.end_line);
        }
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

            println!("  | {}", accent(&indicator));
        }
    }

    for label in &report.labels {
        println!("  = {}", label.message);
    }
    if let Some(help) = &report.help {
        println!("  = help: {}", green(help));
    }
}
