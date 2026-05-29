use crate::common::ast::decl::Decl;
use crate::common::errors::types::CompilerError;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Só armazena, então o que importa é a capacidade.
    Int,
    Char,
    Float,
    Double,
    Void,
    Array(Box<Type>),   //Determinar os tipos de arrays
    Pointer(Box<Type>), // olhar onde pode usar
    Struct(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualifierType {
    pub ty: Type,
    pub is_const: bool,
    pub is_unsigned: bool,
}

/// AST do programa, incluindo declarações parseadas e erros coletados durante o parse.
/// Mesmo com erros, `decls` contém as declarações que foram parseadas com sucesso.
#[derive(Debug)]
pub struct Program {
    pub decls: Vec<Decl>,
    pub errors: Vec<CompilerError>,
}
