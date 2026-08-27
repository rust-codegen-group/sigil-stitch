//! Shared test harness for cross-language parameterized tests.
//!
//! Each language provides a `quote_suite.rs` implementing `LanguageTestSuite`.
//! The shared runners here replace the ~15 near-identical copies of
//! `test_control_flow`, `test_basic`, etc. across language test directories.

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::spec::file_spec::FileSpec;

pub mod golden;
pub mod languages;

pub use golden::assert_golden;

/// A language participating in cross-language golden tests.
pub trait LanguageTestSuite {
    /// Build the `CodeBlock` for `test_control_flow` (if/else).
    fn control_flow_block() -> CodeBlock;

    /// Golden file path for control_flow (e.g., `"bash/macro_control_flow.bash"`).
    fn control_flow_golden_path() -> &'static str;

    /// Build the `CodeBlock` for `test_basic`.
    fn basic_block() -> CodeBlock;

    /// Golden file path for basic (e.g., `"bash/macro_basic.bash"`).
    fn basic_golden_path() -> &'static str;

    /// Render a block through `FileSpec` and return the output string.
    ///
    /// Default: `FileSpec::builder(ext).add_code(block).build().render(80)`.
    /// Override for languages that need `builder_with(ext, lang)`.
    fn render(block: CodeBlock) -> String {
        let ext = Self::file_spec_name();
        FileSpec::builder(ext)
            .add_code(block)
            .build()
            .unwrap()
            .render(80)
            .unwrap()
    }

    /// FileSpec name (e.g., `"test.bash"`).
    fn file_spec_name() -> &'static str;
}

/// Run the shared `test_control_flow` golden test for a language.
pub fn run_control_flow_test<T: LanguageTestSuite>() {
    let block = T::control_flow_block();
    let output = T::render(block);
    assert_golden(T::control_flow_golden_path(), &output);
}

/// Run the shared `test_basic` golden test for a language.
pub fn run_basic_test<T: LanguageTestSuite>() {
    let block = T::basic_block();
    let output = T::render(block);
    assert_golden(T::basic_golden_path(), &output);
}
