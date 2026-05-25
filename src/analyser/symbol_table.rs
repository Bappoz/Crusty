use crate::common::ast::ast::QualifierType;
use crate::common::errors::{
    error_data::Span,
    types::{CompilerError, SemanticError, SemanticErrorKind},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: QualifierType,
    pub mutable: bool,
    pub decl_span: Span,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    /// Declara um símbolo no escopo atual. Erro se já declarado no mesmo escopo.
    pub fn declare(&mut self, symbol: Symbol) -> Result<(), CompilerError> {
        let scope = self.scopes.last_mut().expect("nenhum escopo ativado");
        if scope.contains_key(&symbol.name) {
            return Err(CompilerError::Semantic(SemanticError {
                span: symbol.decl_span.clone(),
                kind: SemanticErrorKind::Redeclaration(symbol.name.clone()),
            }));
        }
        scope.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    /// Busca o escopo do mais interno para o mais externo
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    /// Busca apenas o escopo atual
    pub fn lookup_current_scope(&self, name: &str) -> Option<&Symbol> {
        self.scopes.last()?.get(name)
    }
}
