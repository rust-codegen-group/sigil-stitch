//! Enum variant specification for type-safe enum generation.

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::field_spec::FieldSpec;
use crate::spec::modifiers::DeclarationContext;
use crate::type_name::TypeName;

/// Positional context passed to [`EnumVariantSpec::emit()`].
#[derive(Debug, Clone, Copy)]
pub struct VariantContext {
    /// Whether this is the first variant in the enum.
    pub is_first: bool,
    /// Whether this is the last variant in the enum.
    pub is_last: bool,
    /// Whether the enum has members (fields, properties, methods) after the variants.
    pub has_trailing_members: bool,
}

/// A single enum variant (e.g., `Red`, `Up = 'UP'`, `case red`).
///
/// Used with [`TypeSpec`](crate::spec::type_spec::TypeSpec) via `add_variant()`.
/// The language's [`EnumAndAnnotationConfig::variant_prefix`](crate::lang::config::EnumAndAnnotationConfig::variant_prefix),
/// [`variant_separator`](crate::lang::config::EnumAndAnnotationConfig::variant_separator),
/// and [`variant_trailing_separator`](crate::lang::config::EnumAndAnnotationConfig::variant_trailing_separator)
/// control rendering.
///
/// For simple variants use [`EnumVariantSpec::new()`]; for variants with values,
/// annotations, or doc comments use [`EnumVariantSpec::builder()`].
///
/// # Variant forms
///
/// - **Simple**: `EnumVariantSpec::new("Red")` → `Red`
/// - **Valued**: `.value(CodeBlock::of("42", ())?)` → `Red = 42`
/// - **Tuple/associated**: `.associated_type(TypeName::primitive("i32"))` →
///   Rust: `Some(i32)`, Swift: `case success(Data)`
/// - **Struct**: `.add_field(FieldSpec::builder("x", TypeName::primitive("i32")).build())` →
///   Rust: `Move { x: i32, y: i32 }`
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let type_spec = TypeSpec::builder("Direction", TypeKind::Enum)
///     .add_variant(EnumVariantSpec::new("Up").unwrap())
///     .add_variant(
///         EnumVariantSpec::builder("Down")
///             .value(CodeBlock::of("'DOWN'", ()).unwrap())
///             .build().unwrap(),
///     )
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnumVariantSpec {
    pub(crate) name: String,
    pub(crate) doc: Vec<String>,
    pub(crate) value: Option<CodeBlock>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
    /// Associated types for tuple-style variants (e.g., `Some(T)`, `case .success(Data)`).
    pub(crate) associated_types: Vec<TypeName>,
    /// Named fields for struct-style variants (e.g., Rust `Move { x: i32, y: i32 }`).
    pub(crate) fields: Vec<FieldSpec>,
}

impl EnumVariantSpec {
    /// Create a simple variant with just a name.
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    pub fn new(name: &str) -> Result<Self, crate::error::SigilStitchError> {
        snafu::ensure!(
            !name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "EnumVariantSpec",
            }
        );
        Ok(Self {
            name: name.to_string(),
            doc: Vec::new(),
            value: None,
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            associated_types: Vec::new(),
            fields: Vec::new(),
        })
    }

    /// Whether this variant has an explicit value.
    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    /// Create a variant builder for more complex variants.
    pub fn builder(name: &str) -> EnumVariantSpecBuilder {
        EnumVariantSpecBuilder {
            name: name.to_string(),
            doc: Vec::new(),
            value: None,
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            associated_types: Vec::new(),
            fields: Vec::new(),
        }
    }

    /// Emit this variant as a CodeBlock fragment, including prefix, value,
    /// struct fields, separator, and terminator.
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
        ctx: VariantContext,
    ) -> Result<CodeBlock, SigilStitchError> {
        let mut cb = CodeBlock::builder();
        self.emit_into(&mut cb, lang, ctx)?;
        cb.build()
    }

    /// Emit this variant directly into an existing builder.
    pub fn emit_into(
        &self,
        cb: &mut CodeBlockBuilder,
        lang: &dyn CodeLang,
        ctx: VariantContext,
    ) -> Result<(), SigilStitchError> {
        let ea = lang.enum_and_annotation();
        let sep = ea.variant_separator;
        let trailing = ea.variant_trailing_separator;

        let emit_doc = || -> Option<String> {
            if self.doc.is_empty() || lang.doc_comment_inside_body() {
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

        let prefix = if ctx.is_first {
            ea.variant_prefix_first.unwrap_or(ea.variant_prefix)
        } else {
            ea.variant_prefix
        };
        let mut fmt = String::new();
        let mut args: Vec<Arg> = Vec::new();
        fmt.push_str(prefix);
        fmt.push_str(&self.name);

        // Tuple/associated types: Name(Type1, Type2)
        if !self.associated_types.is_empty() {
            fmt.push('(');
            for (j, ty) in self.associated_types.iter().enumerate() {
                if j > 0 {
                    fmt.push_str(", ");
                }
                fmt.push_str("%T");
                args.push(Arg::TypeName(ty.clone()));
            }
            fmt.push(')');
        }

        // Struct fields: Name { field: Type, ... }
        if !self.fields.is_empty() {
            fmt.push_str(" {");
            cb.add(&fmt, args);
            cb.add_line();
            cb.add("%>", ());
            for field in &self.fields {
                cb.add_code(field.emit(lang, DeclarationContext::Member)?);
            }
            cb.add("%<", ());
            if ctx.is_last && ctx.has_trailing_members && !ea.variant_section_terminator.is_empty()
            {
                cb.add(&format!("}}{}", ea.variant_section_terminator), ());
            } else if !sep.is_empty() && (!ctx.is_last || trailing) {
                cb.add(&format!("}}{sep}"), ());
            } else {
                cb.add("}", ());
            }
            cb.add_line();
            return Ok(());
        }

        if let Some(val) = &self.value {
            match ea.variant_value_format {
                crate::lang::config::VariantValueFormat::Assignment => {
                    fmt.push_str(" = %L");
                }
                crate::lang::config::VariantValueFormat::ConstructorArg => {
                    fmt.push_str("(%L)");
                }
            }
            args.push(Arg::Code(val.clone()));
        }

        if ctx.is_last && ctx.has_trailing_members && !ea.variant_section_terminator.is_empty() {
            fmt.push_str(ea.variant_section_terminator);
        } else if !sep.is_empty() && (!ctx.is_last || trailing) {
            fmt.push_str(sep);
        }

        cb.add(&fmt, args);
        cb.add_line();
        Ok(())
    }
}

/// Builder for [`EnumVariantSpec`].
#[derive(Debug)]
pub struct EnumVariantSpecBuilder {
    name: String,
    doc: Vec<String>,
    value: Option<CodeBlock>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    associated_types: Vec<TypeName>,
    fields: Vec<FieldSpec>,
}

impl EnumVariantSpecBuilder {
    /// Add a doc comment line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }

    /// Set the variant's value (e.g., `= 0`, `= 'UP'`, `= auto()`).
    pub fn value(mut self, val: CodeBlock) -> Self {
        self.value = Some(val);
        self
    }

    /// Add an annotation (e.g., `#[default]`, `@JsonValue`).
    pub fn annotation(mut self, ann: CodeBlock) -> Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured annotation.
    pub fn annotate(mut self, spec: AnnotationSpec) -> Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Add an associated type for tuple-style variants.
    ///
    /// Call multiple times for multi-element tuples.
    /// Example: `Some(i32)` or `case .success(Data, Int)`.
    pub fn associated_type(mut self, ty: TypeName) -> Self {
        self.associated_types.push(ty);
        self
    }

    /// Add a named field for struct-style variants.
    ///
    /// Example: Rust `Move { x: i32, y: i32 }`.
    pub fn add_field(mut self, field: FieldSpec) -> Self {
        self.fields.push(field);
        self
    }

    /// Build the variant spec.
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    pub fn build(self) -> Result<EnumVariantSpec, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "EnumVariantSpecBuilder",
            }
        );
        Ok(EnumVariantSpec {
            name: self.name,
            doc: self.doc,
            value: self.value,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            associated_types: self.associated_types,
            fields: self.fields,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::CodeLang;
    use crate::lang::rust_lang::RustLang;
    use crate::lang::swift::Swift;
    use crate::lang::typescript::TypeScript;
    use crate::spec::field_spec::FieldSpec;
    use crate::spec::modifiers::TypeKind;
    use crate::spec::type_spec::TypeSpec;
    use crate::type_name::TypeName;

    fn render_enum(ts: &TypeSpec, lang: &dyn CodeLang) -> String {
        let blocks = ts.emit(lang).unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut output = String::new();
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            let mut renderer = crate::code_renderer::CodeRenderer::new(lang, &imports, 80);
            output.push_str(&renderer.render(block).unwrap());
        }
        output
    }

    #[test]
    fn test_simple_variants() {
        let ts = TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("Red").unwrap())
            .add_variant(EnumVariantSpec::new("Green").unwrap())
            .add_variant(EnumVariantSpec::new("Blue").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &RustLang::new());
        assert!(output.contains("Red,"));
        assert!(output.contains("Green,"));
        assert!(output.contains("Blue,"));
    }

    #[test]
    fn test_variant_with_value() {
        let ts = TypeSpec::builder("Direction", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Up")
                    .value(CodeBlock::of("'UP'", ()).unwrap())
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let output = render_enum(&ts, &TypeScript::new());
        assert!(output.contains("Up = 'UP',"));
    }

    #[test]
    fn test_swift_variant_prefix() {
        let ts = TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("red").unwrap())
            .add_variant(EnumVariantSpec::new("green").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &Swift::new());
        assert!(output.contains("case red"));
        assert!(output.contains("case green"));
        // Swift has no separator.
        assert!(!output.contains("case red,"));
    }

    #[test]
    fn test_trailing_separator() {
        let ts = TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("Red").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &RustLang::new());
        // Rust has trailing comma.
        assert!(output.contains("Red,"));
    }

    #[test]
    fn test_no_trailing_separator() {
        let ts = TypeSpec::builder("Color", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("RED").unwrap())
            .add_variant(EnumVariantSpec::new("GREEN").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &crate::lang::c_lang::CLang::new());
        assert!(output.contains("RED,"));
        // Last variant has no trailing comma in C.
        assert!(output.contains("GREEN\n"));
        assert!(!output.contains("GREEN,"));
    }

    #[test]
    fn test_new_empty_name_errors() {
        let result = EnumVariantSpec::new("");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }

    #[test]
    fn test_build_empty_name_errors() {
        let result = EnumVariantSpec::builder("").build();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }

    #[test]
    fn test_tuple_variant() {
        let ts = TypeSpec::builder("Expr", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Literal")
                    .associated_type(TypeName::primitive("i64"))
                    .build()
                    .unwrap(),
            )
            .add_variant(EnumVariantSpec::new("Unit").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &RustLang::new());
        assert!(output.contains("Literal(i64),"));
        assert!(output.contains("Unit,"));
    }

    #[test]
    fn test_multi_tuple_variant() {
        let ts = TypeSpec::builder("Pair", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("Both")
                    .associated_type(TypeName::primitive("String"))
                    .associated_type(TypeName::primitive("i32"))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let output = render_enum(&ts, &RustLang::new());
        assert!(output.contains("Both(String, i32),"));
    }

    #[test]
    fn test_struct_variant() {
        let ts = TypeSpec::builder("Msg", TypeKind::Enum)
            .add_variant(EnumVariantSpec::new("Quit").unwrap())
            .add_variant(
                EnumVariantSpec::builder("Move")
                    .add_field(
                        FieldSpec::builder("x", TypeName::primitive("i32"))
                            .build()
                            .unwrap(),
                    )
                    .add_field(
                        FieldSpec::builder("y", TypeName::primitive("i32"))
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let output = render_enum(&ts, &RustLang::new());
        assert!(output.contains("Quit,"));
        assert!(output.contains("Move {"));
        assert!(output.contains("x: i32,"));
        assert!(output.contains("y: i32,"));
    }

    #[test]
    fn test_swift_associated_value() {
        let ts = TypeSpec::builder("Result", TypeKind::Enum)
            .add_variant(
                EnumVariantSpec::builder("success")
                    .associated_type(TypeName::primitive("Data"))
                    .build()
                    .unwrap(),
            )
            .add_variant(EnumVariantSpec::new("failure").unwrap())
            .build()
            .unwrap();
        let output = render_enum(&ts, &Swift::new());
        assert!(output.contains("case success(Data)"));
        assert!(output.contains("case failure"));
    }
}
