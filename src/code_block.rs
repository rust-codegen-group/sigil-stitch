use crate::code_node::{CodeNode, parts_args_to_nodes};
use crate::import::ImportRef;
use crate::lang::CodeLang;
use crate::type_name::TypeName;

/// A parsed format specifier from a format string.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) enum FormatPart {
    /// Literal text (no interpolation).
    Literal(String),
    /// `%T` - type reference (consumes an Arg::TypeName).
    Type,
    /// `%N` - name reference (consumes an Arg::Name).
    Name,
    /// `%S` - string literal (consumes an Arg::StringLit).
    StringLit,
    /// `%L` - literal/nested code block (consumes an Arg::Literal or Arg::Code).
    Literal_,
    /// `%W` - soft line break point (no argument consumed).
    Wrap,
    /// `%>` - increase indent (no argument consumed).
    Indent,
    /// `%<` - decrease indent (no argument consumed).
    Dedent,
    /// `%[` - statement begin (no argument consumed).
    StatementBegin,
    /// `%]` - statement end (no argument consumed).
    StatementEnd,
    /// Newline.
    Newline,
    /// Block open delimiter — resolved at render time via `lang.block_open()`.
    /// Emitted by control-flow builders; braces for TS/Rust/Go, colon for Python.
    BlockOpen,
    /// Block open with an overridden delimiter (not resolved via `lang.block_open()`).
    /// Emitted by `begin_control_flow_with_open` for constructs that need a
    /// different opener than the language default (e.g., Haskell `where` vs `=`).
    BlockOpenOverride(String),
    /// Block close delimiter (terminal) — resolved at render time via `lang.block_close()`.
    /// Emitted by `end_control_flow`. When non-empty, also emits a trailing newline.
    /// When empty (indent-only languages like OCaml/Haskell/Python), emits nothing.
    BlockClose,
    /// Block close delimiter (transitional) — resolved at render time via
    /// `lang.block_close()` + `" "`. Used by `next_control_flow` to emit `} else`.
    /// When `block_close()` is empty, emits nothing (Python: dedent-only transition).
    BlockCloseTransition,
}

/// An argument to a CodeBlock format string.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Arg {
    /// A type name reference (used by `%T`).
    TypeName(TypeName),
    /// A name string (used by `%N`).
    Name(String),
    /// A string literal value (used by `%S`).
    StringLit(String),
    /// A literal string value or nested code block (used by `%L`).
    Literal(String),
    /// A nested code block (used by `%L`).
    Code(CodeBlock),
}

/// An immutable code fragment with embedded type references.
///
/// `CodeBlock` is the core composition primitive in sigil-stitch. It stores a tree
/// of [`CodeNode`] nodes — self-contained IR nodes produced from format strings
/// (`%T`, `%N`, `%S`, `%L`, etc.). CodeBlocks are produced by [`CodeBlockBuilder`]
/// and consumed by [`FileSpec`](crate::spec::file_spec::FileSpec) during rendering.
/// Type references embedded via `%T` are automatically tracked for import resolution.
///
/// Use [`CodeBlock::builder()`] to construct a block incrementally, or
/// [`CodeBlock::of()`] for simple one-liners.
///
/// # Examples
///
/// ```
/// use sigil_stitch::code_block::CodeBlock;
/// use sigil_stitch::lang::typescript::TypeScript;
/// use sigil_stitch::type_name::TypeName;
///
/// // One-liner with a type reference:
/// let user = TypeName::importable("./models", "User");
/// let block = CodeBlock::of("const u: %T = getUser()", (user,)).unwrap();
///
/// // Multi-statement block via builder:
/// let mut cb = CodeBlock::builder();
/// cb.add_statement("const x = 1", ());
/// cb.add_statement("const y = 2", ());
/// let block = cb.build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CodeBlock {
    pub(crate) nodes: Vec<CodeNode>,
}

impl CodeBlock {
    /// Create a new CodeBlockBuilder.
    pub fn builder() -> CodeBlockBuilder {
        CodeBlockBuilder::new()
    }

    /// Create a CodeBlock from a single format string and arguments.
    pub fn of(format: &str, args: impl IntoArgs) -> Result<Self, crate::error::SigilStitchError> {
        let mut builder = CodeBlockBuilder::new();
        builder.add(format, args);
        builder.build()
    }

    /// Check if this code block is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Check if this code block ends with a newline or block close.
    pub(crate) fn ends_with_newline_or_block_close(&self) -> bool {
        fn check_last(nodes: &[CodeNode]) -> bool {
            match nodes.last() {
                Some(CodeNode::Newline | CodeNode::BlockClose) => true,
                Some(CodeNode::Sequence(children)) => check_last(children),
                _ => false,
            }
        }
        check_last(&self.nodes)
    }

    /// Collect all import references from this code block.
    pub fn collect_imports(&self, out: &mut Vec<ImportRef>) {
        collect_imports_from_nodes(&self.nodes, out);
    }

    /// Render this code block to a string without import resolution.
    ///
    /// Creates a temporary empty import group and renders using the given
    /// language and target line width. Useful for quick one-off rendering
    /// in tests or when import management is not needed.
    pub fn render_standalone(
        &self,
        lang: &dyn CodeLang,
        width: usize,
    ) -> Result<String, crate::error::SigilStitchError> {
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(lang, &imports, width);
        renderer.render(self)
    }
}

fn collect_imports_from_nodes(nodes: &[CodeNode], out: &mut Vec<ImportRef>) {
    for node in nodes {
        match node {
            CodeNode::TypeRef(tn) => tn.collect_imports(out),
            CodeNode::Nested(block) => block.collect_imports(out),
            CodeNode::Sequence(children) => collect_imports_from_nodes(children, out),
            _ => {}
        }
    }
}

/// Builder for constructing [`CodeBlock`] instances.
///
/// Provides methods for adding formatted code fragments, statements, control
/// flow blocks, and nested code blocks. Format strings use `%T`, `%N`, `%S`,
/// `%L` for type/name/string/literal substitution, and `%W`, `%>`, `%<` for
/// soft line breaks and indentation.
///
/// # Examples
///
/// ```
/// use sigil_stitch::code_block::CodeBlock;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let mut cb = CodeBlock::builder();
/// cb.begin_control_flow("if (x > 0)", ());
/// cb.add_statement("return x", ());
/// cb.next_control_flow("else", ());
/// cb.add_statement("return -x", ());
/// cb.end_control_flow();
/// let block = cb.build().unwrap();
/// ```
#[derive(Debug)]
pub struct CodeBlockBuilder {
    nodes: Vec<CodeNode>,
    indent_depth: i32,
    errors: Vec<crate::error::SigilStitchError>,
}

impl CodeBlockBuilder {
    /// Create a new empty code block builder.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            indent_depth: 0,
            errors: Vec::new(),
        }
    }

    /// Add a formatted code fragment.
    pub fn add(&mut self, format: &str, args: impl IntoArgs) -> &mut Self {
        let arg_vec = args.into_args();
        let parsed = match parse_format(format) {
            Ok(parts) => parts,
            Err(err) => {
                self.errors.push(err);
                return self;
            }
        };

        let consuming_specifiers: Vec<String> = parsed
            .iter()
            .filter_map(|p| match p {
                FormatPart::Type => Some("%T".to_string()),
                FormatPart::Name => Some("%N".to_string()),
                FormatPart::StringLit => Some("%S".to_string()),
                FormatPart::Literal_ => Some("%L".to_string()),
                _ => None,
            })
            .collect();

        let expected_args = consuming_specifiers.len();

        if expected_args != arg_vec.len() {
            let actual_arg_kinds: Vec<String> = arg_vec
                .iter()
                .map(|a| match a {
                    Arg::TypeName(_) => "TypeName".to_string(),
                    Arg::Name(_) => "Name".to_string(),
                    Arg::StringLit(_) => "StringLit".to_string(),
                    Arg::Literal(_) => "Literal".to_string(),
                    Arg::Code(_) => "Code".to_string(),
                })
                .collect();
            self.errors
                .push(crate::error::SigilStitchError::FormatArgCount {
                    format: format.to_string(),
                    expected: expected_args,
                    actual: arg_vec.len(),
                    expected_specifiers: consuming_specifiers,
                    actual_arg_kinds,
                });
            return self;
        }

        let new_nodes = parts_args_to_nodes(&parsed, &arg_vec);
        self.nodes.extend(new_nodes);
        self
    }

    /// Add a statement (wraps in %[...%] and appends language semicolon).
    pub fn add_statement(&mut self, format: &str, args: impl IntoArgs) -> &mut Self {
        self.nodes.push(CodeNode::StatementBegin);
        self.add(format, args);
        self.nodes.push(CodeNode::StatementEnd);
        self.nodes.push(CodeNode::Newline);
        self
    }

    /// Begin a control flow block (e.g., "if foo" -> "if foo {\n" + indent).
    pub fn begin_control_flow(&mut self, format: &str, args: impl IntoArgs) -> &mut Self {
        self.add(format, args);
        self.nodes.push(CodeNode::BlockOpen);
        self.nodes.push(CodeNode::Newline);
        self.nodes.push(CodeNode::Indent);
        self.indent_depth += 1;
        self
    }

    /// Begin a control flow block with a custom block-open string.
    ///
    /// Like [`begin_control_flow`](Self::begin_control_flow), but uses
    /// `custom_open` instead of the language's `block_open()`. Pass `""`
    /// to suppress the block opener entirely (e.g., OCaml `match x with`).
    pub fn begin_control_flow_with_open(
        &mut self,
        format: &str,
        args: impl IntoArgs,
        custom_open: &str,
    ) -> &mut Self {
        self.add(format, args);
        if !custom_open.is_empty() {
            self.nodes
                .push(CodeNode::BlockOpenOverride(custom_open.to_string()));
        }
        self.nodes.push(CodeNode::Newline);
        self.nodes.push(CodeNode::Indent);
        self.indent_depth += 1;
        self
    }

    /// Add an else/else-if clause (e.g., "} else {" or "elif ...:" for Python).
    pub fn next_control_flow(&mut self, format: &str, args: impl IntoArgs) -> &mut Self {
        self.nodes.push(CodeNode::Dedent);
        self.indent_depth -= 1;
        self.nodes.push(CodeNode::BlockCloseTransition);
        self.add(format, args);
        self.nodes.push(CodeNode::BlockOpen);
        self.nodes.push(CodeNode::Newline);
        self.nodes.push(CodeNode::Indent);
        self.indent_depth += 1;
        self
    }

    /// End a control flow block (emits "}" or nothing for Python, and decreases indent).
    pub fn end_control_flow(&mut self) -> &mut Self {
        self.nodes.push(CodeNode::Dedent);
        self.indent_depth -= 1;
        self.nodes.push(CodeNode::BlockClose);
        self
    }

    /// Add a blank line.
    pub fn add_line(&mut self) -> &mut Self {
        self.nodes.push(CodeNode::Newline);
        self
    }

    /// Add an inline comment.
    pub fn add_comment(&mut self, text: &str) -> &mut Self {
        self.nodes.push(CodeNode::Comment(text.to_string()));
        self.nodes.push(CodeNode::Newline);
        self
    }

    /// Add a nested CodeBlock inline.
    pub fn add_code(&mut self, block: CodeBlock) -> &mut Self {
        self.nodes.push(CodeNode::Nested(block));
        self
    }

    /// Build the immutable CodeBlock.
    ///
    /// Returns an error if any format string had an argument count mismatch,
    /// or if indent depth is not balanced (unmatched
    /// begin_control_flow / end_control_flow).
    pub fn build(self) -> Result<CodeBlock, crate::error::SigilStitchError> {
        if let Some(err) = self.errors.into_iter().next() {
            return Err(err);
        }
        if self.indent_depth != 0 {
            return Err(crate::error::SigilStitchError::UnbalancedIndent {
                depth: self.indent_depth,
            });
        }
        Ok(CodeBlock { nodes: self.nodes })
    }

    /// Build the CodeBlock, panicking on error.
    pub fn build_unwrap(self) -> CodeBlock {
        self.build().unwrap()
    }
}

impl Default for CodeBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a format string into FormatParts.
fn parse_format(format: &str) -> Result<Vec<FormatPart>, crate::error::SigilStitchError> {
    let mut parts = Vec::new();
    let mut current_literal = String::new();
    let mut chars = format.char_indices().peekable();

    while let Some(&(_, ch)) = chars.peek() {
        if ch == '%' {
            chars.next();
            if let Some(&(_, spec)) = chars.peek() {
                chars.next();
                let part = match spec {
                    'T' => Some(FormatPart::Type),
                    'N' => Some(FormatPart::Name),
                    'S' => Some(FormatPart::StringLit),
                    'L' => Some(FormatPart::Literal_),
                    'W' => Some(FormatPart::Wrap),
                    '>' => Some(FormatPart::Indent),
                    '<' => Some(FormatPart::Dedent),
                    '[' => Some(FormatPart::StatementBegin),
                    ']' => Some(FormatPart::StatementEnd),
                    '%' => {
                        current_literal.push('%');
                        continue;
                    }
                    _ => {
                        return Err(crate::error::SigilStitchError::InvalidFormatSpecifier {
                            format: format.to_string(),
                            specifier: spec,
                        });
                    }
                };
                if let Some(part) = part {
                    if !current_literal.is_empty() {
                        parts.push(FormatPart::Literal(std::mem::take(&mut current_literal)));
                    }
                    parts.push(part);
                }
            }
        } else if ch == '\n' {
            chars.next();
            if !current_literal.is_empty() {
                parts.push(FormatPart::Literal(std::mem::take(&mut current_literal)));
            }
            parts.push(FormatPart::Newline);
        } else {
            chars.next();
            current_literal.push(ch);
        }
    }

    if !current_literal.is_empty() {
        parts.push(FormatPart::Literal(current_literal));
    }

    Ok(parts)
}

// === IntoArgs trait and implementations ===

/// Trait for converting various types into a `Vec<Arg>` for format strings.
///
/// Implemented for `()` (no args), `TypeName`, `&str`, `String`, `CodeBlock`,
/// `NameArg`, `StringLitArg`, `Vec<Arg>`, and tuples up to 8 elements.
/// Bare strings convert to `Arg::Literal`; use [`NameArg`] or [`StringLitArg`]
/// wrappers to target `%N` or `%S` specifiers instead.
pub trait IntoArgs {
    /// Convert into a vector of format arguments.
    fn into_args(self) -> Vec<Arg>;
}

/// Empty args (for format strings with no specifiers).
impl IntoArgs for () {
    fn into_args(self) -> Vec<Arg> {
        Vec::new()
    }
}

/// Single TypeName arg.
impl IntoArgs for TypeName {
    fn into_args(self) -> Vec<Arg> {
        vec![Arg::TypeName(self)]
    }
}

/// Single string arg (as literal).
impl IntoArgs for &str {
    fn into_args(self) -> Vec<Arg> {
        vec![Arg::Literal(self.to_string())]
    }
}

impl IntoArgs for String {
    fn into_args(self) -> Vec<Arg> {
        vec![Arg::Literal(self)]
    }
}

/// Single CodeBlock arg.
impl IntoArgs for CodeBlock {
    fn into_args(self) -> Vec<Arg> {
        vec![Arg::Code(self)]
    }
}

/// Pre-built args vector (used by specs that dynamically build format strings).
impl IntoArgs for Vec<Arg> {
    fn into_args(self) -> Vec<Arg> {
        self
    }
}

/// A wrapper to explicitly mark a string as a Name arg (for `%N`).
///
/// By default, bare strings convert to `Arg::Literal` (for `%L`). Wrap with
/// `NameArg` when your format string uses `%N`.
///
/// # Examples
///
/// ```
/// use sigil_stitch::code_block::{CodeBlock, NameArg};
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let mut cb = CodeBlock::builder();
/// cb.add("this.%N()", (NameArg("getData".to_string()),));
/// let block = cb.build().unwrap();
/// ```
pub struct NameArg(pub String);

impl IntoArgs for NameArg {
    fn into_args(self) -> Vec<Arg> {
        vec![Arg::Name(self.0)]
    }
}

/// A wrapper to explicitly mark a string as a StringLit arg (for `%S`).
///
/// By default, bare strings convert to `Arg::Literal` (for `%L`). Wrap with
/// `StringLitArg` when your format string uses `%S` to emit a quoted string.
///
/// # Examples
///
/// ```
/// use sigil_stitch::code_block::{CodeBlock, StringLitArg};
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let mut cb = CodeBlock::builder();
/// cb.add_statement("const msg = %S", (StringLitArg("hello".to_string()),));
/// let block = cb.build().unwrap();
/// ```
pub struct StringLitArg(pub String);

impl IntoArgs for StringLitArg {
    fn into_args(self) -> Vec<Arg> {
        vec![Arg::StringLit(self.0)]
    }
}

// Individual Arg conversions.
impl From<TypeName> for Arg {
    fn from(tn: TypeName) -> Self {
        Arg::TypeName(tn)
    }
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Arg::Literal(s.to_string())
    }
}

impl From<String> for Arg {
    fn from(s: String) -> Self {
        Arg::Literal(s)
    }
}

impl From<CodeBlock> for Arg {
    fn from(cb: CodeBlock) -> Self {
        Arg::Code(cb)
    }
}

impl From<NameArg> for Arg {
    fn from(n: NameArg) -> Self {
        Arg::Name(n.0)
    }
}

impl From<StringLitArg> for Arg {
    fn from(s: StringLitArg) -> Self {
        Arg::StringLit(s.0)
    }
}

// Tuple implementations for IntoArgs.
// Each element must implement Into<Arg>.

macro_rules! impl_into_args_tuple {
    ($($idx:tt $T:ident),+) => {
        impl<$($T: Into<Arg>),+> IntoArgs for ($($T,)+) {
            fn into_args(self) -> Vec<Arg> {
                vec![$(self.$idx.into()),+]
            }
        }
    };
}

impl_into_args_tuple!(0 A);
impl_into_args_tuple!(0 A, 1 B);
impl_into_args_tuple!(0 A, 1 B, 2 C);
impl_into_args_tuple!(0 A, 1 B, 2 C, 3 D);
impl_into_args_tuple!(0 A, 1 B, 2 C, 3 D, 4 E);
impl_into_args_tuple!(0 A, 1 B, 2 C, 3 D, 4 E, 5 F);
impl_into_args_tuple!(0 A, 1 B, 2 C, 3 D, 4 E, 5 F, 6 G);
impl_into_args_tuple!(0 A, 1 B, 2 C, 3 D, 4 E, 5 F, 6 G, 7 H);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_node::CodeNode;

    #[test]
    fn test_parse_all_specifiers() {
        let parts = parse_format("hello %T world %N %S %L %W %> %< %[ %]").unwrap();
        assert!(parts.contains(&FormatPart::Type));
        assert!(parts.contains(&FormatPart::Name));
        assert!(parts.contains(&FormatPart::StringLit));
        assert!(parts.contains(&FormatPart::Literal_));
        assert!(parts.contains(&FormatPart::Wrap));
        assert!(parts.contains(&FormatPart::Indent));
        assert!(parts.contains(&FormatPart::Dedent));
        assert!(parts.contains(&FormatPart::StatementBegin));
        assert!(parts.contains(&FormatPart::StatementEnd));
    }

    #[test]
    fn test_parse_literal_percent() {
        let parts = parse_format("100%%").unwrap();
        assert_eq!(parts, vec![FormatPart::Literal("100%".to_string())]);
    }

    #[test]
    fn test_parse_empty() {
        let parts = parse_format("").unwrap();
        assert!(parts.is_empty());
    }

    #[test]
    fn test_parse_newlines() {
        let parts = parse_format("line1\nline2").unwrap();
        assert_eq!(
            parts,
            vec![
                FormatPart::Literal("line1".to_string()),
                FormatPart::Newline,
                FormatPart::Literal("line2".to_string()),
            ]
        );
    }

    #[test]
    fn test_builder_add_statement() {
        let mut b = CodeBlock::builder();
        b.add_statement("const x = %L", "42");
        let block = b.build().unwrap();

        assert!(!block.is_empty());
        let has_stmt_begin = block
            .nodes
            .iter()
            .any(|n| matches!(n, CodeNode::StatementBegin));
        let has_stmt_end = block
            .nodes
            .iter()
            .any(|n| matches!(n, CodeNode::StatementEnd));
        assert!(has_stmt_begin);
        assert!(has_stmt_end);
    }

    #[test]
    fn test_builder_control_flow() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow("if (x > 0)", ());
        b.add_statement("return x", ());
        b.end_control_flow();
        let block = b.build().unwrap();

        assert!(!block.is_empty());
    }

    #[test]
    fn test_builder_unbalanced_control_flow() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow("if (x)", ());
        b.add_statement("y()", ());
        // missing end_control_flow
        let result = b.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unbalanced"));
    }

    #[test]
    fn test_mismatched_arg_count() {
        let mut b = CodeBlock::builder();
        b.add("%T", ());
        let result = b.build();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expects 1 args but got 0")
        );
    }

    #[test]
    fn test_into_args_tuple() {
        let user = TypeName::importable("./models", "User");
        let args: Vec<Arg> = (user, "hello").into_args();
        assert_eq!(args.len(), 2);
        assert!(matches!(&args[0], Arg::TypeName(_)));
        assert!(matches!(&args[1], Arg::Literal(s) if s == "hello"));
    }

    #[test]
    fn test_into_args_single_typename() {
        let user = TypeName::importable("./models", "User");
        let args: Vec<Arg> = user.into_args();
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_into_args_single_str() {
        let args: Vec<Arg> = "hello".into_args();
        assert_eq!(args.len(), 1);
        assert!(matches!(&args[0], Arg::Literal(s) if s == "hello"));
    }

    #[test]
    fn test_collect_imports_from_codeblock() {
        let user = TypeName::importable("./models", "User");
        let tag = TypeName::importable("./models", "Tag");
        let mut b = CodeBlock::builder();
        b.add_statement("const u: %T = getUser()", (user,));
        b.add_statement("const t: %T = getTag()", (tag,));
        let block = b.build().unwrap();

        let mut imports = Vec::new();
        block.collect_imports(&mut imports);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].name, "User");
        assert_eq!(imports[1].name, "Tag");
    }

    #[test]
    fn test_nested_codeblock_imports() {
        let user = TypeName::importable("./models", "User");
        let mut ib = CodeBlock::builder();
        ib.add_statement("return new %T()", (user,));
        let inner = ib.build().unwrap();

        let mut ob = CodeBlock::builder();
        ob.add_code(inner);
        let outer = ob.build().unwrap();

        let mut imports = Vec::new();
        outer.collect_imports(&mut imports);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "User");
    }

    #[test]
    fn test_name_arg() {
        let mut b = CodeBlock::builder();
        b.add("this.%N()", (NameArg("getUser".to_string()),));
        let block = b.build().unwrap();
        let has_name = block
            .nodes
            .iter()
            .any(|n| matches!(n, CodeNode::NameRef(s) if s == "getUser"));
        assert!(has_name);
    }

    #[test]
    fn test_string_lit_arg() {
        let mut b = CodeBlock::builder();
        b.add("const x = %S", (StringLitArg("hello".to_string()),));
        let block = b.build().unwrap();
        let has_str_lit = block
            .nodes
            .iter()
            .any(|n| matches!(n, CodeNode::StringLit(s) if s == "hello"));
        assert!(has_str_lit);
    }

    #[test]
    fn test_invalid_format_specifier() {
        let mut b = CodeBlock::builder();
        b.add("hello %X world", ());
        let result = b.build();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid format specifier"));
        assert!(err_msg.contains("%X"));
    }

    #[test]
    fn test_parse_format_invalid_specifier_returns_error() {
        let result = parse_format("foo %Z bar");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid format specifier"));
        assert!(err_msg.contains("%Z"));
    }

    #[test]
    fn test_mismatched_arg_count_includes_specifiers_and_kinds() {
        let user = TypeName::importable("./models", "User");
        let mut b = CodeBlock::builder();
        b.add("%T %S %L", (user,));
        let result = b.build();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("expects 3 args but got 1"));
        assert!(err_msg.contains("%T"));
        assert!(err_msg.contains("%S"));
        assert!(err_msg.contains("%L"));
        assert!(err_msg.contains("TypeName"));
    }

    #[test]
    fn test_begin_control_flow_with_open_non_empty() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow_with_open("class Functor f", (), " where");
        b.add_statement("fmap :: (a -> b) -> f a -> f b", ());
        b.end_control_flow();
        let block = b.build().unwrap();
        let has_override = block
            .nodes
            .iter()
            .any(|n| matches!(n, CodeNode::BlockOpenOverride(s) if s == " where"));
        assert!(has_override, "should contain BlockOpenOverride(\" where\")");
        let has_block_open = block.nodes.iter().any(|n| matches!(n, CodeNode::BlockOpen));
        assert!(
            !has_block_open,
            "should NOT contain BlockOpen when override is used"
        );
    }

    #[test]
    fn test_begin_control_flow_with_open_empty() {
        let mut b = CodeBlock::builder();
        b.begin_control_flow_with_open("match x with", (), "");
        b.add("| Red -> red", ());
        b.add_line();
        b.end_control_flow();
        let block = b.build().unwrap();
        let has_override = block
            .nodes
            .iter()
            .any(|n| matches!(n, CodeNode::BlockOpenOverride(_)));
        assert!(
            !has_override,
            "empty custom_open should skip BlockOpenOverride"
        );
        let has_block_open = block.nodes.iter().any(|n| matches!(n, CodeNode::BlockOpen));
        assert!(!has_block_open, "should NOT contain BlockOpen either");
    }
}
