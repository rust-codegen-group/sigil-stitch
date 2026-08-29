//! Bash shell language implementation.

use crate::code_node::BlockIntent;
use crate::import::ImportGroup;
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapabilityProfile, FunctionContext, FunctionForm,
    LanguageCapabilities,
};
#[expect(deprecated, reason = "0.6.8 compatibility implementation")]
use crate::lang::config::{
    BlockSyntaxConfig, EnumAndAnnotationConfig, FunctionSyntaxConfig, GenericSyntaxConfig,
    TypeDeclSyntaxConfig, TypePresentationConfig,
};
use crate::lang::{CodeLang, RendererLang};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

fn bash_block_open_for_intent(intent: BlockIntent, condition: &str) -> Option<&'static str> {
    let raw = condition.trim();
    let t = raw.trim_end_matches(';').trim();
    match intent {
        BlockIntent::If | BlockIntent::ElseIf => {
            if t.ends_with("; then") {
                Some("")
            } else if raw.ends_with(';') {
                Some(" then")
            } else {
                Some("; then")
            }
        }
        BlockIntent::For | BlockIntent::While | BlockIntent::Until => {
            if t.ends_with("; do") {
                Some("")
            } else if raw.ends_with(';') {
                Some(" do")
            } else {
                Some("; do")
            }
        }
        BlockIntent::Case => {
            if t.ends_with(" in") {
                Some("")
            } else {
                Some(" in")
            }
        }
        BlockIntent::Else => Some(""),
        _ => None,
    }
}

fn bash_block_close_for_intent(intent: BlockIntent) -> Option<&'static str> {
    match intent {
        BlockIntent::If | BlockIntent::ElseIf => Some("fi"),
        BlockIntent::For | BlockIntent::While | BlockIntent::Until => Some("done"),
        BlockIntent::Case => Some("esac"),
        _ => None,
    }
}

/// Language-local opener fallback inferred from condition text for Generic
/// intents and source-constructed legacy nodes.
fn bash_block_open_from_condition_text(condition: &str) -> Option<&'static str> {
    let raw = condition.trim();
    let t = raw.trim_end_matches(';').trim();
    if t.ends_with("; then")
        || t.ends_with("; do")
        || t.ends_with(" in")
        || t == "else"
        || t == "elif"
    {
        Some("")
    } else if t.starts_with("if ") || t.starts_with("elif ") {
        if raw.ends_with(';') {
            Some(" then")
        } else {
            Some("; then")
        }
    } else if t.starts_with("for ") || t.starts_with("while ") || t.starts_with("until ") {
        if raw.ends_with(';') {
            Some(" do")
        } else {
            Some("; do")
        }
    } else if t.starts_with("case ") {
        Some(" in")
    } else {
        None
    }
}

/// Language-local closer fallback inferred from condition text for Generic
/// intents and source-constructed legacy nodes.
fn bash_block_close_from_condition_text(condition: &str) -> Option<&'static str> {
    let t = condition.trim().trim_end_matches(';').trim();
    if t.starts_with("if ") || t.starts_with("elif ") || t == "else" {
        Some("fi")
    } else if t.starts_with("for ") || t.starts_with("while ") || t.starts_with("until ") {
        Some("done")
    } else if t.starts_with("case ") {
        Some("esac")
    } else {
        None
    }
}

/// Bash shell language implementation.
///
/// Bash-specific behaviors:
/// - 4-space indentation (configurable)
/// - No semicolons (newline-separated statements)
/// - `source "path"` imports
/// - `#` comments
/// - Double-quoted string literals with `$`, `` ` ``, `\`, `"`, `!` escaping
/// - `function` keyword for function declarations
/// - `{ }` brace blocks for functions
///
/// # Control Flow
///
/// Bash uses keyword-based block delimiters that vary per construct (`then`/`fi`,
/// `do`/`done`, `in`/`esac`). [`RendererLang::render_block_open`] and
/// [`RendererLang::render_block_close`] map the language-neutral [`BlockIntent`]
/// locally to the correct delimiters, while
/// [`RendererLang::render_branch_transition`] handles branch boundaries:
///
/// ```text
/// // Builder API — begin_control_flow/end_control_flow work directly:
/// b.begin_control_flow("if [ -f \"$file\" ];", ());   // emits "; then"
/// b.add_statement("echo \"found\"", ());
/// b.end_control_flow();                                // emits "fi"
///
/// // sigil_quote! — use { } and the backend handles the rest:
/// sigil_quote!(Bash {
///     if [ -f "$$file" ]; {
///         echo "found"
///     }
/// })
/// ```
///
/// # Shebang
///
/// Use `FileSpec::header()` for the shebang line:
///
/// ```text
/// let mut header_b = CodeBlock::builder();
/// header_b.add("#!/usr/bin/env bash\n", ());
/// header_b.add("set -euo pipefail", ());
/// fb.header(header_b.build().unwrap());
/// ```
#[derive(Debug, Clone)]
pub struct Bash {
    /// Indent with this string (default: "    " -- 4 spaces).
    pub indent: String,
    /// File extension (default: "bash"). Set to "sh" for POSIX-ish scripts.
    pub extension: String,
}

impl Default for Bash {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "bash".to_string(),
        }
    }
}

impl Bash {
    /// Create a new Bash language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"    "` for 4-space default, `"  "` for 2 spaces, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (e.g., `"bash"` or `"sh"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

const BASH_RESERVED: &[&str] = &[
    "break", "case", "continue", "coproc", "declare", "do", "done", "elif", "else", "esac", "eval",
    "exec", "exit", "export", "fi", "for", "function", "if", "in", "local", "readonly", "return",
    "select", "shift", "source", "then", "time", "trap", "typeset", "unset", "until", "while",
];

#[deny(deprecated)]
impl RendererLang for Bash {
    fn lower_type_name(
        &self,
        type_name: &crate::type_name::TypeName,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::type_name_lowering::bash(type_name)
    }
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        BASH_RESERVED
    }

    fn render_string_literal(&self, s: &str) -> String {
        // Double-quoted string with Bash-specific escaping.
        // Must escape: \, ", $, `, !
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('`', "\\`")
            .replace('!', "\\!");
        format!("\"{escaped}\"")
    }

    fn render_verbatim_string(&self, s: &str) -> String {
        // Shell interpolates by default — no wrapping quotes needed.
        // Users control quoting in the $V content itself.
        s.to_string()
    }

    fn line_comment_prefix(&self) -> &str {
        "#"
    }

    // --- Deprecated 0.6.8 config accessors ---

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_presentation(&self) -> TypePresentationConfig<'_> {
        TypePresentationConfig::default()
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
        GenericSyntaxConfig {
            constraint_keyword: "",
            constraint_separator: "",
            ..Default::default()
        }
    }

    // --- Language-owned renderer events ---

    fn indent_unit(&self) -> &str {
        &self.indent
    }

    fn render_statement_end(&self) -> Result<&str, crate::error::SigilStitchError> {
        Ok("")
    }

    fn render_block_open(
        &self,
        intent: BlockIntent,
        condition: &str,
    ) -> Result<&str, crate::error::SigilStitchError> {
        let open = if intent == BlockIntent::Generic {
            bash_block_open_from_condition_text(condition)
        } else {
            bash_block_open_for_intent(intent, condition)
        };
        Ok(open.unwrap_or(" {"))
    }

    fn render_block_close(
        &self,
        intent: BlockIntent,
        condition: &str,
    ) -> Result<&str, crate::error::SigilStitchError> {
        let close = if intent == BlockIntent::Generic {
            bash_block_close_from_condition_text(condition)
        } else {
            bash_block_close_for_intent(intent)
        };
        Ok(close.unwrap_or("}"))
    }

    fn render_branch_transition(
        &self,
        _intent: BlockIntent,
        _condition: &str,
    ) -> Result<String, crate::error::SigilStitchError> {
        Ok(String::new())
    }

    // --- Deprecated 0.6.8 renderer compatibility hooks ---

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            indent_unit: &self.indent,
            uses_semicolons: false,
            field_terminator: "",
            close_on_transition: false,
            ..Default::default()
        }
    }

    fn block_open_for(&self, condition: &str) -> Option<&str> {
        bash_block_open_from_condition_text(condition)
    }

    fn block_close_for(&self, condition: &str) -> Option<&str> {
        bash_block_close_from_condition_text(condition)
    }

    fn block_open_for_intent(&self, intent: BlockIntent, condition: &str) -> Option<&str> {
        bash_block_open_for_intent(intent, condition)
    }

    fn block_close_for_intent(&self, intent: BlockIntent, _condition: &str) -> Option<&str> {
        bash_block_close_for_intent(intent)
    }
}

const BASH_FUNCTIONS: &[FunctionCapabilityProfile] =
    &[
        FunctionCapabilityProfile::new(FunctionContext::TopLevel, FunctionForm::Function, &[])
            .with_body_policy(FunctionBodyPolicy::Required)
            .with_maximum_parameters(0),
    ];

impl CodeLang for Bash {
    fn validate_resolved_imports(
        &self,
        imports: &crate::import::ImportGroup,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::import_validation::reject_aliases(self, imports)
    }
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        // Bash has no type declaration system; use CodeBlock for shell
        // functions and control flow instead.
        LanguageCapabilities::strict().with_functions(BASH_FUNCTIONS)
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::bash_function_lowering::lower(self, function)
    }

    fn render_imports(&self, imports: &ImportGroup) -> String {
        if imports.entries().is_empty() {
            return String::new();
        }

        // Deduplicate to unique source paths.
        let mut paths: Vec<&str> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for entry in imports.entries() {
            if seen.insert(entry.module.as_str()) {
                paths.push(&entry.module);
            }
        }
        paths.sort();

        paths
            .iter()
            .map(|p| format!("source \"{p}\""))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_doc_comment(&self, lines: &[&str]) -> String {
        // Bash has no doc comment convention; use # comment blocks.
        lines
            .iter()
            .map(|line| {
                if line.is_empty() {
                    "#".to_string()
                } else {
                    format!("# {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // --- Spec support ---
    // Shell has no visibility, generics, inheritance, or interfaces.
    // Return empty/no-op for all structural methods.

    fn render_visibility(&self, _vis: Visibility, _ctx: DeclarationContext) -> &str {
        ""
    }

    fn function_keyword(&self, _ctx: DeclarationContext) -> &str {
        "function"
    }

    fn type_keyword(&self, _kind: TypeKind) -> &str {
        ""
    }

    fn methods_inside_type_body(&self, _kind: TypeKind) -> bool {
        true
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn function_syntax(&self) -> FunctionSyntaxConfig<'_> {
        FunctionSyntaxConfig {
            return_type_separator: "",
            ..Default::default()
        }
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn type_decl_syntax(&self) -> TypeDeclSyntaxConfig<'_> {
        TypeDeclSyntaxConfig::default()
    }

    #[expect(deprecated, reason = "0.6.8 compatibility implementation")]
    fn enum_and_annotation(&self) -> EnumAndAnnotationConfig<'_> {
        EnumAndAnnotationConfig::default()
    }
}

#[cfg(test)]
#[expect(deprecated, reason = "0.6.8 compatibility assertions")]
mod tests {
    use super::*;

    #[test]
    fn test_file_extension() {
        let bash = Bash::new();
        assert_eq!(bash.file_extension(), "bash");
    }

    #[test]
    fn test_reserved_words() {
        let bash = Bash::new();
        let reserved = bash.reserved_words();
        assert!(reserved.contains(&"if"));
        assert!(reserved.contains(&"fi"));
        assert!(reserved.contains(&"function"));
        assert!(reserved.contains(&"esac"));
        assert!(!reserved.contains(&"echo"));
    }

    #[test]
    fn test_escape_reserved() {
        let bash = Bash::new();
        assert_eq!(bash.escape_reserved("if"), "if_");
        assert_eq!(bash.escape_reserved("name"), "name");
        assert_eq!(bash.escape_reserved("function"), "function_");
    }

    #[test]
    fn test_string_literal_basic() {
        let bash = Bash::new();
        assert_eq!(bash.render_string_literal("hello"), "\"hello\"");
    }

    #[test]
    fn test_string_literal_escaping() {
        let bash = Bash::new();
        // Dollar sign
        assert_eq!(bash.render_string_literal("$HOME"), "\"\\$HOME\"");
        // Double quote
        assert_eq!(
            bash.render_string_literal("say \"hi\""),
            "\"say \\\"hi\\\"\""
        );
        // Backtick
        assert_eq!(bash.render_string_literal("`cmd`"), "\"\\`cmd\\`\"");
        // Backslash
        assert_eq!(bash.render_string_literal("a\\b"), "\"a\\\\b\"");
        // Exclamation
        assert_eq!(bash.render_string_literal("wow!"), "\"wow\\!\"");
    }

    #[test]
    fn test_string_literal_combined() {
        let bash = Bash::new();
        assert_eq!(
            bash.render_string_literal("$USER says \"hi!\""),
            "\"\\$USER says \\\"hi\\!\\\"\"",
        );
    }

    #[test]
    fn test_render_imports_empty() {
        let bash = Bash::new();
        let imports = ImportGroup::from(vec![]);
        assert_eq!(bash.render_imports(&imports), "");
    }

    #[test]
    fn test_render_imports_single() {
        let bash = Bash::new();
        let imports = ImportGroup {
            entries: vec![crate::import::ImportEntry {
                module: "./lib/utils.sh".into(),
                name: "log_info".into(),
                alias: None,
                is_type_only: false,
                is_side_effect: false,
                is_wildcard: false,
            }],
        };
        assert_eq!(bash.render_imports(&imports), "source \"./lib/utils.sh\"");
    }

    #[test]
    fn test_render_imports_dedup() {
        let bash = Bash::new();
        let imports = ImportGroup {
            entries: vec![
                crate::import::ImportEntry {
                    module: "./lib/utils.sh".into(),
                    name: "log_info".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                crate::import::ImportEntry {
                    module: "./lib/utils.sh".into(),
                    name: "log_error".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                crate::import::ImportEntry {
                    module: "./lib/config.sh".into(),
                    name: "load_config".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        let output = bash.render_imports(&imports);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "source \"./lib/config.sh\"");
        assert_eq!(lines[1], "source \"./lib/utils.sh\"");
    }

    #[test]
    fn test_doc_comment_single() {
        let bash = Bash::new();
        assert_eq!(bash.render_doc_comment(&["A function."]), "# A function.");
    }

    #[test]
    fn test_doc_comment_multi() {
        let bash = Bash::new();
        let doc = bash.render_doc_comment(&["First line.", "", "Second paragraph."]);
        let lines: Vec<&str> = doc.lines().collect();
        assert_eq!(lines[0], "# First line.");
        assert_eq!(lines[1], "#");
        assert_eq!(lines[2], "# Second paragraph.");
    }

    #[test]
    fn test_no_semicolons() {
        let bash = Bash::new();
        assert!(!bash.block_syntax().uses_semicolons);
    }

    #[test]
    fn test_comment_prefix() {
        let bash = Bash::new();
        assert_eq!(bash.line_comment_prefix(), "#");
    }

    #[test]
    fn test_function_keyword() {
        let bash = Bash::new();
        assert_eq!(
            bash.function_keyword(DeclarationContext::TopLevel),
            "function"
        );
    }

    #[test]
    fn test_block_delimiters() {
        let bash = Bash::new();
        assert_eq!(bash.block_syntax().block_open, " {");
        assert_eq!(bash.block_syntax().block_close, "}");
    }

    #[test]
    fn test_bash_builder_fluent() {
        let bash = Bash::new().with_indent("  ").with_extension("sh");
        assert_eq!(bash.file_extension(), "sh");
        assert_eq!(bash.block_syntax().indent_unit, "  ");
    }

    #[test]
    fn test_module_separator() {
        let bash = Bash::new();
        assert_eq!(bash.module_separator(), None);
    }

    #[test]
    fn test_block_intent_delimiters() {
        let bash = Bash::new();
        assert_eq!(
            bash.block_open_for_intent(BlockIntent::If, "if [ -f x ]"),
            Some("; then")
        );
        assert_eq!(
            bash.block_open_for_intent(BlockIntent::If, "if [ -f x ];"),
            Some(" then")
        );
        assert_eq!(
            bash.block_open_for_intent(BlockIntent::If, "if [ -f x ]; then"),
            Some("")
        );
        assert_eq!(
            bash.block_close_for_intent(BlockIntent::If, "if [ -f x ]"),
            Some("fi")
        );
        assert_eq!(
            bash.block_open_for_intent(BlockIntent::For, "for f in *.txt"),
            Some("; do")
        );
        assert_eq!(
            bash.block_close_for_intent(BlockIntent::For, "for f in *.txt"),
            Some("done")
        );
        assert_eq!(
            bash.block_open_for_intent(BlockIntent::Case, "case $x in"),
            Some("")
        );
        assert_eq!(
            bash.block_close_for_intent(BlockIntent::Case, "case $x in"),
            Some("esac")
        );
        assert_eq!(
            bash.block_open_for_intent(BlockIntent::Else, "else"),
            Some("")
        );
    }

    #[test]
    fn block_intent_negative_near_matches_keep_the_if_closer() {
        let bash = Bash::new();
        for condition in ["if [ $x = in ]", "if [ $x = do ]", "if [ $x = in ];"] {
            assert_eq!(
                bash.block_close_for_intent(BlockIntent::If, condition),
                Some("fi"),
                "condition {condition:?} must keep If intent"
            );
            assert_ne!(
                bash.block_close_for_intent(BlockIntent::If, condition),
                Some("done")
            );
            assert_ne!(
                bash.block_close_for_intent(BlockIntent::If, condition),
                Some("esac")
            );
        }
    }
}
