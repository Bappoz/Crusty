use crate::common::ast::decl::Decl;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Só armazena, então o que importa é a capacidade.
    Int,
    Char,
    Float,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub decls: Vec<Decl>,
}
