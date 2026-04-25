#![allow(clippy::module_inception)]

pub mod parser;
pub mod precedence;
pub mod rules;

pub use parser::Parser;
//para permitir usar: use create::parser::Parser;
