//! Frozen 0.6.8 import resolution compatibility.
//!
//! This module keeps the deprecated infallible resolver and its historical
//! first-claim alias policy separate from the current fallible resolver.

use std::collections::{HashMap, HashSet};

use crate::import::{ImportEntry, ImportGroup, ImportRef};

/// Resolve raw import references with the frozen 0.6.8 policy.
pub(crate) fn resolve(refs: &[ImportRef]) -> ImportGroup {
    let mut entries = Vec::new();
    let mut claimed: HashMap<String, String> = HashMap::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    for import_ref in refs {
        resolve_ref(import_ref, &mut claimed, &mut seen, &mut entries);
    }

    entries.into()
}

/// Resolve raw references after reserving explicit entries with the frozen policy.
pub(crate) fn resolve_with_explicit(refs: &[ImportRef], explicit: Vec<ImportEntry>) -> ImportGroup {
    let mut entries = Vec::new();
    let mut claimed: HashMap<String, String> = HashMap::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

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
        resolve_ref(import_ref, &mut claimed, &mut seen, &mut entries);
    }

    entries.into()
}

/// Process one reference using the frozen first-claim and alias policy.
fn resolve_ref(
    import_ref: &ImportRef,
    claimed: &mut HashMap<String, String>,
    seen: &mut HashSet<(String, String)>,
    entries: &mut Vec<ImportEntry>,
) {
    let key = (import_ref.module.clone(), import_ref.name.clone());
    if seen.contains(&key) {
        return;
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
            // Same module, same name, already claimed. No alias needed.
            None
        } else {
            // Conflict: another module already claimed this simple name.
            let module_prefix = crate::import::module_to_prefix(&import_ref.module);
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

#[cfg(test)]
#[allow(deprecated)]
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
        let group = resolve(&refs);
        assert_eq!(group.entries().len(), 1);
        assert_eq!(group.entries()[0].name, "User");
        assert!(group.entries()[0].alias.is_none());
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
        let group = resolve(&refs);
        assert_eq!(group.entries().len(), 2);
        assert!(group.entries()[0].alias.is_none());
        assert_eq!(group.entries()[0].name, "User");
        assert_eq!(group.entries()[1].alias.as_deref(), Some("OtherUser"));
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
        let group = resolve(&refs);
        assert_eq!(group.resolved_name("./models", "User"), Some("User"));
        assert_eq!(group.resolved_name("./other", "User"), Some("OtherUser"));
    }

    #[test]
    fn test_module_to_prefix() {
        assert_eq!(crate::import::module_to_prefix("./models"), "Models");
        assert_eq!(
            crate::import::module_to_prefix("std::collections"),
            "Collections"
        );
        assert_eq!(crate::import::module_to_prefix("github.com/foo/bar"), "Bar");
        assert_eq!(crate::import::module_to_prefix("net/http"), "Http");
    }

    #[test]
    fn test_preferred_alias_from_ref() {
        let refs = vec![ImportRef {
            module: "./models".into(),
            name: "User".into(),
            is_type_only: false,
            alias: Some("MyUser".into()),
        }];
        let group = resolve(&refs);
        assert_eq!(group.entries().len(), 1);
        assert_eq!(group.entries()[0].alias.as_deref(), Some("MyUser"));
        assert_eq!(group.resolved_name("./models", "User"), Some("MyUser"));
    }

    #[test]
    fn test_preferred_alias_with_conflict() {
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
        let group = resolve(&refs);
        assert_eq!(group.entries().len(), 2);
        assert_eq!(group.entries()[0].alias.as_deref(), Some("ModelUser"));
        assert!(group.entries()[1].alias.is_some());
    }

    #[test]
    fn test_preferred_alias_in_resolve_with_explicit() {
        let refs = vec![ImportRef {
            module: "./models".into(),
            name: "User".into(),
            is_type_only: false,
            alias: Some("MyUser".into()),
        }];
        let group = resolve_with_explicit(&refs, vec![]);
        assert_eq!(group.entries().len(), 1);
        assert_eq!(group.entries()[0].alias.as_deref(), Some("MyUser"));
        assert_eq!(group.resolved_name("./models", "User"), Some("MyUser"));
    }
}
