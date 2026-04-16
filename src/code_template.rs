//! Reusable parameterized code templates.
//!
//! `CodeTemplate` provides named parameters on top of `CodeBlock`'s positional
//! format strings. Templates use `#{name:K}` syntax where `K` is one of:
//!
//! | Kind | Specifier | Argument Type |
//! |------|-----------|---------------|
//! | `T`  | `%T`      | `TypeName<L>` |
//! | `N`  | `%N`      | `NameArg`     |
//! | `S`  | `%S`      | `StringLitArg`|
//! | `L`  | `%L`      | `&str`, `String`, or `CodeBlock<L>` |
//!
//! # Example
//!
//! ```rust
//! use sigil_stitch::code_template::CodeTemplate;
//! use sigil_stitch::code_block::NameArg;
//! use sigil_stitch::lang::typescript::TypeScript;
//! use sigil_stitch::type_name::TypeName;
//!
//! let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = #{init:L}").unwrap();
//! let user_type = TypeName::<TypeScript>::primitive("string");
//!
//! let block = tmpl.apply::<TypeScript>()
//!     .set("var", NameArg("user".into()))
//!     .set("type", user_type)
//!     .set("init", "null")
//!     .build()
//!     .unwrap();
//! ```

use std::collections::HashMap;
use std::marker::PhantomData;

use crate::code_block::{Arg, CodeBlock};
use crate::lang::CodeLang;

/// The kind of a template parameter, matching the format specifier system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// Type reference (`%T`). Expects `Arg::TypeName`.
    Type,
    /// Name identifier (`%N`). Expects `Arg::Name`.
    Name,
    /// String literal (`%S`). Expects `Arg::StringLit`.
    StringLit,
    /// Literal or nested code block (`%L`). Expects `Arg::Literal` or `Arg::Code`.
    Literal,
}

impl ParamKind {
    fn specifier(self) -> &'static str {
        match self {
            ParamKind::Type => "%T",
            ParamKind::Name => "%N",
            ParamKind::StringLit => "%S",
            ParamKind::Literal => "%L",
        }
    }

    fn from_char(c: u8) -> Option<Self> {
        match c {
            b'T' => Some(ParamKind::Type),
            b'N' => Some(ParamKind::Name),
            b'S' => Some(ParamKind::StringLit),
            b'L' => Some(ParamKind::Literal),
            _ => None,
        }
    }

    fn matches_arg<L: CodeLang>(&self, arg: &Arg<L>) -> bool {
        matches!(
            (self, arg),
            (ParamKind::Type, Arg::TypeName(_))
                | (ParamKind::Name, Arg::Name(_))
                | (ParamKind::StringLit, Arg::StringLit(_))
                | (ParamKind::Literal, Arg::Literal(_) | Arg::Code(_))
        )
    }

    fn label(self) -> &'static str {
        match self {
            ParamKind::Type => "Type",
            ParamKind::Name => "Name",
            ParamKind::StringLit => "StringLit",
            ParamKind::Literal => "Literal",
        }
    }
}

/// A parsed named parameter from the template format string.
#[derive(Debug, Clone)]
struct TemplateParam {
    name: String,
    kind: ParamKind,
}

/// A reusable, parameterized code template.
///
/// Templates are language-agnostic format string patterns using `#{name:K}`
/// syntax for named parameters. The language parameter `L` enters at
/// [`apply`](CodeTemplate::apply) time when concrete arguments are provided.
/// This allows the same template to be reused across different target languages.
///
/// Duplicate parameter names are allowed -- the same value is used at each
/// occurrence.
///
/// # Examples
///
/// ```ignore
/// use sigil_stitch::code_template::CodeTemplate;
/// use sigil_stitch::code_block::NameArg;
/// use sigil_stitch::lang::typescript::TypeScript;
/// use sigil_stitch::type_name::TypeName;
///
/// let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = #{init:L}").unwrap();
///
/// let block = tmpl.apply::<TypeScript>()
///     .set("var", NameArg("user".into()))
///     .set("type", TypeName::<TypeScript>::primitive("string"))
///     .set("init", "null")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct CodeTemplate {
    /// Original template format string (for error messages).
    source: String,
    /// The positional format string with `#{...}` replaced by `%T`/`%N`/`%S`/`%L`.
    positional_format: String,
    /// Ordered list of parameters as they appear in the format string.
    params: Vec<TemplateParam>,
}

impl CodeTemplate {
    /// Parse a template format string with `#{name:K}` named parameters.
    ///
    /// Returns `Err` if the syntax is invalid (unclosed `#{}`, unknown kind
    /// letter, or bare positional specifiers like `%T`).
    pub fn new(format: &str) -> Result<Self, crate::error::SigilStitchError> {
        let (positional_format, params) = parse_template(format)?;
        Ok(CodeTemplate {
            source: format.to_string(),
            positional_format,
            params,
        })
    }

    /// Begin applying this template with concrete arguments.
    pub fn apply<L: CodeLang>(&self) -> TemplateApply<'_, L> {
        TemplateApply {
            template: self,
            args: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Get the parameter names and their kinds (for introspection).
    pub fn param_names(&self) -> Vec<(&str, ParamKind)> {
        let mut seen = std::collections::HashSet::new();
        self.params
            .iter()
            .filter(|p| seen.insert(&p.name))
            .map(|p| (p.name.as_str(), p.kind))
            .collect()
    }
}

/// Builder for applying a [`CodeTemplate`] with concrete arguments.
///
/// Created by [`CodeTemplate::apply()`]. Set named parameters with `set()`,
/// then call `build()` to produce a `CodeBlock`.
pub struct TemplateApply<'t, L: CodeLang> {
    template: &'t CodeTemplate,
    args: HashMap<String, Arg<L>>,
    _phantom: PhantomData<L>,
}

impl<L: CodeLang> TemplateApply<'_, L> {
    /// Set a named parameter value.
    ///
    /// The argument kind is validated against the declared parameter kind
    /// in [`build`](TemplateApply::build).
    pub fn set(&mut self, name: &str, arg: impl Into<Arg<L>>) -> &mut Self {
        self.args.insert(name.to_string(), arg.into());
        self
    }

    /// Build the `CodeBlock`, validating all parameters are provided and
    /// kind-correct.
    pub fn build(&mut self) -> Result<CodeBlock<L>, crate::error::SigilStitchError> {
        let mut positional_args: Vec<Arg<L>> = Vec::with_capacity(self.template.params.len());

        for param in &self.template.params {
            let arg = self.args.get(&param.name).ok_or_else(|| {
                crate::error::SigilStitchError::Template {
                    message: format!(
                        "Template {:?}: missing parameter '{}'",
                        self.template.source, param.name
                    ),
                }
            })?;

            if !param.kind.matches_arg(arg) {
                return Err(crate::error::SigilStitchError::Template {
                    message: format!(
                        "Template {:?}: parameter '{}' declared as {} but received {:?}",
                        self.template.source,
                        param.name,
                        param.kind.label(),
                        arg_kind_label(arg),
                    ),
                });
            }

            positional_args.push(arg.clone());
        }

        CodeBlock::of(&self.template.positional_format, positional_args)
    }
}

fn arg_kind_label<L: CodeLang>(arg: &Arg<L>) -> &'static str {
    match arg {
        Arg::TypeName(_) => "TypeName",
        Arg::Name(_) => "Name",
        Arg::StringLit(_) => "StringLit",
        Arg::Literal(_) => "Literal",
        Arg::Code(_) => "Code",
    }
}

// ── Template parser ─────────────────────────────────────

fn template_err(message: String) -> crate::error::SigilStitchError {
    crate::error::SigilStitchError::Template { message }
}

/// Parse a template format string, replacing `#{name:K}` with positional
/// specifiers and collecting parameter declarations.
fn parse_template(
    format: &str,
) -> Result<(String, Vec<TemplateParam>), crate::error::SigilStitchError> {
    let mut output = String::with_capacity(format.len());
    let mut params = Vec::new();
    let bytes = format.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Check for bare positional arg-consuming specifiers.
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            let spec = bytes[i + 1];
            match spec {
                // Arg-consuming specifiers are forbidden in templates.
                b'T' | b'N' | b'S' | b'L' => {
                    return Err(template_err(format!(
                        "Template format string contains bare positional specifier '%{}' \
                         at byte {}; use #{{name:{}}} syntax instead",
                        spec as char, i, spec as char,
                    )));
                }
                // Non-arg specifiers pass through.
                b'W' | b'>' | b'<' | b'[' | b']' | b'%' => {
                    output.push('%');
                    output.push(spec as char);
                    i += 2;
                }
                _ => {
                    // Unknown %-specifier, pass through as literal.
                    output.push('%');
                    output.push(spec as char);
                    i += 2;
                }
            }
            continue;
        }

        // Check for `#` sequences.
        if bytes[i] == b'#' {
            // `##` → literal `#`.
            if i + 1 < bytes.len() && bytes[i + 1] == b'#' {
                output.push('#');
                i += 2;
                continue;
            }

            // `#{...}` → named parameter.
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                let start = i;
                let close = format[i + 2..]
                    .find('}')
                    .ok_or_else(|| template_err(format!("Unclosed '#{{' at byte {start}")))?;
                let inner = &format[i + 2..i + 2 + close];

                // Parse "name:K".
                let colon = inner.find(':').ok_or_else(|| {
                    template_err(format!(
                        "Expected '#{{name:K}}' but found '#{{{}}}' at byte {} \
                         (missing ':' separator)",
                        inner, start
                    ))
                })?;
                let name = &inner[..colon];
                let kind_str = &inner[colon + 1..];

                if name.is_empty() {
                    return Err(template_err(format!(
                        "Empty parameter name in '#{{{}}}' at byte {}",
                        inner, start
                    )));
                }
                if kind_str.len() != 1 {
                    return Err(template_err(format!(
                        "Expected single kind letter (T/N/S/L) but found '{}' in '#{{{}}}' at byte {}",
                        kind_str, inner, start
                    )));
                }

                let kind = ParamKind::from_char(kind_str.as_bytes()[0]).ok_or_else(|| {
                    template_err(format!(
                        "Unknown parameter kind '{}' in '#{{{}}}' at byte {} \
                         (expected T, N, S, or L)",
                        kind_str, inner, start
                    ))
                })?;

                // Check duplicate names have consistent kinds.
                if let Some(existing) = params.iter().find(|p: &&TemplateParam| p.name == name)
                    && existing.kind != kind
                {
                    return Err(template_err(format!(
                        "Parameter '{}' declared as {} at byte {} but previously declared as {}",
                        name,
                        kind.label(),
                        start,
                        existing.kind.label(),
                    )));
                }

                output.push_str(kind.specifier());
                params.push(TemplateParam {
                    name: name.to_string(),
                    kind,
                });

                i = i + 2 + close + 1; // skip past '}'
                continue;
            }

            // Bare `#` followed by anything else → literal.
            output.push('#');
            i += 1;
            continue;
        }

        // Regular character.
        output.push(bytes[i] as char);
        i += 1;
    }

    Ok((output, params))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_block::NameArg;
    use crate::code_block::StringLitArg;
    use crate::import_collector;
    use crate::lang::rust_lang::RustLang;
    use crate::lang::typescript::TypeScript;
    use crate::type_name::TypeName;

    #[test]
    fn test_parse_simple_template() {
        let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = #{init:L}").unwrap();
        assert_eq!(tmpl.positional_format, "const %N: %T = %L");
        let names = tmpl.param_names();
        assert_eq!(names.len(), 3);
        assert_eq!(names[0], ("var", ParamKind::Name));
        assert_eq!(names[1], ("type", ParamKind::Type));
        assert_eq!(names[2], ("init", ParamKind::Literal));
    }

    #[test]
    fn test_apply_simple() {
        let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = #{init:L}").unwrap();
        let ty = TypeName::<TypeScript>::primitive("string");
        let block = tmpl
            .apply::<TypeScript>()
            .set("var", NameArg("user".into()))
            .set("type", ty)
            .set("init", "null")
            .build()
            .unwrap();
        assert!(!block.is_empty());
    }

    #[test]
    fn test_missing_param_error() {
        let tmpl = CodeTemplate::new("#{a:N} #{b:T}").unwrap();
        let result = tmpl
            .apply::<TypeScript>()
            .set("a", NameArg("x".into()))
            .build();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("missing parameter 'b'"),
            "got: {err}"
        );
    }

    #[test]
    fn test_kind_mismatch_error() {
        let tmpl = CodeTemplate::new("#{name:N}").unwrap();
        // Pass a TypeName where Name is expected.
        let result = tmpl
            .apply::<TypeScript>()
            .set("name", TypeName::<TypeScript>::primitive("string"))
            .build();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("declared as Name"), "got: {err}");
        assert!(err.to_string().contains("TypeName"), "got: {err}");
    }

    #[test]
    fn test_duplicate_param_name() {
        let tmpl = CodeTemplate::new("#{t:T} and #{t:T}").unwrap();
        let ty = TypeName::<TypeScript>::primitive("number");
        let block = tmpl.apply::<TypeScript>().set("t", ty).build().unwrap();
        assert!(!block.is_empty());
        // Should have 2 args (one for each occurrence).
        assert_eq!(block.args.len(), 2);
    }

    #[test]
    fn test_escaped_hash() {
        let tmpl = CodeTemplate::new("##[derive(Debug)]").unwrap();
        assert_eq!(tmpl.positional_format, "#[derive(Debug)]");
        assert!(tmpl.params.is_empty());
    }

    #[test]
    fn test_bare_hash_passthrough() {
        let tmpl = CodeTemplate::new("# comment").unwrap();
        assert_eq!(tmpl.positional_format, "# comment");
        assert!(tmpl.params.is_empty());
    }

    #[test]
    fn test_reject_bare_positional_specifiers() {
        let result = CodeTemplate::new("#{name:N} %T");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("bare positional specifier '%T'"),
            "got: {err}"
        );
    }

    #[test]
    fn test_allow_non_arg_specifiers() {
        let tmpl = CodeTemplate::new("#{a:N}%W#{b:L}").unwrap();
        assert_eq!(tmpl.positional_format, "%N%W%L");
        assert_eq!(tmpl.params.len(), 2);
    }

    #[test]
    fn test_allow_percent_escape() {
        let tmpl = CodeTemplate::new("100%%").unwrap();
        assert_eq!(tmpl.positional_format, "100%%");
    }

    #[test]
    fn test_allow_indent_dedent_specifiers() {
        let tmpl = CodeTemplate::new("%>#{body:L}%<").unwrap();
        assert_eq!(tmpl.positional_format, "%>%L%<");
    }

    #[test]
    fn test_template_with_imports() {
        let tmpl = CodeTemplate::new("const x: #{type:T} = init()").unwrap();
        let ty = TypeName::<TypeScript>::importable_type("./models", "User");
        let block = tmpl.apply::<TypeScript>().set("type", ty).build().unwrap();
        let refs = import_collector::collect_imports(&block);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].name, "User");
    }

    #[test]
    fn test_language_agnostic() {
        let tmpl = CodeTemplate::new("#{name:N}: #{type:T}").unwrap();

        // Apply to TypeScript.
        let ts_block = tmpl
            .apply::<TypeScript>()
            .set("name", NameArg("x".into()))
            .set("type", TypeName::<TypeScript>::primitive("string"))
            .build()
            .unwrap();
        assert!(!ts_block.is_empty());

        // Same template applied to Rust.
        let rs_block = tmpl
            .apply::<RustLang>()
            .set("name", NameArg("x".into()))
            .set("type", TypeName::<RustLang>::primitive("String"))
            .build()
            .unwrap();
        assert!(!rs_block.is_empty());
    }

    #[test]
    fn test_unclosed_brace_error() {
        let result = CodeTemplate::new("#{name:T");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unclosed"));
    }

    #[test]
    fn test_missing_colon_error() {
        let result = CodeTemplate::new("#{name}");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing ':'"));
    }

    #[test]
    fn test_unknown_kind_error() {
        let result = CodeTemplate::new("#{name:X}");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown parameter kind")
        );
    }

    #[test]
    fn test_empty_name_error() {
        let result = CodeTemplate::new("#{:T}");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Empty parameter name")
        );
    }

    #[test]
    fn test_inconsistent_duplicate_kind_error() {
        let result = CodeTemplate::new("#{x:T} #{x:N}");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("previously declared"),
            "got: {err}"
        );
    }

    #[test]
    fn test_string_lit_param() {
        let tmpl = CodeTemplate::new("console.log(#{msg:S})").unwrap();
        let block = tmpl
            .apply::<TypeScript>()
            .set("msg", StringLitArg("hello".into()))
            .build()
            .unwrap();
        assert!(!block.is_empty());
    }

    #[test]
    fn test_code_block_param() {
        let tmpl = CodeTemplate::new("fn main() { #{body:L} }").unwrap();
        let body = CodeBlock::<RustLang>::of("println!(\"hi\")", ()).unwrap();
        let block = tmpl.apply::<RustLang>().set("body", body).build().unwrap();
        assert!(!block.is_empty());
    }
}
