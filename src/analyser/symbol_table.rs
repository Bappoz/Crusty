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
    /// For functions, the parameter types (in order). `None` for non-functions.
    pub params: Option<Vec<QualifierType>>,
    pub decl_span: Span,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,

    struct_table: HashMap<String, Vec<(QualifierType, String)>>,
    type_aliases: HashMap<String, QualifierType>,
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

    pub fn register_struct(&mut self, name: String, fields: Vec<(QualifierType, String)>) {
        self.struct_table.insert(name, fields);
    }

    pub fn lookup_struct(&self, name: &str) -> Option<&[(QualifierType, String)]> {
        self.struct_table.get(name).map(|v| v.as_slice())
    }

    pub fn register_type_alias(&mut self, name: String, ty: QualifierType) {
        self.type_aliases.insert(name, ty);
    }

    pub fn lookup_type_alias(&self, name: &str) -> Option<&QualifierType> {
        self.type_aliases.get(name)
    }
}
