#[warn(unused_imports)]
use crate::common::errors::error_data::{Label, Span};

#[derive(Debug)]
pub struct Report {
    pub message: String,
    pub span: Option<Span>,
    pub labels: Vec<Label>,
    pub help: Option<String>,
    pub system: Option<String>,
}

impl Report {
    pub fn new(msg: &str) -> Self {
        Self {
            message: msg.to_string(),
            span: None,
            labels: vec![],
            help: None,
            system: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_label(mut self, span: Span, msg: String) -> Self {
        self.labels.push(Label { span, message: msg });
        self
    }

    pub fn with_help(mut self, msg: &str) -> Self {
        self.help = Some(msg.to_string());
        self
    }

    pub fn with_system_error(mut self, msg: &str) -> Self {
        self.system = Some(format!("[SYSTEM ERROR] {}", msg));
        self
    }
}

pub trait ToReport: std::fmt::Debug {
    fn to_report(&self) -> Report;
}
