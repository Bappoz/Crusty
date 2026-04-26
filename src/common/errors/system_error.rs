use crate::common::errors::report::{Report, ToReport};

#[derive(Debug)]
pub struct SystemError {
    pub msg: String,
}

impl ToReport for SystemError {
    /// Converte um erro de sistema em `Report` com a mensagem técnica de falha de I/O ou ambiente.
    fn to_report(&self) -> super::report::Report {
        Report::new("System Error").with_system_error(&self.msg)
    }
}

impl From<SystemError> for Box<dyn ToReport> {
    /// Encapsula `SystemError` em um trait object `Box<dyn ToReport>` para uso polimórfico no pipeline.
    fn from(value: SystemError) -> Self {
        Box::new(value)
    }
}
