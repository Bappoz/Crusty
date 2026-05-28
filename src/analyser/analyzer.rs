use crate::common::ast::ast::{QualifierType, Type};
use crate::common::ast::expr::{Expr, MemberAccess};
use crate::common::errors::types::{CompilerError, SemanticError, SemanticErrorKind};
use crate::analyser::symbol_table::SymbolTable;

pub struct Analyser{
    pub symbols: SymbolTable,
}

impl Analyser{
    // Instâcia Analyser com a estrutura de struct_table
    pub fn new() -> Self {
        Self{
            symbols: SymbolTable::new(),
        }
    }

    pub fn check_expr(&mut self, expr: &Expr) -> Result<QualifierType, CompilerError> {
        match expr{
            Expr::Ident(name, span) => {
                self.symbols
                    .lookup(name)
                    .map(|symbol| symbol.ty.clone())
                    .ok_or_else(|| {
                        CompilerError::Semantic(SemanticError {
                            span: span.clone(),
                            kind: SemanticErrorKind::UndefinedVariable(name.clone()),
                        })
                    })
            }
            Expr::Member(left_expr, access_kind, field_name, span) => {
                let left_type = self.check_expr(left_expr)?;

                let struct_name = match access_kind {
                    MemberAccess::Direct => {
                        if let Type::Struct(name) = &left_type.ty { // se o valor dentro do left_type for Struct extraia o nome e salve em name
                            name.clone() // copia o nome de dentro da ast para passar para struct_table
                        }else{
                            return Err(CompilerError::Semantic(SemanticError{
                                span: span.clone(),
                                kind: SemanticErrorKind::TypeMismatch {
                                    expected: "Esperava uma Struct".to_string(),
                                    found: format!("{:?}", left_type.ty),
                                }
                            }));
                        }
                    }
                    MemberAccess::Pointer => {
                        if let Type::Pointer(inner_type) = &left_type.ty {
                            if let Type::Struct(name) = &**inner_type {
                                name.clone()
                            } else{ // se o left_type for ponteiro mas não um acesso a struct
                                return Err(CompilerError::Semantic(SemanticError{
                                    span: span.clone(),
                                    kind: SemanticErrorKind::TypeMismatch {
                                        expected: "Esperava um ponteiro para struct (*struct)".to_string(),
                                        found: format!("{:?}", left_type.ty),
                                    },
                                }));
                            }
                        }else{
                            return Err(CompilerError::Semantic(SemanticError {
                                span: span.clone(),
                                kind: SemanticErrorKind::TypeMismatch {
                                    expected: "Esperava um ponteiro".to_string(),
                                    found: format!("{:?}", left_type.ty),
                                },
                            }));
                        }
                    }
                };

                // Busca na tabela
                if let Some(fields) = self.symbols.lookup_struct(&struct_name) {

                    if let Some((field_type, _)) = fields.iter().find(|(_, name)| name == field_name){ // iter() criar um iterador que é um objeto que vai percorrer os elemntos do vetor
                                                                                                      // find(| |) seleciona o que está dentro do | | e compara
                        
                        Ok(field_type.clone())
                    }else{
                        Err(CompilerError::Semantic(SemanticError {
                            span: span.clone(),
                            kind: SemanticErrorKind::TypeMismatch{
                                expected: format!("campo '{}' em struct '{}'", field_name, struct_name),
                                found: "campo nao encontrado".to_string(),
                            },
                        }))
                    }
                }else{
                    Err(CompilerError::Semantic(SemanticError {
                        span: span.clone(),
                        kind: SemanticErrorKind::TypeMismatch{
                            expected: format!("Esperava uma estrutura chamada '{}'", struct_name),
                            found: "Struct indefinida".to_string(),
                        },
                    }))
                }
            }
            _=>todo!(),
            
        }
    }

    
}