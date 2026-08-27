use std::fmt;

/// Crate-owned builder for deterministic lowering diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiagnosticPath(String);

impl DiagnosticPath {
    pub(crate) fn root(name: &str) -> Self {
        Self(name.to_string())
    }

    pub(crate) fn member_block(member: usize, block: usize) -> Self {
        Self(format!("member[{member}].block[{block}]"))
    }

    pub(crate) fn raw_metadata(member: usize, type_index: usize) -> Self {
        Self(format!("raw_with_imports[{member}].type[{type_index}]"))
    }

    pub(crate) fn node(&self, index: usize) -> Self {
        self.indexed("node", index)
    }

    pub(crate) fn nested(&self, index: usize) -> Self {
        self.indexed("nested", index)
    }

    pub(crate) fn sequence(&self, index: usize) -> Self {
        self.indexed("sequence", index)
    }

    pub(crate) fn child(&self, edge: &str) -> Self {
        Self(format!("{}.{edge}", self.0))
    }

    pub(crate) fn indexed(&self, edge: &str, index: usize) -> Self {
        Self(format!("{}.{edge}[{index}]", self.0))
    }
}

impl fmt::Display for DiagnosticPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
