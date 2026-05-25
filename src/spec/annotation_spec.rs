//! Structured annotation builder.
//!
//! `AnnotationSpec` provides language-aware annotation construction that
//! renders with the correct prefix/suffix for each language:
//! - Java/Kotlin/TS/etc.: `@Name(args)`
//! - Rust: `#[name(args)]`
//! - C++: `[[name(args)]]`
//! - C: `__attribute__((name(args)))`
//!
//! The existing `.annotation(CodeBlock)` API remains as an escape hatch
//! for annotations that don't fit this model.

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::lang::CodeLang;
use crate::type_name::TypeName;

/// A structured annotation that renders with language-appropriate syntax.
///
/// `AnnotationSpec` produces annotations with the correct prefix and suffix
/// for each language: `@Name(args)` in Java/Kotlin/TS, `#[name(args)]` in Rust,
/// `[[name(args)]]` in C++, `__attribute__((name(args)))` in C.
///
/// Use [`AnnotationSpec::new()`] for simple names or
/// [`AnnotationSpec::importable()`] for import-tracked annotation types.
/// The existing `.annotation(CodeBlock)` API on builders remains as an
/// escape hatch for annotations that don't fit this model.
///
/// # Examples
///
/// ```
/// use sigil_stitch::spec::annotation_spec::AnnotationSpec;
/// use sigil_stitch::lang::rust::Rust;
///
/// // Simple: #[allow(dead_code)]
/// let ann = AnnotationSpec::new("allow").arg("dead_code");
///
/// // Multiple args: #[cfg(test, feature = "nightly")]
/// let ann = AnnotationSpec::new("cfg")
///     .arg("test")
///     .arg("feature = \"nightly\"");
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotationSpec {
    pub(crate) name: AnnotationName,
    pub(crate) arguments: Vec<String>,
}

/// The name of an annotation — either a simple string or an import-tracked type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum AnnotationName {
    /// A simple name string (e.g., "Override", "deprecated").
    Simple(String),
    /// An importable type name that triggers import tracking via `%T`.
    Importable(TypeName),
}

impl AnnotationSpec {
    /// Create an annotation with a simple (non-imported) name.
    ///
    /// ```text
    /// AnnotationSpec::new("deprecated")
    /// // TS: @deprecated
    /// // Rust: #[deprecated]
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: AnnotationName::Simple(name.into()),
            arguments: Vec::new(),
        }
    }

    /// Create an annotation with an import-tracked name.
    ///
    /// The `TypeName` is rendered via `%T` so the import collector picks it up.
    ///
    /// ```text
    /// AnnotationSpec::importable(
    ///     TypeName::importable("javax.annotation", "Nullable")
    /// )
    /// // Java: @Nullable (with import javax.annotation.Nullable)
    /// ```
    pub fn importable(type_name: TypeName) -> Self {
        Self {
            name: AnnotationName::Importable(type_name),
            arguments: Vec::new(),
        }
    }

    /// Add a pre-formatted argument string.
    ///
    /// Arguments are joined with `", "` inside parentheses.
    ///
    /// ```text
    /// AnnotationSpec::new("allow")
    ///     .arg("dead_code")
    /// // Rust: #[allow(dead_code)]
    ///
    /// AnnotationSpec::new("SuppressWarnings")
    ///     .arg("\"unchecked\"")
    /// // Java: @SuppressWarnings("unchecked")
    /// ```
    pub fn arg(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());
        self
    }

    /// Add multiple arguments at once.
    ///
    /// Arguments are joined with `", "` inside parentheses.
    ///
    /// ```text
    /// AnnotationSpec::new("derive")
    ///     .args(["Debug", "Clone", "Serialize"])
    /// // Rust: #[derive(Debug, Clone, Serialize)]
    /// ```
    pub fn args(mut self, arguments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.arguments
            .extend(arguments.into_iter().map(|a| a.into()));
        self
    }

    /// Emit this annotation as a `CodeBlock` using the language's annotation syntax.
    ///
    /// Called during spec `emit()` methods which have access to `&L`.
    pub fn emit(&self, lang: &dyn CodeLang) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let ea = lang.enum_and_annotation();
        let (prefix, suffix) = (ea.annotation_prefix, ea.annotation_suffix);

        // Build the argument list portion: "(arg1, arg2)" or empty.
        let args_str = if self.arguments.is_empty() {
            String::new()
        } else {
            format!("({})", self.arguments.join(", "))
        };

        match &self.name {
            AnnotationName::Simple(name) => {
                // Simple name: render directly as a literal string.
                let rendered = format!("{prefix}{name}{args_str}{suffix}");
                CodeBlock::of("%L", rendered)
            }
            AnnotationName::Importable(type_name) => {
                // Importable name: use %T so the import collector tracks it.
                // We need to build the CodeBlock manually to wrap prefix/suffix around %T.
                let mut cb = CodeBlockBuilder::new();
                if !prefix.is_empty() {
                    cb.add("%L", prefix.to_string());
                }
                cb.add("%T", type_name.clone());
                let tail = format!("{args_str}{suffix}");
                if !tail.is_empty() {
                    cb.add("%L", tail);
                }
                cb.build()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::rust::Rust;
    use crate::lang::typescript::TypeScript;

    #[test]
    fn test_simple_annotation_ts() {
        let ts = TypeScript::new();
        let ann = AnnotationSpec::new("deprecated");
        let cb = ann.emit(&ts).unwrap();
        assert!(!cb.is_empty());
    }

    #[test]
    fn test_simple_annotation_with_args() {
        let ts = TypeScript::new();
        let ann = AnnotationSpec::new("deprecated").arg("reason: 'use v2'");
        let cb = ann.emit(&ts).unwrap();
        assert!(!cb.is_empty());
    }

    #[test]
    fn test_rust_prefix() {
        let rs = Rust::new();
        let ann = AnnotationSpec::new("allow").arg("dead_code");
        let cb = ann.emit(&rs).unwrap();
        assert!(!cb.is_empty());
    }

    #[test]
    fn test_importable_annotation() {
        let ts = TypeScript::new();
        let type_name = TypeName::importable("./decorators", "Component");
        let ann = AnnotationSpec::importable(type_name);
        let cb = ann.emit(&ts).unwrap();
        assert!(!cb.is_empty());
    }

    #[test]
    fn test_arg_chaining() {
        let rs = Rust::new();
        let ann = AnnotationSpec::new("cfg")
            .arg("test")
            .arg("feature = \"nightly\"");
        let cb = ann.emit(&rs).unwrap();
        assert!(!cb.is_empty());
    }
}
