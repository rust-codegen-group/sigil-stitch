//! Enum variant specification for type-safe enum generation.

use crate::code_block::CodeBlock;
use crate::lang::CodeLang;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::field_spec::FieldSpec;
use crate::type_name::TypeName;

/// A single enum variant (e.g., `Red`, `Up = 'UP'`, `case red`).
///
/// Used with [`crate::spec::type_spec::TypeSpec`] via `add_variant()`.
/// The language's [`CodeLang::enum_variant_prefix`], [`CodeLang::enum_variant_separator`],
/// and [`CodeLang::enum_variant_trailing_separator`] control rendering.
///
/// # Variant forms
///
/// - **Simple**: `EnumVariantSpec::new("Red")` → `Red`
/// - **Valued**: `.value(CodeBlock::of("42", ())?)` → `Red = 42`
/// - **Tuple/associated**: `.associated_type(TypeName::primitive("i32"))` →
///   Rust: `Some(i32)`, Swift: `case success(Data)`
/// - **Struct**: `.add_field(FieldSpec::builder("x", TypeName::primitive("i32")).build())` →
///   Rust: `Move { x: i32, y: i32 }`
#[derive(Debug, Clone)]
pub struct EnumVariantSpec<L: CodeLang> {
    pub(crate) name: String,
    pub(crate) doc: Vec<String>,
    pub(crate) value: Option<CodeBlock<L>>,
    pub(crate) annotations: Vec<CodeBlock<L>>,
    pub(crate) annotation_specs: Vec<AnnotationSpec<L>>,
    /// Associated types for tuple-style variants (e.g., `Some(T)`, `case .success(Data)`).
    pub(crate) associated_types: Vec<TypeName<L>>,
    /// Named fields for struct-style variants (e.g., Rust `Move { x: i32, y: i32 }`).
    pub(crate) fields: Vec<FieldSpec<L>>,
}

impl<L: CodeLang> EnumVariantSpec<L> {
    /// Create a simple variant with just a name.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty.
    pub fn new(name: &str) -> Self {
        assert!(
            !name.is_empty(),
            "EnumVariantSpec::new() failed: 'name' must not be empty",
        );
        Self {
            name: name.to_string(),
            doc: Vec::new(),
            value: None,
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            associated_types: Vec::new(),
            fields: Vec::new(),
        }
    }

    /// Create a variant builder for more complex variants.
    pub fn builder(name: &str) -> EnumVariantSpecBuilder<L> {
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
}

/// Builder for [`EnumVariantSpec`].
#[derive(Debug)]
pub struct EnumVariantSpecBuilder<L: CodeLang> {
    name: String,
    doc: Vec<String>,
    value: Option<CodeBlock<L>>,
    annotations: Vec<CodeBlock<L>>,
    annotation_specs: Vec<AnnotationSpec<L>>,
    associated_types: Vec<TypeName<L>>,
    fields: Vec<FieldSpec<L>>,
}

impl<L: CodeLang> EnumVariantSpecBuilder<L> {
    /// Add a doc comment line.
    pub fn doc(&mut self, line: &str) -> &mut Self {
        self.doc.push(line.to_string());
        self
    }

    /// Set the variant's value (e.g., `= 0`, `= 'UP'`, `= auto()`).
    pub fn value(&mut self, val: CodeBlock<L>) -> &mut Self {
        self.value = Some(val);
        self
    }

    /// Add an annotation (e.g., `#[default]`, `@JsonValue`).
    pub fn annotation(&mut self, ann: CodeBlock<L>) -> &mut Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured annotation.
    pub fn annotate(&mut self, spec: AnnotationSpec<L>) -> &mut Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Add an associated type for tuple-style variants.
    ///
    /// Call multiple times for multi-element tuples.
    /// Example: `Some(i32)` or `case .success(Data, Int)`.
    pub fn associated_type(&mut self, ty: TypeName<L>) -> &mut Self {
        self.associated_types.push(ty);
        self
    }

    /// Add a named field for struct-style variants.
    ///
    /// Example: Rust `Move { x: i32, y: i32 }`.
    pub fn add_field(&mut self, field: FieldSpec<L>) -> &mut Self {
        self.fields.push(field);
        self
    }

    /// Build the variant spec.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty.
    pub fn build(self) -> EnumVariantSpec<L> {
        assert!(
            !self.name.is_empty(),
            "EnumVariantSpecBuilder::build() failed: 'name' must not be empty",
        );
        EnumVariantSpec {
            name: self.name,
            doc: self.doc,
            value: self.value,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            associated_types: self.associated_types,
            fields: self.fields,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::rust_lang::RustLang;
    use crate::lang::swift::Swift;
    use crate::lang::typescript::TypeScript;
    use crate::spec::field_spec::FieldSpec;
    use crate::spec::modifiers::TypeKind;
    use crate::spec::type_spec::TypeSpec;
    use crate::type_name::TypeName;

    fn render_enum<L: CodeLang>(ts: &TypeSpec<L>, lang: &L) -> String {
        let blocks = ts.emit(lang);
        let imports = crate::import::ImportGroup::new();
        let mut output = String::new();
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            let mut renderer = crate::code_renderer::CodeRenderer::new(lang, &imports, 80);
            output.push_str(&renderer.render(block));
        }
        output
    }

    #[test]
    fn test_simple_variants() {
        let mut tb = TypeSpec::<RustLang>::builder("Color", TypeKind::Enum);
        tb.add_variant(EnumVariantSpec::new("Red"));
        tb.add_variant(EnumVariantSpec::new("Green"));
        tb.add_variant(EnumVariantSpec::new("Blue"));
        let ts = tb.build();
        let output = render_enum(&ts, &RustLang::new());
        assert!(output.contains("Red,"));
        assert!(output.contains("Green,"));
        assert!(output.contains("Blue,"));
    }

    #[test]
    fn test_variant_with_value() {
        let mut tb = TypeSpec::<TypeScript>::builder("Direction", TypeKind::Enum);
        let mut v = EnumVariantSpec::builder("Up");
        v.value(CodeBlock::<TypeScript>::of("'UP'", ()).unwrap());
        tb.add_variant(v.build());
        let ts = tb.build();
        let output = render_enum(&ts, &TypeScript::new());
        assert!(output.contains("Up = 'UP',"));
    }

    #[test]
    fn test_swift_variant_prefix() {
        let mut tb = TypeSpec::<Swift>::builder("Color", TypeKind::Enum);
        tb.add_variant(EnumVariantSpec::new("red"));
        tb.add_variant(EnumVariantSpec::new("green"));
        let ts = tb.build();
        let output = render_enum(&ts, &Swift::new());
        assert!(output.contains("case red"));
        assert!(output.contains("case green"));
        // Swift has no separator.
        assert!(!output.contains("case red,"));
    }

    #[test]
    fn test_trailing_separator() {
        let mut tb = TypeSpec::<RustLang>::builder("Color", TypeKind::Enum);
        tb.add_variant(EnumVariantSpec::new("Red"));
        let ts = tb.build();
        let output = render_enum(&ts, &RustLang::new());
        // Rust has trailing comma.
        assert!(output.contains("Red,"));
    }

    #[test]
    fn test_no_trailing_separator() {
        let mut tb = TypeSpec::<crate::lang::c_lang::CLang>::builder("Color", TypeKind::Enum);
        tb.add_variant(EnumVariantSpec::new("RED"));
        tb.add_variant(EnumVariantSpec::new("GREEN"));
        let ts = tb.build();
        let output = render_enum(&ts, &crate::lang::c_lang::CLang::new());
        assert!(output.contains("RED,"));
        // Last variant has no trailing comma in C.
        assert!(output.contains("GREEN\n"));
        assert!(!output.contains("GREEN,"));
    }

    #[test]
    #[should_panic(expected = "EnumVariantSpec::new() failed: 'name' must not be empty")]
    fn test_new_empty_name_panics() {
        EnumVariantSpec::<TypeScript>::new("");
    }

    #[test]
    #[should_panic(expected = "EnumVariantSpecBuilder::build() failed: 'name' must not be empty")]
    fn test_build_empty_name_panics() {
        EnumVariantSpec::<TypeScript>::builder("").build();
    }

    #[test]
    fn test_tuple_variant() {
        let mut tb = TypeSpec::<RustLang>::builder("Expr", TypeKind::Enum);
        let mut v = EnumVariantSpec::<RustLang>::builder("Literal");
        v.associated_type(TypeName::primitive("i64"));
        tb.add_variant(v.build());
        tb.add_variant(EnumVariantSpec::new("Unit"));
        let ts = tb.build();
        let output = render_enum(&ts, &RustLang::new());
        assert!(output.contains("Literal(i64),"));
        assert!(output.contains("Unit,"));
    }

    #[test]
    fn test_multi_tuple_variant() {
        let mut tb = TypeSpec::<RustLang>::builder("Pair", TypeKind::Enum);
        let mut v = EnumVariantSpec::<RustLang>::builder("Both");
        v.associated_type(TypeName::primitive("String"));
        v.associated_type(TypeName::primitive("i32"));
        tb.add_variant(v.build());
        let ts = tb.build();
        let output = render_enum(&ts, &RustLang::new());
        assert!(output.contains("Both(String, i32),"));
    }

    #[test]
    fn test_struct_variant() {
        let mut tb = TypeSpec::<RustLang>::builder("Msg", TypeKind::Enum);
        tb.add_variant(EnumVariantSpec::new("Quit"));
        let mut v = EnumVariantSpec::<RustLang>::builder("Move");
        v.add_field(FieldSpec::builder("x", TypeName::primitive("i32")).build());
        v.add_field(FieldSpec::builder("y", TypeName::primitive("i32")).build());
        tb.add_variant(v.build());
        let ts = tb.build();
        let output = render_enum(&ts, &RustLang::new());
        assert!(output.contains("Quit,"));
        assert!(output.contains("Move {"));
        assert!(output.contains("x: i32,"));
        assert!(output.contains("y: i32,"));
    }

    #[test]
    fn test_swift_associated_value() {
        let mut tb = TypeSpec::<Swift>::builder("Result", TypeKind::Enum);
        let mut v_success = EnumVariantSpec::<Swift>::builder("success");
        v_success.associated_type(TypeName::primitive("Data"));
        tb.add_variant(v_success.build());
        tb.add_variant(EnumVariantSpec::new("failure"));
        let ts = tb.build();
        let output = render_enum(&ts, &Swift::new());
        assert!(output.contains("case success(Data)"));
        assert!(output.contains("case failure"));
    }
}
