//! Error types for sigil-stitch.

use snafu::prelude::*;

use crate::lang::capability::SpecCapability;
use crate::spec::modifiers::TypeKind;

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

    /// A format argument does not match the corresponding specifier.
    #[snafu(display(
        "format string {format:?} argument {index} expects {expected} but got {actual}"
    ))]
    FormatArgKind {
        /// The format string that was passed.
        format: String,
        /// Zero-based argument index.
        index: usize,
        /// The expected specifier and argument kind.
        expected: String,
        /// The provided argument variant.
        actual: String,
    },

    /// A format string ends with a bare `%` marker.
    #[snafu(display("trailing format marker '%' at byte {offset} in format string {format:?}"))]
    TrailingFormatMarker {
        /// The format string that contained the marker.
        format: String,
        /// Byte offset of the trailing `%`.
        offset: usize,
    },

    /// A required name or filename field was empty.
    #[snafu(display("{builder}::build() failed: 'name' must not be empty"))]
    EmptyName {
        /// The builder type that detected the error.
        builder: &'static str,
    },

    /// Unbalanced structural indentation markers.
    #[snafu(display(
        "unbalanced structural indentation: depth is {depth} (expected 0). \
         Check %> / %< markers and begin_control_flow / end_control_flow calls."
    ))]
    UnbalancedIndent {
        /// The structural indent depth at validation time.
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

    /// A language does not support the requested type declaration kind.
    #[snafu(display("language {language:?} does not support {kind:?} declaration {type_name:?}"))]
    UnsupportedTypeKind {
        /// The language file extension.
        language: String,
        /// The unsupported declaration kind.
        kind: TypeKind,
        /// The type being emitted.
        type_name: String,
    },

    /// A language does not support one or more semantic spec capabilities.
    #[snafu(display(
        "language {language:?} does not support {capabilities:?} for type {type_name:?}"
    ))]
    UnsupportedSpecCapabilities {
        /// The language file extension.
        language: String,
        /// The type being emitted.
        type_name: String,
        /// The unsupported semantic capabilities.
        capabilities: Vec<SpecCapability>,
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

    /// A FileSpec contains one or more invalid spec members.
    ///
    /// Validation is collected rather than fail-fast: every invalid
    /// [`TypeSpec`](crate::spec::type_spec::TypeSpec) is checked and all
    /// resulting errors are returned together.
    #[snafu(display("FileSpec {filename:?} has {error_count} validation error(s): {errors:?}"))]
    FileSpecValidation {
        /// The filename of the invalid FileSpec.
        filename: String,
        /// The number of collected validation errors. Equal to `errors.len()`.
        error_count: usize,
        /// The collected member validation errors.
        errors: Vec<SigilStitchError>,
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
