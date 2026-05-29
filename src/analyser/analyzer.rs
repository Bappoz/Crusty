use crate::analyser::symbol_table::SymbolTable;
use crate::common::ast::ast::{QualifierType, Type};
use crate::common::ast::expr::{Expr, MemberAccess};
use crate::common::errors::types::{CompilerError, SemanticError, SemanticErrorKind};

#[derive(Default, Debug)]
pub struct Analyser {
    pub symbols: SymbolTable,
}

impl Analyser {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
        }
    }

    pub fn check_expr(&mut self, expr: &Expr) -> Result<QualifierType, CompilerError> {
        match expr {
            Expr::Ident(name, span) => self
                .symbols
                .lookup(name)
                .map(|symbol| symbol.ty.clone())
                .ok_or_else(|| {
                    CompilerError::Semantic(SemanticError {
                        span: span.clone(),
                        kind: SemanticErrorKind::UndefinedVariable(name.clone()),
                    })
                }),
            Expr::Member(left_expr, access_kind, field_name, span) => {
                let left_type = self.check_expr(left_expr)?;

                let struct_name = match access_kind {
                    MemberAccess::Direct => {
                        if let Type::Struct(name) = &left_type.ty {
                            name.clone()
                        } else {
                            return Err(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::TypeMismatch {
                                    expected: "Struct".to_string(),
                                    found: format!("{:?}", left_type.ty),
                                },
                            }));
                        }
                    }
                    MemberAccess::Pointer => {
                        if let Type::Pointer(inner_type) = &left_type.ty {
                            if let Type::Struct(name) = &**inner_type {
                                name.clone()
                            } else {
                                return Err(CompilerError::Semantic(SemanticError {
                                    span: span.clone(),
                                    kind: SemanticErrorKind::TypeMismatch {
                                        expected: "*Struct".to_string(),
                                        found: format!("{:?}", left_type.ty),
                                    },
                                }));
                            }
                        } else {
                            return Err(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::TypeMismatch {
                                    expected: "Pointer".to_string(),
                                    found: format!("{:?}", left_type.ty),
                                },
                            }));
                        }
                    }
                };

                let fields = self.symbols.lookup_struct(&struct_name).ok_or_else(|| {
                    CompilerError::Semantic(SemanticError {
                        span: span.clone(),
                        kind: SemanticErrorKind::UndefinedStruct(struct_name.clone()),
                    })
                })?;

                fields
                    .iter()
                    .find(|(_, name)| name == field_name)
                    .map(|(field_type, _)| field_type.clone())
                    .ok_or_else(|| {
                        CompilerError::Semantic(SemanticError {
                            span: span.clone(),
                            kind: SemanticErrorKind::FieldNotFound {
                                struct_name: struct_name.clone(),
                                field_name: field_name.clone(),
                            },
                        })
                    })
            }
            _ => todo!(),
        }
    }
}
