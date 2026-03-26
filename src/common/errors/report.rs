#[warn(unused_imports)]
use crate::common::errors::error_data::{Label, Source, Span};

pub struct Report {
    pub message: String,
    pub span: Option<Span>,
    pub labels: Vec<Label>,
    pub help: Option<String>,
}

impl Report {
    pub fn new(msg: &str) -> Self {
        Self {
            message: msg.to_string(),
            span: None,
            labels: vec![],
            help: None,
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
}

pub trait ToReport {
    fn to_report(&self) -> Report;
}
