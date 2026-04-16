//! Field specification for struct fields / class properties.

use crate::code_block::{Arg, CodeBlock};
use crate::lang::CodeLang;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::modifiers::{DeclarationContext, Modifiers, Visibility};
use crate::type_name::TypeName;

/// A single field/property in a struct or class.
#[derive(Debug, Clone)]
pub struct FieldSpec<L: CodeLang> {
    pub(crate) name: String,
    pub(crate) field_type: TypeName<L>,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) initializer: Option<CodeBlock<L>>,
    pub(crate) annotations: Vec<CodeBlock<L>>,
    pub(crate) annotation_specs: Vec<AnnotationSpec<L>>,
    /// Struct tag (e.g., Go: `` `json:"name"` ``). Emitted inline after the type.
    pub(crate) tag: Option<String>,
}

impl<L: CodeLang> FieldSpec<L> {
    /// Create a new [`FieldSpecBuilder`] with the given name and type.
    pub fn builder(name: &str, field_type: TypeName<L>) -> FieldSpecBuilder<L> {
        FieldSpecBuilder {
            name: name.to_string(),
            field_type,
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            initializer: None,
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            tag: None,
        }
    }

    /// Returns the field name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the field type.
    pub fn field_type(&self) -> &TypeName<L> {
        &self.field_type
    }

    /// Emit this field as a CodeBlock.
    pub fn emit(&self, lang: &L, ctx: DeclarationContext) -> CodeBlock<L> {
        let mut cb = CodeBlock::<L>::builder();

        // Annotations (structured specs first, then raw CodeBlocks).
        for spec in &self.annotation_specs {
            cb.add_code(spec.emit(lang));
            cb.add_line();
        }
        for ann in &self.annotations {
            cb.add_code(ann.clone());
            cb.add_line();
        }

        // Doc comment.
        if !self.doc.is_empty() {
            let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
            let doc_str = lang.render_doc_comment(&doc_lines);
            cb.add("%L", doc_str);
            cb.add_line();
        }

        // Build the field line.
        let vis = lang.render_visibility(self.modifiers.visibility, ctx);
        let term = lang.field_terminator();

        let mut fmt = String::new();
        let mut args: Vec<Arg<L>> = Vec::new();

        fmt.push_str(vis);
        if self.modifiers.is_static {
            fmt.push_str("static ");
        }

        if lang.type_before_name() {
            // C-style: type name
            if self.modifiers.is_readonly {
                fmt.push_str(lang.readonly_keyword());
            }
            if !self.field_type.is_empty() {
                fmt.push_str("%T");
                args.push(Arg::TypeName(self.field_type.clone()));
                fmt.push(' ');
            }
            fmt.push_str(&lang.escape_reserved(&self.name));
        } else {
            // TS/Rust/Go/Python-style: name sep type
            if self.modifiers.is_readonly {
                fmt.push_str(lang.readonly_keyword());
            } else {
                let mk = lang.mutable_field_keyword();
                if !mk.is_empty() {
                    fmt.push_str(mk);
                }
            }
            fmt.push_str(&lang.escape_reserved(&self.name));

            // Skip type annotation when the type is empty (e.g., Python enum members).
            if !self.field_type.is_empty() {
                let sep = lang.type_annotation_separator();
                fmt.push_str(sep);
                fmt.push_str("%T");
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

        cb.build().unwrap()
    }
}

/// Builder for [`FieldSpec`].
#[derive(Debug)]
pub struct FieldSpecBuilder<L: CodeLang> {
    name: String,
    field_type: TypeName<L>,
    modifiers: Modifiers,
    doc: Vec<String>,
    initializer: Option<CodeBlock<L>>,
    annotations: Vec<CodeBlock<L>>,
    annotation_specs: Vec<AnnotationSpec<L>>,
    tag: Option<String>,
}

impl<L: CodeLang> FieldSpecBuilder<L> {
    /// Set the visibility modifier.
    pub fn visibility(&mut self, vis: Visibility) -> &mut Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this field as static.
    pub fn is_static(&mut self) -> &mut Self {
        self.modifiers.is_static = true;
        self
    }

    /// Mark this field as readonly.
    pub fn is_readonly(&mut self) -> &mut Self {
        self.modifiers.is_readonly = true;
        self
    }

    /// Add a doc comment line.
    pub fn doc(&mut self, line: &str) -> &mut Self {
        self.doc.push(line.to_string());
        self
    }

    /// Set the field initializer expression.
    pub fn initializer(&mut self, init: CodeBlock<L>) -> &mut Self {
        self.initializer = Some(init);
        self
    }

    /// Add a raw annotation [`CodeBlock`].
    pub fn annotation(&mut self, ann: CodeBlock<L>) -> &mut Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured [`AnnotationSpec`].
    pub fn annotate(&mut self, spec: AnnotationSpec<L>) -> &mut Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Set the struct tag (e.g., Go's `` `json:"name"` ``).
    pub fn tag(&mut self, t: &str) -> &mut Self {
        self.tag = Some(t.to_string());
        self
    }

    /// Build the [`FieldSpec`] from this builder.
    pub fn build(self) -> FieldSpec<L> {
        FieldSpec {
            name: self.name,
            field_type: self.field_type,
            modifiers: self.modifiers,
            doc: self.doc,
            initializer: self.initializer,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            tag: self.tag,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::rust_lang::RustLang;
    use crate::lang::typescript::TypeScript;

    fn emit_field_ts(spec: &FieldSpec<TypeScript>, ctx: DeclarationContext) -> String {
        let lang = TypeScript::new();
        let block = spec.emit(&lang, ctx);
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
        renderer.render(&block)
    }

    fn emit_field_rs(spec: &FieldSpec<RustLang>, ctx: DeclarationContext) -> String {
        let lang = RustLang::new();
        let block = spec.emit(&lang, ctx);
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
        renderer.render(&block)
    }

    #[test]
    fn test_ts_field_basic() {
        let field = FieldSpec::builder("name", TypeName::<TypeScript>::primitive("string")).build();
        let output = emit_field_ts(&field, DeclarationContext::Member);
        assert_eq!(output.trim(), "name: string;");
    }

    #[test]
    fn test_ts_field_with_visibility() {
        let mut fb = FieldSpec::builder("name", TypeName::<TypeScript>::primitive("string"));
        fb.visibility(Visibility::Private);
        let field = fb.build();
        let output = emit_field_ts(&field, DeclarationContext::Member);
        assert_eq!(output.trim(), "private name: string;");
    }

    #[test]
    fn test_rust_field_basic() {
        let mut fb = FieldSpec::builder("name", TypeName::<RustLang>::primitive("String"));
        fb.visibility(Visibility::Public);
        let field = fb.build();
        let output = emit_field_rs(&field, DeclarationContext::Member);
        assert_eq!(output.trim(), "pub name: String,");
    }

    #[test]
    fn test_ts_field_readonly_static() {
        let mut fb = FieldSpec::builder("MAX", TypeName::<TypeScript>::primitive("number"));
        fb.is_static();
        fb.is_readonly();
        let field = fb.build();
        let output = emit_field_ts(&field, DeclarationContext::Member);
        assert_eq!(output.trim(), "static readonly MAX: number;");
    }
}
