//! Zsh shell language implementation.

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

fn zsh_block_open_for_intent(intent: BlockIntent, condition: &str) -> Option<&'static str> {
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

fn zsh_block_close_for_intent(intent: BlockIntent) -> Option<&'static str> {
    match intent {
        BlockIntent::If | BlockIntent::ElseIf => Some("fi"),
        BlockIntent::For | BlockIntent::While | BlockIntent::Until => Some("done"),
        BlockIntent::Case => Some("esac"),
        _ => None,
    }
}

/// Language-local opener fallback inferred from condition text for Generic
/// intents and source-constructed legacy nodes.
fn zsh_block_open_from_condition_text(condition: &str) -> Option<&'static str> {
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
fn zsh_block_close_from_condition_text(condition: &str) -> Option<&'static str> {
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

/// Zsh shell language implementation.
///
/// Zsh-specific behaviors:
/// - 4-space indentation (configurable)
/// - No semicolons (newline-separated statements)
/// - `source "path"` imports
/// - `#` comments
/// - Double-quoted string literals with `$`, `` ` ``, `\`, `"`, `!`, `%` escaping
///   (`%` is escaped because Zsh uses it for prompt expansion)
/// - `function` keyword for function declarations
/// - `{ }` brace blocks for functions
///
/// # Differences from Bash
///
/// - File extension: `.zsh` instead of `.sh`
/// - Additional reserved words for Zsh builtins (`autoload`, `compdef`, `zstyle`, etc.)
/// - String literal escaping includes `%` (Zsh prompt expansion character)
///
/// # Control Flow
///
/// Same as Bash: use manual `add()` with `%>`/`%<` for control flow blocks.
/// See [`super::bash::Bash`] for detailed examples.
#[derive(Debug, Clone)]
pub struct Zsh {
    /// Indent with this string (default: "    " -- 4 spaces).
    pub indent: String,
    /// File extension (default: "zsh").
    pub extension: String,
}

impl Default for Zsh {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            extension: "zsh".to_string(),
        }
    }
}

impl Zsh {
    /// Create a new Zsh language instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indent string (e.g., `"    "` for 4-space default, `"  "` for 2 spaces, `"\t"` for tabs).
    pub fn with_indent(mut self, s: &str) -> Self {
        self.indent = s.to_string();
        self
    }

    /// Set the file extension (default: `"zsh"`).
    pub fn with_extension(mut self, s: &str) -> Self {
        self.extension = s.to_string();
        self
    }
}

const ZSH_RESERVED: &[&str] = &[
    "autoload", "bindkey", "break", "case", "chpwd", "compdef", "continue", "coproc", "declare",
    "do", "done", "elif", "else", "emulate", "esac", "eval", "exec", "exit", "export", "fi", "for",
    "function", "if", "in", "local", "precmd", "preexec", "readonly", "return", "select", "setopt",
    "shift", "source", "then", "time", "trap", "typeset", "unset", "unsetopt", "until", "while",
    "zle", "zmodload", "zshexit", "zstyle",
];

#[deny(deprecated)]
impl RendererLang for Zsh {
    fn lower_type_name(
        &self,
        type_name: &crate::type_name::TypeName,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::type_name_lowering::zsh(type_name)
    }
    fn file_extension(&self) -> &str {
        &self.extension
    }

    fn reserved_words(&self) -> &[&str] {
        ZSH_RESERVED
    }

    fn render_string_literal(&self, s: &str) -> String {
        // Double-quoted string with Zsh-specific escaping.
        // Must escape: \, ", $, `, !, %
        // The % is escaped because Zsh uses it for prompt expansion.
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('`', "\\`")
            .replace('!', "\\!")
            .replace('%', "%%");
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
            zsh_block_open_from_condition_text(condition)
        } else {
            zsh_block_open_for_intent(intent, condition)
        };
        Ok(open.unwrap_or(" {"))
    }

    fn render_block_close(
        &self,
        intent: BlockIntent,
        condition: &str,
    ) -> Result<&str, crate::error::SigilStitchError> {
        let close = if intent == BlockIntent::Generic {
            zsh_block_close_from_condition_text(condition)
        } else {
            zsh_block_close_for_intent(intent)
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
        zsh_block_open_from_condition_text(condition)
    }

    fn block_close_for(&self, condition: &str) -> Option<&str> {
        zsh_block_close_from_condition_text(condition)
    }

    fn block_open_for_intent(&self, intent: BlockIntent, condition: &str) -> Option<&str> {
        zsh_block_open_for_intent(intent, condition)
    }

    fn block_close_for_intent(&self, intent: BlockIntent, _condition: &str) -> Option<&str> {
        zsh_block_close_for_intent(intent)
    }
}

const ZSH_FUNCTIONS: &[FunctionCapabilityProfile] =
    &[
        FunctionCapabilityProfile::new(FunctionContext::TopLevel, FunctionForm::Function, &[])
            .with_body_policy(FunctionBodyPolicy::Required)
            .with_maximum_parameters(0),
    ];

impl CodeLang for Zsh {
    fn validate_resolved_imports(
        &self,
        imports: &crate::import::ImportGroup,
    ) -> Result<(), crate::error::SigilStitchError> {
        crate::lang::import_validation::reject_aliases(self, imports)
    }
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        // Zsh has no type declaration system; use CodeBlock for shell
        // functions and control flow instead.
        LanguageCapabilities::strict().with_functions(ZSH_FUNCTIONS)
    }

    fn lower_function(
        &self,
        function: crate::spec::fun_spec::ValidatedFunction<'_>,
    ) -> Result<crate::code_block::CodeBlock, crate::error::SigilStitchError> {
        crate::lang::zsh_function_lowering::lower(self, function)
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
        let zsh = Zsh::new();
        assert_eq!(zsh.file_extension(), "zsh");
    }

    #[test]
    fn test_reserved_words() {
        let zsh = Zsh::new();
        let reserved = zsh.reserved_words();
        // Bash-shared words
        assert!(reserved.contains(&"if"));
        assert!(reserved.contains(&"fi"));
        assert!(reserved.contains(&"function"));
        // Zsh-specific words
        assert!(reserved.contains(&"autoload"));
        assert!(reserved.contains(&"compdef"));
        assert!(reserved.contains(&"zstyle"));
        assert!(reserved.contains(&"setopt"));
        assert!(reserved.contains(&"emulate"));
        assert!(!reserved.contains(&"echo"));
    }

    #[test]
    fn test_escape_reserved() {
        let zsh = Zsh::new();
        assert_eq!(zsh.escape_reserved("autoload"), "autoload_");
        assert_eq!(zsh.escape_reserved("name"), "name");
        assert_eq!(zsh.escape_reserved("setopt"), "setopt_");
    }

    #[test]
    fn test_string_literal_basic() {
        let zsh = Zsh::new();
        assert_eq!(zsh.render_string_literal("hello"), "\"hello\"");
    }

    #[test]
    fn test_string_literal_escaping() {
        let zsh = Zsh::new();
        assert_eq!(zsh.render_string_literal("$HOME"), "\"\\$HOME\"");
        assert_eq!(
            zsh.render_string_literal("say \"hi\""),
            "\"say \\\"hi\\\"\""
        );
        assert_eq!(zsh.render_string_literal("`cmd`"), "\"\\`cmd\\`\"");
        assert_eq!(zsh.render_string_literal("a\\b"), "\"a\\\\b\"");
        assert_eq!(zsh.render_string_literal("wow!"), "\"wow\\!\"");
    }

    #[test]
    fn test_string_literal_percent_escaping() {
        let zsh = Zsh::new();
        // Zsh-specific: % is escaped to %% for prompt expansion safety
        assert_eq!(zsh.render_string_literal("100%"), "\"100%%\"");
        assert_eq!(zsh.render_string_literal("%F{red}"), "\"%%F{red}\"");
    }

    #[test]
    fn test_render_imports_empty() {
        let zsh = Zsh::new();
        let imports = ImportGroup::from(vec![]);
        assert_eq!(zsh.render_imports(&imports), "");
    }

    #[test]
    fn test_render_imports_dedup() {
        let zsh = Zsh::new();
        let imports = ImportGroup {
            entries: vec![
                crate::import::ImportEntry {
                    module: "./lib/utils.zsh".into(),
                    name: "log_info".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
                crate::import::ImportEntry {
                    module: "./lib/utils.zsh".into(),
                    name: "log_error".into(),
                    alias: None,
                    is_type_only: false,
                    is_side_effect: false,
                    is_wildcard: false,
                },
            ],
        };
        assert_eq!(zsh.render_imports(&imports), "source \"./lib/utils.zsh\"");
    }

    #[test]
    fn test_doc_comment() {
        let zsh = Zsh::new();
        let doc = zsh.render_doc_comment(&["A function.", "", "Details."]);
        let lines: Vec<&str> = doc.lines().collect();
        assert_eq!(lines[0], "# A function.");
        assert_eq!(lines[1], "#");
        assert_eq!(lines[2], "# Details.");
    }

    #[test]
    fn test_no_semicolons() {
        let zsh = Zsh::new();
        assert!(!zsh.block_syntax().uses_semicolons);
    }

    #[test]
    fn test_function_keyword() {
        let zsh = Zsh::new();
        assert_eq!(
            zsh.function_keyword(DeclarationContext::TopLevel),
            "function"
        );
    }

    #[test]
    fn test_zsh_builder_fluent() {
        let zsh = Zsh::new().with_indent("\t").with_extension("sh");
        assert_eq!(zsh.file_extension(), "sh");
        assert_eq!(zsh.block_syntax().indent_unit, "\t");
    }

    #[test]
    fn test_module_separator() {
        let zsh = Zsh::new();
        assert_eq!(zsh.module_separator(), None);
    }

    #[test]
    fn test_block_intent_delimiters() {
        let zsh = Zsh::new();
        assert_eq!(
            zsh.block_open_for_intent(BlockIntent::If, "if [[ -f $1 ]]"),
            Some("; then")
        );
        assert_eq!(
            zsh.block_open_for_intent(BlockIntent::If, "if [[ -f $1 ]];"),
            Some(" then")
        );
        assert_eq!(
            zsh.block_close_for_intent(BlockIntent::If, "if [[ -f $1 ]]"),
            Some("fi")
        );
        assert_eq!(
            zsh.block_open_for_intent(BlockIntent::For, "for f in *.txt"),
            Some("; do")
        );
        assert_eq!(
            zsh.block_close_for_intent(BlockIntent::For, "for f in *.txt"),
            Some("done")
        );
        assert_eq!(
            zsh.block_open_for_intent(BlockIntent::Until, "until ready"),
            Some("; do")
        );
        assert_eq!(
            zsh.block_close_for_intent(BlockIntent::Case, "case $x in"),
            Some("esac")
        );
    }

    #[test]
    fn block_intent_negative_near_matches_keep_the_if_closer() {
        let zsh = Zsh::new();
        for condition in ["if [[ $x = in ]]", "if [[ $x = do ]]"] {
            assert_eq!(
                zsh.block_close_for_intent(BlockIntent::If, condition),
                Some("fi"),
                "condition {condition:?} must keep If intent"
            );
        }
    }
}
