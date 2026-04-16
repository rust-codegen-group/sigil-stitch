use std::marker::PhantomData;

use crate::import::ImportEntry;
use crate::lang::CodeLang;

/// Explicit import specification for manual import control.
///
/// Use `ImportSpec` when you need imports that can't be expressed through
/// `%T` / `TypeName::Importable` alone:
///
/// - **Forced imports**: import a symbol even without `%T` usage in code
/// - **Aliased imports**: specify an exact alias (e.g., `import { Foo as Bar }`)
/// - **Side-effect imports**: import a module for side effects only (no binding)
/// - **Wildcard imports**: import all exports from a module
#[derive(Debug, Clone)]
pub enum ImportSpec<L: CodeLang> {
    /// Import a specific named symbol from a module.
    Named {
        /// The module path to import from.
        module: String,
        /// The symbol name to import.
        name: String,
        /// Optional alias for the imported symbol.
        alias: Option<String>,
        /// Whether this is a type-only import (e.g., TypeScript `import type`).
        is_type_only: bool,
        /// Language phantom data.
        _phantom: PhantomData<L>,
    },
    /// Side-effect import: import a module for its side effects only.
    ///
    /// Language-specific rendering:
    /// - TypeScript/JavaScript: `import './polyfill';`
    /// - Go: `import _ "pkg"`
    /// - Python: `import module`
    /// - Dart: `import 'package:foo/init.dart';`
    /// - C/C++: `#include "header.h"`
    SideEffect {
        /// The module path to import for side effects.
        module: String,
        /// Language phantom data.
        _phantom: PhantomData<L>,
    },
    /// Wildcard import: import all exports from a module.
    ///
    /// Language-specific rendering:
    /// - Java/Kotlin: `import module.*;`
    /// - Python: `from module import *`
    /// - Rust: `use module::*;`
    /// - Go: `import . "pkg"`
    Wildcard {
        /// The module path to wildcard-import from.
        module: String,
        /// Language phantom data.
        _phantom: PhantomData<L>,
    },
}

impl<L: CodeLang> ImportSpec<L> {
    /// Create a named import.
    pub fn named(module: &str, name: &str) -> Self {
        ImportSpec::Named {
            module: module.to_string(),
            name: name.to_string(),
            alias: None,
            is_type_only: false,
            _phantom: PhantomData,
        }
    }

    /// Create a named import with an explicit alias.
    pub fn named_as(module: &str, name: &str, alias: &str) -> Self {
        ImportSpec::Named {
            module: module.to_string(),
            name: name.to_string(),
            alias: Some(alias.to_string()),
            is_type_only: false,
            _phantom: PhantomData,
        }
    }

    /// Create a type-only named import (e.g., TypeScript `import type`).
    pub fn named_type(module: &str, name: &str) -> Self {
        ImportSpec::Named {
            module: module.to_string(),
            name: name.to_string(),
            alias: None,
            is_type_only: true,
            _phantom: PhantomData,
        }
    }

    /// Create a side-effect import (no named binding).
    pub fn side_effect(module: &str) -> Self {
        ImportSpec::SideEffect {
            module: module.to_string(),
            _phantom: PhantomData,
        }
    }

    /// Create a wildcard import.
    pub fn wildcard(module: &str) -> Self {
        ImportSpec::Wildcard {
            module: module.to_string(),
            _phantom: PhantomData,
        }
    }

    /// Convert this ImportSpec into an ImportEntry for the resolution pipeline.
    pub(crate) fn into_entry(self) -> ImportEntry {
        match self {
            ImportSpec::Named {
                module,
                name,
                alias,
                is_type_only,
                ..
            } => ImportEntry {
                module,
                name,
                alias,
                is_type_only,
                is_side_effect: false,
                is_wildcard: false,
            },
            ImportSpec::SideEffect { module, .. } => ImportEntry {
                module,
                name: String::new(),
                alias: None,
                is_type_only: false,
                is_side_effect: true,
                is_wildcard: false,
            },
            ImportSpec::Wildcard { module, .. } => ImportEntry {
                module,
                name: String::new(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::typescript::TypeScript;

    #[test]
    fn test_named_import() {
        let spec = ImportSpec::<TypeScript>::named("./models", "User");
        let entry = spec.into_entry();
        assert_eq!(entry.module, "./models");
        assert_eq!(entry.name, "User");
        assert!(entry.alias.is_none());
        assert!(!entry.is_type_only);
        assert!(!entry.is_side_effect);
        assert!(!entry.is_wildcard);
    }

    #[test]
    fn test_named_as_import() {
        let spec = ImportSpec::<TypeScript>::named_as("./models", "User", "MyUser");
        let entry = spec.into_entry();
        assert_eq!(entry.name, "User");
        assert_eq!(entry.alias.as_deref(), Some("MyUser"));
    }

    #[test]
    fn test_named_type_import() {
        let spec = ImportSpec::<TypeScript>::named_type("./models", "User");
        let entry = spec.into_entry();
        assert!(entry.is_type_only);
    }

    #[test]
    fn test_side_effect_import() {
        let spec = ImportSpec::<TypeScript>::side_effect("./polyfill");
        let entry = spec.into_entry();
        assert!(entry.is_side_effect);
        assert!(entry.name.is_empty());
        assert_eq!(entry.module, "./polyfill");
    }

    #[test]
    fn test_wildcard_import() {
        let spec = ImportSpec::<TypeScript>::wildcard("./utils");
        let entry = spec.into_entry();
        assert!(entry.is_wildcard);
        assert!(entry.name.is_empty());
        assert_eq!(entry.module, "./utils");
    }
}
