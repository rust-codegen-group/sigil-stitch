//! Import resolution data structures.
//!
//! These types represent the result of import collection and resolution:
//! deduplicated, conflict-resolved, ready for language-specific rendering.

/// A single resolved import entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportEntry {
    /// Module path (e.g., "./models", "std::collections", "net/http").
    pub module: String,
    /// Simple name being imported (e.g., "User", "HashMap").
    /// Empty for side-effect and wildcard imports.
    pub name: String,
    /// Alias if there was a naming conflict (e.g., "OtherUser").
    pub alias: Option<String>,
    /// Whether this is a type-only import (TypeScript `import type`).
    pub is_type_only: bool,
    /// Whether this is a side-effect import (no named binding).
    pub is_side_effect: bool,
    /// Whether this is a wildcard import (e.g., `import java.util.*`).
    pub is_wildcard: bool,
}

impl ImportEntry {
    /// The name to use when referencing this import in code.
    pub fn resolved_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
}

/// A collection of resolved import entries, ready for rendering.
#[derive(Debug, Clone, Default)]
pub struct ImportGroup {
    /// The resolved import entries.
    pub entries: Vec<ImportEntry>,
}

/// Raw import reference collected from a CodeBlock tree walk (Pass 1).
/// Not yet resolved (no dedup).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportRef {
    /// The module path to import from.
    pub module: String,
    /// The name being imported.
    pub name: String,
    /// Whether this is a type-only import.
    pub is_type_only: bool,
    /// Optional preferred alias from `TypeName::with_alias()`.
    pub alias: Option<String>,
}

impl ImportGroup {
    /// Create a new empty import group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a list of raw import references into a deduplicated ImportGroup.
    /// First-encountered wins the simple name; later duplicates get aliases.
    /// Preferred aliases from `TypeName::with_alias()` take precedence.
    pub fn resolve(refs: &[ImportRef]) -> Self {
        let mut entries = Vec::new();
        // Track which simple names are claimed and by which module.
        let mut claimed: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        // Track (module, name) pairs we've already added.
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for import_ref in refs {
            let key = (import_ref.module.clone(), import_ref.name.clone());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);

            let alias = if let Some(preferred) = &import_ref.alias {
                // User explicitly requested this alias via with_alias().
                // Claim the alias name so other imports don't collide with it.
                claimed.insert(preferred.clone(), import_ref.module.clone());
                // Also claim the original name if nobody else has.
                claimed
                    .entry(import_ref.name.clone())
                    .or_insert_with(|| import_ref.module.clone());
                Some(preferred.clone())
            } else if let Some(existing_module) = claimed.get(&import_ref.name) {
                if *existing_module == import_ref.module {
                    // Same module, same name, already claimed. No alias needed.
                    None
                } else {
                    // Conflict: another module already claimed this simple name.
                    // Generate alias from module path + name.
                    let module_prefix = module_to_prefix(&import_ref.module);
                    let auto_alias = format!("{}{}", module_prefix, import_ref.name);
                    claimed.insert(auto_alias.clone(), import_ref.module.clone());
                    Some(auto_alias)
                }
            } else {
                // First to claim this simple name.
                claimed.insert(import_ref.name.clone(), import_ref.module.clone());
                None
            };

            entries.push(ImportEntry {
                module: import_ref.module.clone(),
                name: import_ref.name.clone(),
                alias,
                is_type_only: import_ref.is_type_only,
                is_side_effect: false,
                is_wildcard: false,
            });
        }

        Self { entries }
    }

    /// Resolve import references, merging with explicit (user-specified) entries.
    ///
    /// Explicit entries are processed first so their aliases and names take
    /// precedence over auto-generated aliases from conflict resolution.
    pub fn resolve_with_explicit(refs: &[ImportRef], explicit: Vec<ImportEntry>) -> Self {
        let mut entries = Vec::new();
        let mut claimed: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        // Process explicit entries first — they take precedence.
        for entry in explicit {
            if entry.is_side_effect || entry.is_wildcard {
                entries.push(entry);
                continue;
            }

            let key = (entry.module.clone(), entry.name.clone());
            seen.insert(key);

            // Claim the resolved name (alias or name).
            let resolved = entry.alias.as_deref().unwrap_or(&entry.name);
            claimed.insert(resolved.to_string(), entry.module.clone());
            // Also claim the original name to prevent auto-imports from taking it.
            if entry.alias.is_some() {
                claimed.insert(entry.name.clone(), entry.module.clone());
            }

            entries.push(entry);
        }

        // Then process auto-collected refs.
        for import_ref in refs {
            let key = (import_ref.module.clone(), import_ref.name.clone());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key);

            let alias = if let Some(preferred) = &import_ref.alias {
                // User explicitly requested this alias via with_alias().
                claimed.insert(preferred.clone(), import_ref.module.clone());
                claimed
                    .entry(import_ref.name.clone())
                    .or_insert_with(|| import_ref.module.clone());
                Some(preferred.clone())
            } else if let Some(existing_module) = claimed.get(&import_ref.name) {
                if *existing_module == import_ref.module {
                    None
                } else {
                    let module_prefix = module_to_prefix(&import_ref.module);
                    let auto_alias = format!("{}{}", module_prefix, import_ref.name);
                    claimed.insert(auto_alias.clone(), import_ref.module.clone());
                    Some(auto_alias)
                }
            } else {
                claimed.insert(import_ref.name.clone(), import_ref.module.clone());
                None
            };

            entries.push(ImportEntry {
                module: import_ref.module.clone(),
                name: import_ref.name.clone(),
                alias,
                is_type_only: import_ref.is_type_only,
                is_side_effect: false,
                is_wildcard: false,
            });
        }

        Self { entries }
    }

    /// Look up the resolved name for a given (module, name) pair.
    pub fn resolved_name(&self, module: &str, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.module == module && e.name == name)
            .map(|e| e.resolved_name())
    }
}

/// Convert a module path to a CamelCase prefix for aliasing.
/// "./models" -> "Models", "std::collections" -> "Collections",
/// "github.com/foo/bar" -> "Bar"
fn module_to_prefix(module: &str) -> String {
    let last_segment = module
        .rsplit(['/', ':', '.'])
        .find(|s| !s.is_empty())
        .unwrap_or(module);

    let mut chars = last_segment.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
    }
}

/// Validate that a module path doesn't contain injection-prone characters.
pub fn validate_module_path(path: &str) -> Result<(), crate::error::SigilStitchError> {
    if path.is_empty() {
        return Err(crate::error::SigilStitchError::InvalidModulePath {
            message: "Module path cannot be empty".to_string(),
        });
    }
    // Reject characters that could break import syntax.
    for ch in path.chars() {
        match ch {
            '\n' | '\r' | '\'' | '"' | '`' | ';' | '{' | '}' | '(' | ')' => {
                return Err(crate::error::SigilStitchError::InvalidModulePath {
                    message: format!("Module path contains invalid character: {:?}", ch),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_same_module() {
        let refs = vec![
            ImportRef {
                module: "./models".into(),
                name: "User".into(),
                is_type_only: true,
                alias: None,
            },
            ImportRef {
                module: "./models".into(),
                name: "User".into(),
                is_type_only: true,
                alias: None,
            },
        ];
        let group = ImportGroup::resolve(&refs);
        assert_eq!(group.entries.len(), 1);
        assert_eq!(group.entries[0].name, "User");
        assert!(group.entries[0].alias.is_none());
    }

    #[test]
    fn test_conflict_different_modules() {
        let refs = vec![
            ImportRef {
                module: "./models".into(),
                name: "User".into(),
                is_type_only: true,
                alias: None,
            },
            ImportRef {
                module: "./other".into(),
                name: "User".into(),
                is_type_only: true,
                alias: None,
            },
        ];
        let group = ImportGroup::resolve(&refs);
        assert_eq!(group.entries.len(), 2);
        // First wins simple name.
        assert!(group.entries[0].alias.is_none());
        assert_eq!(group.entries[0].name, "User");
        // Second gets alias.
        assert_eq!(group.entries[1].alias.as_deref(), Some("OtherUser"));
    }

    #[test]
    fn test_resolved_name_lookup() {
        let refs = vec![
            ImportRef {
                module: "./models".into(),
                name: "User".into(),
                is_type_only: true,
                alias: None,
            },
            ImportRef {
                module: "./other".into(),
                name: "User".into(),
                is_type_only: true,
                alias: None,
            },
        ];
        let group = ImportGroup::resolve(&refs);
        assert_eq!(group.resolved_name("./models", "User"), Some("User"));
        assert_eq!(group.resolved_name("./other", "User"), Some("OtherUser"));
    }

    #[test]
    fn test_module_to_prefix() {
        assert_eq!(module_to_prefix("./models"), "Models");
        assert_eq!(module_to_prefix("std::collections"), "Collections");
        assert_eq!(module_to_prefix("github.com/foo/bar"), "Bar");
        assert_eq!(module_to_prefix("net/http"), "Http");
    }

    #[test]
    fn test_validate_module_path() {
        assert!(validate_module_path("./models").is_ok());
        assert!(validate_module_path("std::collections").is_ok());
        assert!(validate_module_path("").is_err());
        assert!(validate_module_path("foo\nbar").is_err());
        assert!(validate_module_path("foo'bar").is_err());
    }

    #[test]
    fn test_preferred_alias_from_ref() {
        let refs = vec![ImportRef {
            module: "./models".into(),
            name: "User".into(),
            is_type_only: false,
            alias: Some("MyUser".into()),
        }];
        let group = ImportGroup::resolve(&refs);
        assert_eq!(group.entries.len(), 1);
        assert_eq!(group.entries[0].alias.as_deref(), Some("MyUser"));
        assert_eq!(group.resolved_name("./models", "User"), Some("MyUser"));
    }

    #[test]
    fn test_preferred_alias_with_conflict() {
        // First import has a preferred alias, second import (same name, different module)
        // should still get auto-aliased since the first claimed its alias, not the simple name.
        let refs = vec![
            ImportRef {
                module: "./models".into(),
                name: "User".into(),
                is_type_only: false,
                alias: Some("ModelUser".into()),
            },
            ImportRef {
                module: "./other".into(),
                name: "User".into(),
                is_type_only: false,
                alias: None,
            },
        ];
        let group = ImportGroup::resolve(&refs);
        assert_eq!(group.entries.len(), 2);
        // First gets its preferred alias.
        assert_eq!(group.entries[0].alias.as_deref(), Some("ModelUser"));
        // Second: "User" name is claimed by ./models, so it gets auto-aliased.
        assert!(group.entries[1].alias.is_some());
    }

    #[test]
    fn test_preferred_alias_in_resolve_with_explicit() {
        let refs = vec![ImportRef {
            module: "./models".into(),
            name: "User".into(),
            is_type_only: false,
            alias: Some("MyUser".into()),
        }];
        let group = ImportGroup::resolve_with_explicit(&refs, vec![]);
        assert_eq!(group.entries.len(), 1);
        assert_eq!(group.entries[0].alias.as_deref(), Some("MyUser"));
        assert_eq!(group.resolved_name("./models", "User"), Some("MyUser"));
    }
}
