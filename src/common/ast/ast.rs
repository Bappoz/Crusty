use crate::common::ast::decl::Decl;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Só armazena, então o que importa é a capacidade.
    Int,
    Long,
    Short,
    Char,
    Float,
    Double,
    Void,
    Array(Box<Type>),
    Pointer(Box<Type>),
    Struct(String),
    Enum(String),
    Alias(String),
    /// Tipo de uma função: (tipo_retorno, [tipos_parâmetros]).
    /// Usado para registrar assinaturas de protótipos e definições na tabela de símbolos.
    Function(Box<QualifierType>, Vec<QualifierType>),
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
