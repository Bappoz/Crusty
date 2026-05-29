pub mod top_level;
pub mod types;
pub mod var_decl;

pub use top_level::parse_program;
pub use types::{parse_array_suffix, parse_type, starts_type};
pub use var_decl::parse_var_decl;
