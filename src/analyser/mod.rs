pub mod semantic;
pub mod symbol_table;

pub use semantic::analyse;
pub use semantic::analyse_with_builtins;
pub use semantic::SemanticAnalyser;
