//! Error types for sigil-stitch.

use snafu::prelude::*;

/// Errors returned by sigil-stitch operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum SigilStitchError {
    /// Format string argument count mismatch.
    #[snafu(display("format string {format:?} expects {expected} args but got {actual}"))]
    FormatArgCount {
        /// The format string that was passed.
        format: String,
        /// Number of argument slots in the format string.
        expected: usize,
        /// Number of arguments actually provided.
        actual: usize,
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
}
