use crate::common::errors::report::{Report, ToReport};

#[derive(Debug)]
pub struct SystemError {
    pub msg: String,
}

impl ToReport for SystemError {
    fn to_report(&self) -> super::report::Report {
        Report::new("System Error").with_system_error(&self.msg)
    }
}

impl From<SystemError> for Box<dyn ToReport> {
    fn from(value: SystemError) -> Self {
        Box::new(value)
    }
}
