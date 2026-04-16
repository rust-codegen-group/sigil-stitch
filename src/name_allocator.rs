use std::collections::{HashMap, HashSet};

/// A scoped name allocator for generating collision-free variable names.
///
/// Used during import resolution and code generation to ensure no two
/// distinct entities share the same name in the same scope.
#[derive(Debug, Clone)]
pub struct NameAllocator {
    /// tag -> allocated name
    allocated: HashMap<String, String>,
    /// all names currently in use
    used: HashSet<String>,
}

impl NameAllocator {
    /// Create a new empty name allocator.
    pub fn new() -> Self {
        Self {
            allocated: HashMap::new(),
            used: HashSet::new(),
        }
    }

    /// Allocate a unique name based on the suggestion.
    /// If the suggestion is taken, appends "_", "__", etc.
    /// The tag allows later retrieval of the allocated name.
    ///
    /// Returns the allocated name.
    pub fn allocate(&mut self, suggestion: &str, tag: &str) -> String {
        if let Some(existing) = self.allocated.get(tag) {
            return existing.clone();
        }

        let mut candidate = suggestion.to_string();
        while self.used.contains(&candidate) {
            candidate.push('_');
        }

        self.used.insert(candidate.clone());
        self.allocated.insert(tag.to_string(), candidate.clone());
        candidate
    }

    /// Get a previously allocated name by its tag.
    pub fn get(&self, tag: &str) -> Option<&str> {
        self.allocated.get(tag).map(|s| s.as_str())
    }

    /// Clone for nested scopes. Child scope sees parent names as used
    /// but allocations are independent.
    pub fn clone_for_scope(&self) -> Self {
        Self {
            allocated: HashMap::new(),
            used: self.used.clone(),
        }
    }
}

impl Default for NameAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_unique() {
        let mut alloc = NameAllocator::new();
        assert_eq!(alloc.allocate("foo", "tag1"), "foo");
        assert_eq!(alloc.get("tag1"), Some("foo"));
    }

    #[test]
    fn test_allocate_collision() {
        let mut alloc = NameAllocator::new();
        assert_eq!(alloc.allocate("foo", "tag1"), "foo");
        assert_eq!(alloc.allocate("foo", "tag2"), "foo_");
        assert_eq!(alloc.allocate("foo", "tag3"), "foo__");
    }

    #[test]
    fn test_allocate_idempotent() {
        let mut alloc = NameAllocator::new();
        assert_eq!(alloc.allocate("foo", "tag1"), "foo");
        assert_eq!(alloc.allocate("foo", "tag1"), "foo");
    }

    #[test]
    fn test_clone_for_scope() {
        let mut parent = NameAllocator::new();
        parent.allocate("foo", "tag1");

        let mut child = parent.clone_for_scope();
        // Child sees "foo" as used.
        assert_eq!(child.allocate("foo", "child_tag"), "foo_");
        // But child doesn't see parent's tag.
        assert_eq!(child.get("tag1"), None);
        // Parent is unaffected.
        assert_eq!(parent.get("tag1"), Some("foo"));
    }
}
