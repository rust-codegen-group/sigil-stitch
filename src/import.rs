//! Import resolution data structures.
//!
//! These types represent the result of import collection and resolution:
//! deduplicated, conflict-resolved, ready for language-specific rendering.

use std::collections::{HashMap, HashSet};

use crate::error::SigilStitchError;

/// A single resolved import entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    pub(crate) entries: Vec<ImportEntry>,
}

impl From<Vec<ImportEntry>> for ImportGroup {
    fn from(entries: Vec<ImportEntry>) -> Self {
        Self { entries }
    }
}

/// Raw import reference collected from a prepared CodeBlock tree walk.
/// Not yet resolved (no dedup).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// Stable identity of one claim inside an import-alias conflict set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImportAliasClaimId(usize);

/// Strength of one requested local import binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportAliasRequestKind {
    /// Exact binding supplied through an explicit `ImportSpec`.
    Exact,
    /// Soft alias requested by `TypeName::with_alias()`.
    Preferred,
    /// Soft request for the imported symbol's natural name.
    Natural,
}

/// One semantic import participating as a peer in a local-name conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAliasClaim {
    id: ImportAliasClaimId,
    module: String,
    name: String,
    requested_name: String,
    request_kind: ImportAliasRequestKind,
}

impl ImportAliasClaim {
    /// Identity to return in [`ImportAliasAssignment`].
    pub fn id(&self) -> ImportAliasClaimId {
        self.id
    }

    /// Module path of the semantic import.
    pub fn module(&self) -> &str {
        &self.module
    }

    /// Original imported symbol.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Local binding requested before conflict resolution.
    pub fn requested_name(&self) -> &str {
        &self.requested_name
    }

    /// Whether the request is exact, preferred, or natural.
    pub fn request_kind(&self) -> ImportAliasRequestKind {
        self.request_kind
    }
}

/// Peer imports requesting one ambiguous local binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAliasConflict {
    requested_name: String,
    claims: Vec<ImportAliasClaim>,
}

impl ImportAliasConflict {
    /// Ambiguous local binding requested by this class.
    pub fn requested_name(&self) -> &str {
        &self.requested_name
    }

    /// Stable peer claims in source encounter order.
    pub fn claims(&self) -> &[ImportAliasClaim] {
        &self.claims
    }
}

/// Read-only view passed once to a selected alias-conflict resolver.
pub struct ImportAliasConflicts<'a> {
    conflicts: &'a [ImportAliasConflict],
    reserved_names: &'a [String],
}

impl<'a> ImportAliasConflicts<'a> {
    /// Every ambiguous request class in stable order.
    pub fn conflicts(&self) -> &'a [ImportAliasConflict] {
        self.conflicts
    }

    /// Names already assigned by imports outside the ambiguous classes.
    pub fn reserved_names(&self) -> &'a [String] {
        self.reserved_names
    }
}

/// One complete local-name assignment returned by a conflict resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAliasAssignment {
    /// Claim being assigned.
    pub claim_id: ImportAliasClaimId,
    /// Final local binding for that claim.
    pub local_name: String,
}

impl ImportAliasAssignment {
    /// Assign `local_name` to `claim_id`.
    pub fn new(claim_id: ImportAliasClaimId, local_name: impl Into<String>) -> Self {
        Self {
            claim_id,
            local_name: local_name.into(),
        }
    }
}

/// Diagnostic returned deliberately by a custom conflict resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAliasRejection {
    message: String,
}

impl ImportAliasRejection {
    /// Construct a resolver rejection without exposing partial assignments.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Resolver-provided diagnostic text.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Execution-time policy for assigning every ambiguous import claim atomically.
pub trait ImportAliasConflictResolver {
    /// Assign every claim in every conflict class exactly once.
    fn resolve(
        &self,
        conflicts: &ImportAliasConflicts<'_>,
    ) -> Result<Vec<ImportAliasAssignment>, ImportAliasRejection>;
}

/// Built-in stable module-prefix alias policy.
#[derive(Debug, Default, Clone, Copy)]
pub struct ModulePrefixImportAliasResolver;

impl ImportAliasConflictResolver for ModulePrefixImportAliasResolver {
    fn resolve(
        &self,
        conflicts: &ImportAliasConflicts<'_>,
    ) -> Result<Vec<ImportAliasAssignment>, ImportAliasRejection> {
        let reserved: HashSet<&str> = conflicts
            .reserved_names()
            .iter()
            .map(String::as_str)
            .collect();
        let mut assignments = Vec::new();
        for conflict in conflicts.conflicts() {
            let keeper = conflict
                .claims()
                .iter()
                .enumerate()
                .filter(|(_, claim)| {
                    claim.request_kind() == ImportAliasRequestKind::Exact
                        || !reserved.contains(conflict.requested_name())
                })
                .min_by_key(|(index, claim)| {
                    let strength = match claim.request_kind() {
                        ImportAliasRequestKind::Exact => 0,
                        ImportAliasRequestKind::Preferred => 1,
                        ImportAliasRequestKind::Natural => 2,
                    };
                    (strength, *index)
                })
                .map(|(index, _)| index);

            for (index, claim) in conflict.claims().iter().enumerate() {
                let local_name = if Some(index) == keeper {
                    claim.requested_name().to_string()
                } else {
                    format!("{}{}", module_to_prefix(claim.module()), claim.name())
                };
                assignments.push(ImportAliasAssignment::new(claim.id(), local_name));
            }
        }
        Ok(assignments)
    }
}

impl ImportGroup {
    /// Create a new empty import group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve collected and explicit imports with the built-in fallible policy.
    ///
    /// Direct callers that will render an import header must then call the
    /// selected [`crate::lang::CodeLang::validate_resolved_imports`] hook.
    pub fn try_resolve(
        refs: &[ImportRef],
        explicit: Vec<ImportEntry>,
    ) -> Result<Self, SigilStitchError> {
        resolve_fallible(refs, explicit, &ModulePrefixImportAliasResolver, true)
    }

    /// Resolve collected and explicit imports with a borrowed execution-time policy.
    ///
    /// Direct callers that will render an import header must then call the
    /// selected [`crate::lang::CodeLang::validate_resolved_imports`] hook.
    pub fn try_resolve_with(
        refs: &[ImportRef],
        explicit: Vec<ImportEntry>,
        resolver: &dyn ImportAliasConflictResolver,
    ) -> Result<Self, SigilStitchError> {
        resolve_fallible(refs, explicit, resolver, false)
    }

    /// Read-only access to the resolved import entries.
    pub fn entries(&self) -> &[ImportEntry] {
        &self.entries
    }

    /// Resolve a list of raw import references into a deduplicated ImportGroup.
    /// First-encountered wins the simple name; later duplicates get aliases.
    /// Preferred aliases from `TypeName::with_alias()` take precedence.
    #[deprecated(note = "legacy 0.6.8 infallible resolver; use ImportGroup::try_resolve in 0.7")]
    pub fn resolve(refs: &[ImportRef]) -> Self {
        let mut entries = Vec::new();
        let mut claimed: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        for import_ref in refs {
            resolve_ref(import_ref, &mut claimed, &mut seen, &mut entries);
        }

        Self { entries }
    }

    /// Resolve import references, merging with explicit (user-specified) entries.
    ///
    /// Explicit entries are processed first so their aliases and names take
    /// precedence over auto-generated aliases from conflict resolution.
    #[deprecated(note = "legacy 0.6.8 infallible resolver; use ImportGroup::try_resolve in 0.7")]
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
            resolve_ref(import_ref, &mut claimed, &mut seen, &mut entries);
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

#[derive(Debug)]
struct PendingImport {
    order: usize,
    entry: ImportEntry,
    requested_name: String,
    request_kind: ImportAliasRequestKind,
    historical_reservation: Option<String>,
}

fn resolve_fallible(
    refs: &[ImportRef],
    explicit: Vec<ImportEntry>,
    resolver: &dyn ImportAliasConflictResolver,
    preserve_historical_reservations: bool,
) -> Result<ImportGroup, SigilStitchError> {
    let mut pending: Vec<PendingImport> = Vec::new();
    let mut passthrough = Vec::new();
    let mut passthrough_identities = HashSet::new();
    let mut pending_by_identity: HashMap<(String, String), usize> = HashMap::new();
    let mut order = 0;

    for entry in explicit {
        validate_module_path(&entry.module)?;
        if entry.is_side_effect && entry.is_wildcard {
            return Err(SigilStitchError::InvalidImportAliasAssignments {
                reason: "one import entry cannot be both side-effect and wildcard".to_string(),
            });
        }
        if entry.is_side_effect || entry.is_wildcard {
            if !entry.name.is_empty() || entry.alias.is_some() || entry.is_type_only {
                return Err(SigilStitchError::InvalidImportAliasAssignments {
                    reason: "side-effect and wildcard imports cannot carry a name, alias, or type-only marker"
                        .to_string(),
                });
            }
            let identity = (
                entry.module.clone(),
                entry.is_side_effect,
                entry.is_wildcard,
            );
            if passthrough_identities.insert(identity) {
                passthrough.push((order, entry));
            }
            order += 1;
            continue;
        }
        validate_import_text(&entry.name, "explicit import name")?;
        let identity = (entry.module.clone(), entry.name.clone());
        let requested_name = entry.resolved_name().to_string();
        validate_import_text(&requested_name, "explicit local binding")?;
        if let Some(existing_index) = pending_by_identity.get(&identity).copied() {
            let existing = &mut pending[existing_index];
            if existing.requested_name != requested_name {
                return Err(SigilStitchError::ImportAliasConflict {
                    requested_name: entry.name.clone(),
                    reason: "one explicit semantic import requires multiple exact local bindings"
                        .to_string(),
                });
            }
            existing.entry.is_type_only &= entry.is_type_only;
            order += 1;
            continue;
        }
        let historical_reservation = entry.alias.as_ref().map(|_| entry.name.clone());
        let pending_index = pending.len();
        pending.push(PendingImport {
            order,
            entry,
            requested_name,
            request_kind: ImportAliasRequestKind::Exact,
            historical_reservation,
        });
        pending_by_identity.insert(identity, pending_index);
        order += 1;
    }

    for import_ref in refs {
        validate_module_path(&import_ref.module)?;
        validate_import_text(&import_ref.name, "imported name")?;
        let identity = (import_ref.module.clone(), import_ref.name.clone());
        if let Some(existing_index) = pending_by_identity.get(&identity).copied() {
            pending[existing_index].entry.is_type_only &= import_ref.is_type_only;
            continue;
        }
        let (requested_name, request_kind, historical_reservation) = match &import_ref.alias {
            Some(alias) => {
                validate_import_text(alias, "preferred import alias")?;
                (
                    alias.clone(),
                    ImportAliasRequestKind::Preferred,
                    Some(import_ref.name.clone()),
                )
            }
            None => (
                import_ref.name.clone(),
                ImportAliasRequestKind::Natural,
                None,
            ),
        };
        let pending_index = pending.len();
        pending.push(PendingImport {
            order,
            entry: ImportEntry {
                module: import_ref.module.clone(),
                name: import_ref.name.clone(),
                alias: import_ref.alias.clone(),
                is_type_only: import_ref.is_type_only,
                is_side_effect: false,
                is_wildcard: false,
            },
            requested_name,
            request_kind,
            historical_reservation,
        });
        pending_by_identity.insert(identity, pending_index);
        order += 1;
    }

    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    let mut group_by_name = HashMap::new();
    for (index, import) in pending.iter().enumerate() {
        let group_index = *group_by_name
            .entry(import.requested_name.clone())
            .or_insert_with(|| {
                groups.push((import.requested_name.clone(), Vec::new()));
                groups.len() - 1
            });
        groups[group_index].1.push(index);
    }

    for (requested_name, members) in &groups {
        let exact_count = members
            .iter()
            .filter(|index| pending[**index].request_kind == ImportAliasRequestKind::Exact)
            .count();
        if exact_count > 1 {
            return Err(SigilStitchError::ImportAliasConflict {
                requested_name: requested_name.clone(),
                reason: "multiple explicit imports require the same exact local binding"
                    .to_string(),
            });
        }
    }

    let historical: HashMap<String, usize> = if preserve_historical_reservations {
        let mut reservations = HashMap::new();
        for import in &pending {
            if let Some(name) = &import.historical_reservation {
                reservations.entry(name.clone()).or_insert(import.order);
            }
        }
        reservations
    } else {
        HashMap::new()
    };

    let mut conflict_groups = Vec::new();
    let mut conflicting_indices = HashSet::new();
    for (requested_name, members) in groups {
        let conflicts_with_earlier_reservation = members.len() == 1
            && historical
                .get(&requested_name)
                .is_some_and(|reservation_order| pending[members[0]].order > *reservation_order);
        if members.len() <= 1 && !conflicts_with_earlier_reservation {
            continue;
        }
        let claims = members
            .iter()
            .map(|index| {
                conflicting_indices.insert(*index);
                let import = &pending[*index];
                ImportAliasClaim {
                    id: ImportAliasClaimId(*index),
                    module: import.entry.module.clone(),
                    name: import.entry.name.clone(),
                    requested_name: import.requested_name.clone(),
                    request_kind: import.request_kind,
                }
            })
            .collect();
        conflict_groups.push(ImportAliasConflict {
            requested_name,
            claims,
        });
    }

    let mut final_names: HashMap<usize, String> = pending
        .iter()
        .enumerate()
        .filter(|(index, _)| !conflicting_indices.contains(index))
        .map(|(index, import)| (index, import.requested_name.clone()))
        .collect();
    let mut reserved_names: Vec<String> = final_names.values().cloned().collect();
    if preserve_historical_reservations {
        reserved_names.extend(historical.keys().cloned());
    }
    reserved_names.sort();
    reserved_names.dedup();

    if !conflict_groups.is_empty() {
        let view = ImportAliasConflicts {
            conflicts: &conflict_groups,
            reserved_names: &reserved_names,
        };
        let assignments = resolver.resolve(&view).map_err(|rejection| {
            SigilStitchError::ImportAliasResolverRejected {
                reason: rejection.message().to_string(),
            }
        })?;

        for assignment in assignments {
            let index = assignment.claim_id.0;
            if !conflicting_indices.contains(&index) {
                return Err(SigilStitchError::InvalidImportAliasAssignments {
                    reason: format!("assignment refers to unknown claim {}", index),
                });
            }
            if final_names.insert(index, assignment.local_name).is_some() {
                return Err(SigilStitchError::InvalidImportAliasAssignments {
                    reason: format!("claim {} was assigned more than once", index),
                });
            }
        }
    }

    if final_names.len() != pending.len() {
        return Err(SigilStitchError::InvalidImportAliasAssignments {
            reason: "resolver did not assign every conflicting claim".to_string(),
        });
    }

    let mut claimed = HashMap::<String, usize>::new();
    for (index, import) in pending.iter_mut().enumerate() {
        let final_name = final_names
            .remove(&index)
            .expect("complete assignment checked");
        validate_import_text(&final_name, "resolved import binding")?;
        if import.request_kind == ImportAliasRequestKind::Exact
            && final_name != import.requested_name
        {
            return Err(SigilStitchError::InvalidImportAliasAssignments {
                reason: format!("exact claim {} changed its requested binding", index),
            });
        }
        if let Some(other) = claimed.insert(final_name.clone(), index) {
            return Err(SigilStitchError::InvalidImportAliasAssignments {
                reason: format!(
                    "claims {} and {} resolve to the same local binding {:?}",
                    other, index, final_name
                ),
            });
        }
        import.entry.alias = (final_name != import.entry.name).then_some(final_name);
    }

    let mut ordered: Vec<(usize, ImportEntry)> = pending
        .into_iter()
        .map(|import| (import.order, import.entry))
        .chain(passthrough)
        .collect();
    ordered.sort_by_key(|(order, _)| *order);
    Ok(ImportGroup {
        entries: ordered.into_iter().map(|(_, entry)| entry).collect(),
    })
}

fn validate_import_text(value: &str, subject: &str) -> Result<(), SigilStitchError> {
    if value.trim().is_empty()
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '\u{2028}' | '\u{2029}'))
    {
        return Err(SigilStitchError::InvalidImportAliasAssignments {
            reason: format!("{subject} must be non-blank and control-free"),
        });
    }
    Ok(())
}

/// Process a single import reference: dedup, detect conflicts, assign aliases.
fn resolve_ref(
    import_ref: &ImportRef,
    claimed: &mut std::collections::HashMap<String, String>,
    seen: &mut std::collections::HashSet<(String, String)>,
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
        if ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}') {
            return Err(crate::error::SigilStitchError::InvalidModulePath {
                message: format!("Module path contains invalid character: {:?}", ch),
            });
        }
        match ch {
            '\'' | '"' | '`' | ';' | '{' | '}' | '(' | ')' => {
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
#[allow(deprecated)] // Exercises the frozen 0.6.8 resolver contract.
mod tests {
    use std::cell::Cell;

    use super::*;

    fn import_ref(module: &str, name: &str, alias: Option<&str>) -> ImportRef {
        ImportRef {
            module: module.to_string(),
            name: name.to_string(),
            is_type_only: true,
            alias: alias.map(str::to_string),
        }
    }

    fn explicit(module: &str, name: &str, alias: Option<&str>) -> ImportEntry {
        ImportEntry {
            module: module.to_string(),
            name: name.to_string(),
            alias: alias.map(str::to_string),
            is_type_only: true,
            is_side_effect: false,
            is_wildcard: false,
        }
    }

    struct Resolver<F>(F);

    impl<F> ImportAliasConflictResolver for Resolver<F>
    where
        F: Fn(
            &ImportAliasConflicts<'_>,
        ) -> Result<Vec<ImportAliasAssignment>, ImportAliasRejection>,
    {
        fn resolve(
            &self,
            conflicts: &ImportAliasConflicts<'_>,
        ) -> Result<Vec<ImportAliasAssignment>, ImportAliasRejection> {
            (self.0)(conflicts)
        }
    }

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
        let group = ImportGroup::resolve(&refs);
        assert_eq!(group.entries().len(), 2);
        // First wins simple name.
        assert!(group.entries()[0].alias.is_none());
        assert_eq!(group.entries()[0].name, "User");
        // Second gets alias.
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
        assert!(validate_module_path("foo\0bar").is_err());
        assert!(validate_module_path("foo\tbar").is_err());
        assert!(validate_module_path("foo\u{2028}bar").is_err());
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
        assert_eq!(group.entries().len(), 1);
        assert_eq!(group.entries()[0].alias.as_deref(), Some("MyUser"));
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
        assert_eq!(group.entries().len(), 2);
        // First gets its preferred alias.
        assert_eq!(group.entries()[0].alias.as_deref(), Some("ModelUser"));
        // Second: "User" name is claimed by ./models, so it gets auto-aliased.
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
        let group = ImportGroup::resolve_with_explicit(&refs, vec![]);
        assert_eq!(group.entries().len(), 1);
        assert_eq!(group.entries()[0].alias.as_deref(), Some("MyUser"));
        assert_eq!(group.resolved_name("./models", "User"), Some("MyUser"));
    }

    #[test]
    fn fallible_default_preserves_exact_and_prefers_preferred_then_natural() {
        let exact_conflict = ImportGroup::try_resolve(
            &[import_ref("./generated", "User", None)],
            vec![explicit("./manual", "User", None)],
        )
        .unwrap();
        assert_eq!(
            exact_conflict.resolved_name("./manual", "User"),
            Some("User")
        );
        assert_eq!(
            exact_conflict.resolved_name("./generated", "User"),
            Some("GeneratedUser")
        );

        let preferred_conflict = ImportGroup::try_resolve(
            &[
                import_ref("./preferred", "PreferredUser", Some("User")),
                import_ref("./natural", "User", None),
            ],
            vec![],
        )
        .unwrap();
        assert_eq!(
            preferred_conflict.resolved_name("./preferred", "PreferredUser"),
            Some("User")
        );
        assert_eq!(
            preferred_conflict.resolved_name("./natural", "User"),
            Some("NaturalUser")
        );

        let natural_conflict = ImportGroup::try_resolve(
            &[
                import_ref("./first", "User", None),
                import_ref("./second", "User", None),
            ],
            vec![],
        )
        .unwrap();
        assert_eq!(
            natural_conflict.resolved_name("./first", "User"),
            Some("User")
        );
        assert_eq!(
            natural_conflict.resolved_name("./second", "User"),
            Some("SecondUser")
        );
    }

    #[test]
    fn fallible_default_retains_historical_original_name_reservation() {
        let group = ImportGroup::try_resolve(
            &[
                import_ref("./models", "User", Some("ModelUser")),
                import_ref("./other", "User", None),
            ],
            vec![],
        )
        .unwrap();

        assert_eq!(group.resolved_name("./models", "User"), Some("ModelUser"));
        assert_eq!(group.resolved_name("./other", "User"), Some("OtherUser"));
    }

    #[test]
    fn incompatible_exact_claims_fail_before_resolver_invocation() {
        let calls = Cell::new(0);
        let resolver = Resolver(|_: &ImportAliasConflicts<'_>| {
            calls.set(calls.get() + 1);
            Ok(vec![])
        });

        let error = ImportGroup::try_resolve_with(
            &[],
            vec![
                explicit("./models", "User", None),
                explicit("./other", "User", None),
            ],
            &resolver,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            SigilStitchError::ImportAliasConflict { requested_name, .. }
                if requested_name == "User"
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn duplicate_explicit_identity_requires_one_exact_binding() {
        let error = ImportGroup::try_resolve(
            &[],
            vec![
                explicit("./models", "User", Some("PrimaryUser")),
                explicit("./models", "User", Some("SecondaryUser")),
            ],
        )
        .unwrap_err();

        assert!(matches!(
            error,
            SigilStitchError::ImportAliasConflict { requested_name, reason }
                if requested_name == "User" && reason.contains("multiple exact local bindings")
        ));
    }

    #[test]
    fn duplicate_semantic_imports_preserve_value_import_requirements() {
        let refs = [
            import_ref("./models", "User", None),
            ImportRef {
                is_type_only: false,
                ..import_ref("./models", "User", None)
            },
        ];
        let group = ImportGroup::try_resolve(&refs, vec![]).unwrap();

        assert_eq!(group.entries().len(), 1);
        assert!(!group.entries()[0].is_type_only);

        let explicit_then_value = ImportGroup::try_resolve(
            &[ImportRef {
                is_type_only: false,
                ..import_ref("./models", "User", None)
            }],
            vec![explicit("./models", "User", None)],
        )
        .unwrap();
        assert!(!explicit_then_value.entries()[0].is_type_only);
    }

    #[test]
    fn duplicate_passthrough_imports_keep_first_occurrence_per_form() {
        let side_effect = ImportEntry {
            module: "./shared".to_string(),
            name: String::new(),
            alias: None,
            is_type_only: false,
            is_side_effect: true,
            is_wildcard: false,
        };
        let wildcard = ImportEntry {
            is_side_effect: false,
            is_wildcard: true,
            ..side_effect.clone()
        };

        let group = ImportGroup::try_resolve(
            &[],
            vec![
                side_effect.clone(),
                side_effect.clone(),
                wildcard.clone(),
                wildcard.clone(),
            ],
        )
        .unwrap();

        assert_eq!(group.entries(), &[side_effect, wildcard]);
    }

    #[test]
    fn fallible_resolution_rejects_incoherent_import_entry_shapes() {
        let invalid_entries = [
            ImportEntry {
                module: "./module".to_string(),
                name: String::new(),
                alias: None,
                is_type_only: false,
                is_side_effect: true,
                is_wildcard: true,
            },
            ImportEntry {
                module: "./module".to_string(),
                name: "Named".to_string(),
                alias: None,
                is_type_only: false,
                is_side_effect: true,
                is_wildcard: false,
            },
            ImportEntry {
                module: "./module".to_string(),
                name: String::new(),
                alias: Some("Alias".to_string()),
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: true,
            },
        ];

        for entry in invalid_entries {
            assert!(matches!(
                ImportGroup::try_resolve(&[], vec![entry]),
                Err(SigilStitchError::InvalidImportAliasAssignments { .. })
            ));
        }
    }

    #[test]
    fn custom_resolver_receives_all_conflicts_once_and_assigns_every_peer() {
        let calls = Cell::new(0);
        let resolver = Resolver(|conflicts: &ImportAliasConflicts<'_>| {
            calls.set(calls.get() + 1);
            assert_eq!(conflicts.conflicts().len(), 2);
            assert_eq!(conflicts.reserved_names(), ["Stable"]);

            Ok(conflicts
                .conflicts()
                .iter()
                .flat_map(ImportAliasConflict::claims)
                .map(|claim| {
                    ImportAliasAssignment::new(
                        claim.id(),
                        format!("{}{}", module_to_prefix(claim.module()), claim.name()),
                    )
                })
                .collect())
        });
        let refs = [
            import_ref("./stable", "Stable", None),
            import_ref("./models", "User", None),
            import_ref("./other", "User", None),
            import_ref("./models", "Config", None),
            import_ref("./other", "Config", None),
        ];

        let group = ImportGroup::try_resolve_with(&refs, vec![], &resolver).unwrap();

        assert_eq!(calls.get(), 1);
        assert_eq!(group.resolved_name("./models", "User"), Some("ModelsUser"));
        assert_eq!(group.resolved_name("./other", "User"), Some("OtherUser"));
        assert_eq!(
            group.resolved_name("./models", "Config"),
            Some("ModelsConfig")
        );
        assert_eq!(
            group.resolved_name("./other", "Config"),
            Some("OtherConfig")
        );
    }

    #[test]
    fn custom_resolver_is_not_called_without_conflicts() {
        let calls = Cell::new(0);
        let resolver = Resolver(|_: &ImportAliasConflicts<'_>| {
            calls.set(calls.get() + 1);
            Ok(vec![])
        });

        let group = ImportGroup::try_resolve_with(
            &[
                import_ref("./models", "User", None),
                import_ref("./models", "Config", None),
            ],
            vec![],
            &resolver,
        )
        .unwrap();

        assert_eq!(calls.get(), 0);
        assert_eq!(group.resolved_name("./models", "User"), Some("User"));
    }

    fn conflicting_refs() -> [ImportRef; 2] {
        [
            import_ref("./models", "User", None),
            import_ref("./other", "User", None),
        ]
    }

    #[test]
    fn custom_resolver_rejects_missing_duplicate_unknown_and_unsafe_assignments() {
        let missing = Resolver(|_: &ImportAliasConflicts<'_>| Ok(vec![]));
        assert!(matches!(
            ImportGroup::try_resolve_with(&conflicting_refs(), vec![], &missing),
            Err(SigilStitchError::InvalidImportAliasAssignments { reason })
                if reason.contains("did not assign every")
        ));

        let duplicate = Resolver(|conflicts: &ImportAliasConflicts<'_>| {
            let claim = &conflicts.conflicts()[0].claims()[0];
            Ok(vec![
                ImportAliasAssignment::new(claim.id(), "FirstUser"),
                ImportAliasAssignment::new(claim.id(), "DuplicateUser"),
            ])
        });
        assert!(matches!(
            ImportGroup::try_resolve_with(&conflicting_refs(), vec![], &duplicate),
            Err(SigilStitchError::InvalidImportAliasAssignments { reason })
                if reason.contains("more than once")
        ));

        let unknown = Resolver(|conflicts: &ImportAliasConflicts<'_>| {
            let mut assignments: Vec<_> = conflicts.conflicts()[0]
                .claims()
                .iter()
                .map(|claim| ImportAliasAssignment::new(claim.id(), claim.requested_name()))
                .collect();
            assignments.push(ImportAliasAssignment::new(
                ImportAliasClaimId(usize::MAX),
                "UnknownUser",
            ));
            Ok(assignments)
        });
        assert!(matches!(
            ImportGroup::try_resolve_with(&conflicting_refs(), vec![], &unknown),
            Err(SigilStitchError::InvalidImportAliasAssignments { reason })
                if reason.contains("unknown claim")
        ));

        let unsafe_name = Resolver(|conflicts: &ImportAliasConflicts<'_>| {
            Ok(conflicts.conflicts()[0]
                .claims()
                .iter()
                .enumerate()
                .map(|(index, claim)| {
                    ImportAliasAssignment::new(
                        claim.id(),
                        if index == 0 { "\n" } else { "OtherUser" },
                    )
                })
                .collect())
        });
        assert!(matches!(
            ImportGroup::try_resolve_with(&conflicting_refs(), vec![], &unsafe_name),
            Err(SigilStitchError::InvalidImportAliasAssignments { reason })
                if reason.contains("non-blank and control-free")
        ));
    }

    #[test]
    fn custom_resolver_cannot_change_exact_names_or_create_secondary_collisions() {
        let change_exact = Resolver(|conflicts: &ImportAliasConflicts<'_>| {
            Ok(conflicts.conflicts()[0]
                .claims()
                .iter()
                .map(|claim| {
                    ImportAliasAssignment::new(
                        claim.id(),
                        match claim.request_kind() {
                            ImportAliasRequestKind::Exact => "ChangedUser",
                            _ => "GeneratedUser",
                        },
                    )
                })
                .collect())
        });
        assert!(matches!(
            ImportGroup::try_resolve_with(
                &[import_ref("./generated", "User", None)],
                vec![explicit("./manual", "User", None)],
                &change_exact,
            ),
            Err(SigilStitchError::InvalidImportAliasAssignments { reason })
                if reason.contains("changed its requested binding")
        ));

        let secondary_collision = Resolver(|conflicts: &ImportAliasConflicts<'_>| {
            Ok(conflicts.conflicts()[0]
                .claims()
                .iter()
                .enumerate()
                .map(|(index, claim)| {
                    ImportAliasAssignment::new(
                        claim.id(),
                        if index == 0 { "Stable" } else { "OtherUser" },
                    )
                })
                .collect())
        });
        let refs = [
            import_ref("./stable", "Stable", None),
            import_ref("./models", "User", None),
            import_ref("./other", "User", None),
        ];
        assert!(matches!(
            ImportGroup::try_resolve_with(&refs, vec![], &secondary_collision),
            Err(SigilStitchError::InvalidImportAliasAssignments { reason })
                if reason.contains("same local binding")
        ));
    }

    #[test]
    fn custom_resolver_rejection_is_propagated() {
        let resolver = Resolver(|_: &ImportAliasConflicts<'_>| {
            Err(ImportAliasRejection::new("project mapping is incomplete"))
        });

        assert!(matches!(
            ImportGroup::try_resolve_with(&conflicting_refs(), vec![], &resolver),
            Err(SigilStitchError::ImportAliasResolverRejected { reason })
                if reason == "project mapping is incomplete"
        ));
    }
}
