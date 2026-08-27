//! Crate-owned validation and materialization for language-owned type syntax.

pub(crate) mod compatibility;
pub(crate) mod materialize;
pub(crate) mod path;
pub(crate) mod structure;
pub(crate) mod validation;

pub(crate) use materialize::TypeNameMaterializer;
pub(crate) use path::DiagnosticPath;
