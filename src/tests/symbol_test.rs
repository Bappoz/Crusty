// src/tests/symbol_table_test.rs

#[cfg(test)]
mod tests {
    use crate::analyser::symbol_table::{Symbol, SymbolTable};
    use crate::common::ast::ast::{QualifierType, Type};
    use crate::common::errors::error_data::Span;
    use crate::common::errors::types::CompilerError;
    use crate::common::errors::types::SemanticErrorKind;

    fn span() -> Span {
        Span {
            line: 1,
            end_line: 1,
            column_start: 1,
            column_end: 2,
        }
    }

    fn sym(name: &str, ty: Type, is_const: bool) -> Symbol {
        Symbol {
            name: name.to_string(),
            ty: QualifierType {
                ty,
                is_const,
                is_unsigned: false,
            },
            mutable: !is_const,
            params: None,
            decl_span: span(),
        }
    }

    // ── lookup ────────────────────────────────────────────────────────────────

    #[test]
    fn lookup_returns_none_for_undeclared() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        assert!(table.lookup("x").is_none());
    }

    #[test]
    fn lookup_finds_declared_symbol() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("x", Type::Int, false)).unwrap();
        assert!(table.lookup("x").is_some());
    }

    #[test]
    fn lookup_traverses_outer_scopes() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("x", Type::Int, false)).unwrap();
        table.enter_scope();
        // x foi declarado no escopo externo; deve ser encontrado no interno
        let found = table.lookup("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "x");
    }

    // ── shadowing ─────────────────────────────────────────────────────────────

    #[test]
    fn inner_scope_shadows_outer() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("x", Type::Int, false)).unwrap();

        table.enter_scope();
        table.declare(sym("x", Type::Float, false)).unwrap();

        // lookup retorna o do escopo mais interno
        let found = table.lookup("x").unwrap();
        assert!(matches!(found.ty.ty, Type::Float));
    }

    // ── redeclaration ─────────────────────────────────────────────────────────

    #[test]
    fn redeclaration_in_same_scope_errors() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("x", Type::Int, false)).unwrap();

        let err = table.declare(sym("x", Type::Int, false)).unwrap_err();
        assert!(matches!(
            err,
            CompilerError::Semantic(ref e)
                if matches!(e.kind, SemanticErrorKind::Redeclaration(_))
        ));
    }

    #[test]
    fn redeclaration_in_different_scopes_is_ok() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("x", Type::Int, false)).unwrap();
        table.enter_scope();
        // escopo diferente → não é redeclaração
        assert!(table.declare(sym("x", Type::Float, false)).is_ok());
    }

    // ── exit_scope ────────────────────────────────────────────────────────────

    #[test]
    fn exit_scope_removes_declarations() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.enter_scope();
        table.declare(sym("y", Type::Int, false)).unwrap();
        assert!(table.lookup("y").is_some());

        table.exit_scope();
        assert!(table.lookup("y").is_none());
    }

    #[test]
    fn exit_scope_preserves_outer_symbols() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("outer", Type::Int, false)).unwrap();

        table.enter_scope();
        table.declare(sym("inner", Type::Float, false)).unwrap();
        table.exit_scope();

        assert!(table.lookup("outer").is_some());
        assert!(table.lookup("inner").is_none());
    }

    // ── lookup_current_scope ──────────────────────────────────────────────────

    #[test]
    fn lookup_current_scope_ignores_outer() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("x", Type::Int, false)).unwrap();

        table.enter_scope();
        // x está no escopo externo; current_scope não deve encontrar
        assert!(table.lookup_current_scope("x").is_none());
    }

    #[test]
    fn lookup_current_scope_finds_local() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("x", Type::Int, false)).unwrap();
        assert!(table.lookup_current_scope("x").is_some());
    }

    // ── qualificadores ────────────────────────────────────────────────────────

    #[test]
    fn const_symbol_is_not_mutable() {
        let mut table = SymbolTable::new();
        table.enter_scope();
        table.declare(sym("PI", Type::Float, true)).unwrap();
        let found = table.lookup("PI").unwrap();
        assert!(!found.mutable);
        assert!(found.ty.is_const);
    }
}
