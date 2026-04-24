//! Field specification for struct fields / class properties.

use crate::code_block::{Arg, CodeBlock};
use crate::lang::CodeLang;
use crate::lang::config::OptionalFieldStyle;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::modifiers::{DeclarationContext, Modifiers, Visibility};
use crate::type_name::TypeName;

/// A single field/property in a struct or class.
///
/// `FieldSpec` represents a named, typed member of a type declaration. It supports
/// visibility modifiers, static/readonly flags, initializers, doc comments,
/// annotations, and struct tags (for Go). The emitted format adapts to the target
/// language (e.g., `name: string;` in TypeScript vs `pub name: String,` in Rust).
///
/// Use [`FieldSpec::builder()`] to construct, then add to a
/// [`TypeSpec`](crate::spec::type_spec::TypeSpec) with `add_field()`.
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let field = FieldSpec::builder("name", TypeName::primitive("string"))
///     .visibility(Visibility::Private)
///     .is_readonly()
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldSpec {
    pub(crate) name: String,
    pub(crate) field_type: TypeName,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) initializer: Option<CodeBlock>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
    /// Struct tag (e.g., Go: `` `json:"name"` ``). Emitted inline after the type.
    pub(crate) tag: Option<String>,
    /// Whether this field is optional (key may be absent from the containing value).
    ///
    /// Distinct from nullability (value may be `null`), which is expressed via
    /// [`TypeName::Optional`]. Rendering is delegated to
    /// [`CodeLang::optional_field_style`].
    pub(crate) is_optional: bool,
}

impl FieldSpec {
    /// Create a new [`FieldSpecBuilder`] with the given name and type.
    pub fn builder(name: &str, field_type: TypeName) -> FieldSpecBuilder {
        FieldSpecBuilder {
            name: name.to_string(),
            field_type,
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            initializer: None,
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            tag: None,
            is_optional: false,
        }
    }

    /// Convenience constructor for a simple field (name + type, no modifiers).
    pub fn new(name: &str, field_type: TypeName) -> Result<Self, crate::error::SigilStitchError> {
        Self::builder(name, field_type).build()
    }

    /// Infallible convenience constructor for a simple field.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty.
    pub fn of(name: &str, field_type: TypeName) -> Self {
        Self::new(name, field_type).expect("FieldSpec name must not be empty")
    }

    /// Returns the field name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the field type.
    pub fn field_type(&self) -> &TypeName {
        &self.field_type
    }

    /// Emit this field as a CodeBlock.
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let mut cb = CodeBlock::builder();

        let emit_doc = || -> Option<String> {
            if self.doc.is_empty() {
                return None;
            }
            let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
            Some(lang.render_doc_comment(&doc_lines))
        };

        if lang.doc_before_annotations()
            && let Some(doc_str) = emit_doc()
        {
            cb.add("%L", doc_str);
            cb.add_line();
        }

        for spec in &self.annotation_specs {
            cb.add_code(spec.emit(lang)?);
            cb.add_line();
        }
        for ann in &self.annotations {
            cb.add_code(ann.clone());
            cb.add_line();
        }

        if !lang.doc_before_annotations()
            && let Some(doc_str) = emit_doc()
        {
            cb.add("%L", doc_str);
            cb.add_line();
        }

        // Build the field line.
        let vis = lang.render_visibility(self.modifiers.visibility, ctx);
        let term = lang.block_syntax().field_terminator;

        let mut fmt = String::new();
        let mut args: Vec<Arg> = Vec::new();

        fmt.push_str(vis);
        if self.modifiers.is_static {
            fmt.push_str("static ");
        }

        // Resolve the optional-field style (only applied when `is_optional` is set).
        let opt_style = if self.is_optional {
            lang.optional_field_style()
        } else {
            OptionalFieldStyle::Ignored
        };
        let type_before = lang.type_decl_syntax().type_before_name;

        let name_suffix: &str = match opt_style {
            OptionalFieldStyle::NameSuffix(s) => s,
            _ => "",
        };
        let name_prefix: &str = match opt_style {
            // In type-before-name languages the prefix attaches to the name
            // position (C: `T *name`).
            OptionalFieldStyle::TypePrefix(s) if type_before => s,
            _ => "",
        };
        let (type_pre, type_post): (String, String) = match opt_style {
            OptionalFieldStyle::TypeSuffix(s) => (String::new(), s.to_string()),
            OptionalFieldStyle::TypeWrap { open, close } => (open.to_string(), close.to_string()),
            // In name-before-type languages the prefix attaches to the type
            // position (Go: `name *T`).
            OptionalFieldStyle::TypePrefix(s) if !type_before => (s.to_string(), String::new()),
            OptionalFieldStyle::UnionWithNone(sep) => (String::new(), format!("{sep}None")),
            _ => (String::new(), String::new()),
        };

        if type_before {
            // C-style: type name
            if self.modifiers.is_readonly {
                fmt.push_str(lang.enum_and_annotation().readonly_keyword);
            }
            if !self.field_type.is_empty() {
                fmt.push_str(&type_pre);
                fmt.push_str("%T");
                fmt.push_str(&type_post);
                args.push(Arg::TypeName(self.field_type.clone()));
                fmt.push(' ');
            }
            fmt.push_str(name_prefix);
            fmt.push_str(&lang.escape_reserved(&self.name));
            fmt.push_str(name_suffix);
        } else {
            // TS/Rust/Go/Python-style: name sep type
            if self.modifiers.is_readonly {
                fmt.push_str(lang.enum_and_annotation().readonly_keyword);
            } else {
                let mk = lang.enum_and_annotation().mutable_field_keyword;
                if !mk.is_empty() {
                    fmt.push_str(mk);
                }
            }
            fmt.push_str(&lang.escape_reserved(&self.name));
            fmt.push_str(name_suffix);

            // Skip type annotation when the type is empty (e.g., Python enum members).
            if !self.field_type.is_empty() {
                let sep = lang.type_decl_syntax().type_annotation_separator;
                fmt.push_str(sep);
                fmt.push_str(&type_pre);
                fmt.push_str("%T");
                fmt.push_str(&type_post);
                args.push(Arg::TypeName(self.field_type.clone()));
            }
        }

        if let Some(init) = &self.initializer {
            fmt.push_str(" = %L");
            args.push(Arg::Code(init.clone()));
        }

        if let Some(tag) = &self.tag {
            fmt.push_str(" `");
            fmt.push_str(tag);
            fmt.push('`');
        }

        fmt.push_str(term);
        cb.add(&fmt, args);
        cb.add_line();

        cb.build()
    }
}

/// Builder for [`FieldSpec`].
#[derive(Debug)]
pub struct FieldSpecBuilder {
    name: String,
    field_type: TypeName,
    modifiers: Modifiers,
    doc: Vec<String>,
    initializer: Option<CodeBlock>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    tag: Option<String>,
    is_optional: bool,
}

impl FieldSpecBuilder {
    /// Set the visibility modifier.
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this field as static.
    pub fn is_static(mut self) -> Self {
        self.modifiers.is_static = true;
        self
    }

    /// Mark this field as readonly.
    pub fn is_readonly(mut self) -> Self {
        self.modifiers.is_readonly = true;
        self
    }

    /// Mark this field as optional (the key may be absent from the containing value).
    ///
    /// Rendering is language-specific and delegates to
    /// [`CodeLang::optional_field_style`]: TypeScript emits `name?: T`, Rust
    /// emits `Option<T>`, Go emits `*T`, etc. Languages that cannot express
    /// optionality (JavaScript, Bash, Zsh) render the field as if it were
    /// required.
    pub fn is_optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    /// Add a doc comment line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }

    /// Set the field initializer expression.
    pub fn initializer(mut self, init: CodeBlock) -> Self {
        self.initializer = Some(init);
        self
    }

    /// Add a raw annotation [`CodeBlock`].
    pub fn annotation(mut self, ann: CodeBlock) -> Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured [`AnnotationSpec`].
    pub fn annotate(mut self, spec: AnnotationSpec) -> Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Set the struct tag (e.g., Go's `` `json:"name"` ``).
    pub fn tag(mut self, t: &str) -> Self {
        self.tag = Some(t.to_string());
        self
    }

    /// Build the [`FieldSpec`] from this builder.
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`](crate::error::SigilStitchError::EmptyName) if `name` is empty.
    pub fn build(self) -> Result<FieldSpec, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "FieldSpecBuilder",
            }
        );
        Ok(FieldSpec {
            name: self.name,
            field_type: self.field_type,
            modifiers: self.modifiers,
            doc: self.doc,
            initializer: self.initializer,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            tag: self.tag,
            is_optional: self.is_optional,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::rust_lang::RustLang;
    use crate::lang::typescript::TypeScript;

    fn emit_field_ts(spec: &FieldSpec, ctx: DeclarationContext) -> String {
        let lang = TypeScript::new();
        let block = spec.emit(&lang, ctx).unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
        renderer.render(&block).unwrap()
    }

    fn emit_field_rs(spec: &FieldSpec, ctx: DeclarationContext) -> String {
        let lang = RustLang::new();
        let block = spec.emit(&lang, ctx).unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
        renderer.render(&block).unwrap()
    }

    #[test]
    fn test_ts_field_basic() {
        let field = FieldSpec::builder("name", TypeName::primitive("string"))
            .build()
            .unwrap();
        let output = emit_field_ts(&field, DeclarationContext::Member);
        assert_eq!(output.trim(), "name: string;");
    }

    #[test]
    fn test_ts_field_with_visibility() {
        let field = FieldSpec::builder("name", TypeName::primitive("string"))
            .visibility(Visibility::Private)
            .build()
            .unwrap();
        let output = emit_field_ts(&field, DeclarationContext::Member);
        assert_eq!(output.trim(), "private name: string;");
    }

    #[test]
    fn test_rust_field_basic() {
        let field = FieldSpec::builder("name", TypeName::primitive("String"))
            .visibility(Visibility::Public)
            .build()
            .unwrap();
        let output = emit_field_rs(&field, DeclarationContext::Member);
        assert_eq!(output.trim(), "pub name: String,");
    }

    #[test]
    fn test_ts_field_readonly_static() {
        let field = FieldSpec::builder("MAX", TypeName::primitive("number"))
            .is_static()
            .is_readonly()
            .build()
            .unwrap();
        let output = emit_field_ts(&field, DeclarationContext::Member);
        assert_eq!(output.trim(), "static readonly MAX: number;");
    }

    #[test]
    fn test_build_empty_name_errors() {
        let result = FieldSpec::builder("", TypeName::primitive("string")).build();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }

    // --- Optional field rendering across languages ---

    fn emit_for(lang: &dyn CodeLang, spec: &FieldSpec, ctx: DeclarationContext) -> String {
        let block = spec.emit(lang, ctx).unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(lang, &imports, 80);
        renderer.render(&block).unwrap()
    }

    fn optional_field(type_name: TypeName) -> FieldSpec {
        FieldSpec::builder("name", type_name)
            .is_optional()
            .build()
            .unwrap()
    }

    #[test]
    fn test_ts_optional_field_uses_name_suffix() {
        let field = optional_field(TypeName::primitive("string"));
        let out = emit_for(&TypeScript::new(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), "name?: string;");
    }

    #[test]
    fn test_rust_optional_field_wraps_with_option() {
        let field = optional_field(TypeName::primitive("String"));
        let out = emit_for(&RustLang::new(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), "name: Option<String>,");
    }

    #[test]
    fn test_go_optional_field_prefixes_type_with_pointer() {
        use crate::lang::go_lang::GoLang;
        let field = optional_field(TypeName::primitive("string"));
        let out = emit_for(&GoLang::new(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), "name *string");
    }

    #[test]
    fn test_python_optional_field_unions_with_none() {
        use crate::lang::python::Python;
        let field = optional_field(TypeName::primitive("str"));
        let out = emit_for(&Python::new(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), "name: str | None");
    }

    #[test]
    fn test_java_optional_field_wraps_with_optional() {
        use crate::lang::java_lang::JavaLang;
        let field = optional_field(TypeName::primitive("String"));
        let out = emit_for(&JavaLang::new(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), "Optional<String> name;");
    }

    #[test]
    fn test_kotlin_optional_field_suffixes_type() {
        use crate::lang::kotlin::Kotlin;
        let field = optional_field(TypeName::primitive("String"));
        let out = emit_for(&Kotlin::new(), &field, DeclarationContext::Member);
        // Kotlin renders a mutable `var` and its default visibility keyword.
        assert!(
            out.contains("name: String?"),
            "expected type suffix '?', got {out:?}"
        );
    }

    #[test]
    fn test_swift_optional_field_suffixes_type() {
        use crate::lang::swift::Swift;
        let field = optional_field(TypeName::primitive("String"));
        let out = emit_for(&Swift::new(), &field, DeclarationContext::Member);
        // Swift prepends `var` for mutable stored properties.
        assert!(
            out.contains("name: String?"),
            "expected type suffix '?', got {out:?}"
        );
    }

    #[test]
    fn test_dart_optional_field_suffixes_type() {
        use crate::lang::dart::DartLang;
        let field = optional_field(TypeName::primitive("String"));
        let out = emit_for(&DartLang::new(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), "String? name;");
    }

    #[test]
    fn test_c_optional_field_prefixes_name_with_pointer() {
        use crate::lang::c_lang::CLang;
        let field = optional_field(TypeName::primitive("int"));
        let out = emit_for(&CLang::new(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), "int *name;");
    }

    #[test]
    fn test_cpp_optional_field_wraps_with_std_optional() {
        use crate::lang::cpp_lang::CppLang;
        let field = optional_field(TypeName::primitive("int"));
        let out = emit_for(&CppLang::new(), &field, DeclarationContext::Member);
        assert_eq!(out.trim(), "std::optional<int> name;");
    }

    #[test]
    fn test_javascript_optional_field_is_ignored() {
        use crate::lang::javascript::JavaScript;
        let field = optional_field(TypeName::primitive("any"));
        let out = emit_for(&JavaScript::new(), &field, DeclarationContext::Member);
        // JavaScript cannot express optional fields, so renders as required.
        assert!(out.contains("name"), "expected name in output, got {out:?}");
        assert!(
            !out.contains("?"),
            "JS output must not contain '?': {out:?}"
        );
    }
}
