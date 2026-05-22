//! Shared test harness for cross-language parameterized tests.
//!
//! Each language provides a `quote_suite.rs` implementing `LanguageTestSuite`.
//! The shared runners here replace the ~15 near-identical copies of
//! `test_control_flow`, `test_basic`, etc. across language test directories.

use std::path::PathBuf;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::spec::file_spec::FileSpec;

/// Mirror of `golden::assert_golden` so shared tests don't need to
/// depend on `#[path = "../golden.rs"]` directly.
pub fn assert_golden(name: &str, actual: &str) {
    let golden_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-goldens");
    let golden_path = golden_dir.join(name);

    if std::env::var("BLESS").is_ok() {
        if let Some(parent) = golden_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&golden_path, actual).unwrap();
        return;
    }

    if !golden_path.exists() {
        if let Some(parent) = golden_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&golden_path, actual).unwrap();
        eprintln!(
            "Golden file created: {}. Re-run to verify.",
            golden_path.display()
        );
        return;
    }

    let expected = std::fs::read_to_string(&golden_path).unwrap();
    if actual != expected {
        eprintln!("Golden file mismatch: {}", golden_path.display());
        eprintln!("--- expected ---");
        eprintln!("{expected}");
        eprintln!("--- actual ---");
        eprintln!("{actual}");
        eprintln!("---");
        eprintln!("Run with BLESS=1 to update golden files.");
        panic!("Golden file mismatch: {}", golden_path.display());
    }
}

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
