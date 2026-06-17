//! Error types for sigil-stitch.

use snafu::prelude::*;

/// Errors returned by sigil-stitch operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum SigilStitchError {
    /// Format string argument count mismatch.
    #[snafu(display(
        "format string {format:?} expects {expected} args but got {actual}\n  \
         specifiers: {expected_specifiers:?}\n  \
         arg kinds:  {actual_arg_kinds:?}"
    ))]
    FormatArgCount {
        /// The format string that was passed.
        format: String,
        /// Number of argument slots in the format string.
        expected: usize,
        /// Number of arguments actually provided.
        actual: usize,
        /// The sequence of specifier names from the format string (e.g., `["%T", "%S", "%L"]`).
        expected_specifiers: Vec<String>,
        /// The variant names of the provided args (e.g., `["TypeName", "Literal", "Literal"]`).
        actual_arg_kinds: Vec<String>,
    },

    /// A required name or filename field was empty.
    #[snafu(display("{builder}::build() failed: 'name' must not be empty"))]
    EmptyName {
        /// The builder type that detected the error.
        builder: &'static str,
    },

    /// Unbalanced begin_control_flow / end_control_flow calls.
    #[snafu(display(
        "unbalanced control flow: indent depth is {depth} (expected 0). \
         Check begin_control_flow / end_control_flow calls."
    ))]
    UnbalancedIndent {
        /// The indent depth at build time.
        depth: i32,
    },

    /// A structural indentation marker reached output as raw literal text.
    #[snafu(display(
        "unresolved indentation marker '{marker}' in {context}. \
         Pass structured fragments as CodeBlock/CodeFragment instead of raw %L text."
    ))]
    UnresolvedIndentMarker {
        /// The unresolved marker, e.g. `%>` or `%<`.
        marker: String,
        /// Where the marker was found.
        context: String,
    },

    /// Error during code rendering.
    #[snafu(display("{context}: {message}"))]
    Render {
        /// What was being rendered.
        context: String,
        /// The error message.
        message: String,
    },

    /// Error in template parsing or application.
    #[snafu(display("template error: {message}"))]
    Template {
        /// The error message.
        message: String,
    },

    /// I/O error (e.g., writing project files).
    #[snafu(display("{context}"))]
    Io {
        /// The underlying I/O error.
        source: std::io::Error,
        /// What was being done when the error occurred.
        context: String,
    },

    /// Module path validation failure.
    #[snafu(display("invalid module path: {message}"))]
    InvalidModulePath {
        /// The error message.
        message: String,
    },

    /// Invalid format specifier in a format string.
    #[snafu(display("invalid format specifier '%{specifier}' in format string {format:?}"))]
    InvalidFormatSpecifier {
        /// The format string that contained the invalid specifier.
        format: String,
        /// The unrecognized character after `%`.
        specifier: char,
    },

    /// Duplicate field name in a type specification.
    #[snafu(display("duplicate field name {field_name:?} in type {type_name:?}"))]
    DuplicateFieldName {
        /// The name of the type that contains the duplicate.
        type_name: String,
        /// The duplicated field name.
        field_name: String,
    },

    /// Invalid TypeAlias or Newtype declaration.
    #[snafu(display("invalid {kind} {type_name:?}: {reason}"))]
    InvalidTypeAlias {
        /// The kind of declaration ("TypeAlias" or "Newtype").
        kind: &'static str,
        /// The type name.
        type_name: String,
        /// The reason the declaration is invalid.
        reason: String,
    },

    /// Duplicate filename in a project specification.
    #[snafu(display("duplicate filename {filename:?} in ProjectSpec (appears {count} times)"))]
    DuplicateFileName {
        /// The duplicated filename.
        filename: String,
        /// How many times it appeared.
        count: usize,
    },

    /// FileSpec has no language set (e.g. after deserialization).
    #[snafu(display(
        "FileSpec {filename:?} has no language — call .with_lang() after deserialization \
         or use FileSpec::builder_with() to set one"
    ))]
    MissingLang {
        /// The filename of the FileSpec.
        filename: String,
    },

    /// Invalid enum declaration.
    #[snafu(display("invalid enum {type_name:?}: {reason}"))]
    InvalidEnum {
        /// The type name.
        type_name: String,
        /// The reason the declaration is invalid.
        reason: String,
    },
}
