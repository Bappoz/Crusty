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
    /// `true` quando o símbolo veio de um protótipo sem implementação correspondente ainda.
    pub prototype_only: bool,
    /// `false` enquanto a variável nunca foi referenciada em uma leitura.
    /// Usado para emitir o aviso `unused-variable` ao sair do escopo.
    pub used: bool,
    /// `false` enquanto a variável não recebeu inicializador nem atribuição.
    /// Usado para emitir o aviso `may-be-uninitialized` no primeiro uso em leitura.
    pub initialized: bool,
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

    /// Remove o escopo atual e devolve seus símbolos, permitindo inspecioná-los
    /// (por exemplo, para emitir avisos de `unused-variable` ao fim do escopo).
    pub fn drain_scope(&mut self) -> Option<HashMap<String, Symbol>> {
        self.scopes.pop()
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

    /// Registra um símbolo de função no escopo atual, lidando com o caso de protótipo prévio:
    /// - Se não existe: declara normalmente.
    /// - Se existe como `prototype_only`: compara assinaturas e, se compatível, marca como implementado.
    /// - Se existe como `prototype_only` mas assinatura diverge: emite `PrototypeMismatch`.
    /// - Se existe sem ser `prototype_only`: `Redeclaration`.
    pub fn declare_or_upgrade(
        &mut self,
        symbol: Symbol,
        expected_sig: &str,
        found_sig: &str,
    ) -> Result<(), CompilerError> {
        let scope = self.scopes.last_mut().expect("nenhum escopo ativado");
        match scope.get_mut(&symbol.name) {
            None => {
                scope.insert(symbol.name.clone(), symbol);
                Ok(())
            }
            Some(existing) if existing.prototype_only => {
                if existing.ty == symbol.ty && existing.params == symbol.params {
                    if !symbol.prototype_only {
                        // Definição completa compatível: remove o flag de protótipo pendente.
                        existing.prototype_only = false;
                    }
                    // Protótipo idêntico redeclarado: ignorado silenciosamente (C permite).
                    Ok(())
                } else {
                    Err(CompilerError::Semantic(SemanticError {
                        span: symbol.decl_span.clone(),
                        kind: SemanticErrorKind::PrototypeMismatch {
                            name: symbol.name.clone(),
                            expected: expected_sig.to_string(),
                            found: found_sig.to_string(),
                        },
                    }))
                }
            }
            Some(_) => Err(CompilerError::Semantic(SemanticError {
                span: symbol.decl_span.clone(),
                kind: SemanticErrorKind::Redeclaration(symbol.name.clone()),
            })),
        }
    }

    /// Retorna todos os símbolos do escopo global (primeiro escopo) que ainda são `prototype_only`.
    pub fn dangling_prototypes(&self) -> Vec<&Symbol> {
        self.scopes
            .first()
            .map(|scope| scope.values().filter(|s| s.prototype_only).collect())
            .unwrap_or_default()
    }

    /// Busca o escopo do mais interno para o mais externo
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    /// Busca mutável, do escopo mais interno para o mais externo.
    /// Permite marcar `used`/`initialized` no símbolo que efetivamente resolve o nome.
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.scopes.iter_mut().rev().find_map(|s| s.get_mut(name))
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
