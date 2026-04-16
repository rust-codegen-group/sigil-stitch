//! Type specification for structs, classes, interfaces, traits, enums.

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::lang::CodeLang;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::enum_variant_spec::EnumVariantSpec;
use crate::spec::field_spec::FieldSpec;
use crate::spec::fun_spec::{FunSpec, TypeParamSpec, render_type_params};
use crate::spec::modifiers::{DeclarationContext, Modifiers, TypeKind, Visibility};
use crate::spec::property_spec::PropertySpec;
use crate::type_name::TypeName;

/// A type declaration (struct, class, interface, trait, enum).
#[derive(Debug, Clone)]
pub struct TypeSpec<L: CodeLang> {
    pub(crate) name: String,
    pub(crate) kind: TypeKind,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) fields: Vec<FieldSpec<L>>,
    pub(crate) properties: Vec<PropertySpec<L>>,
    pub(crate) methods: Vec<FunSpec<L>>,
    pub(crate) type_params: Vec<TypeParamSpec<L>>,
    pub(crate) super_types: Vec<TypeName<L>>,
    pub(crate) impl_types: Vec<TypeName<L>>,
    pub(crate) annotations: Vec<CodeBlock<L>>,
    pub(crate) annotation_specs: Vec<AnnotationSpec<L>>,
    pub(crate) extra_members: Vec<CodeBlock<L>>,
    pub(crate) variants: Vec<EnumVariantSpec<L>>,
}

impl<L: CodeLang> TypeSpec<L> {
    /// Create a new builder for a type declaration with the given name and kind.
    pub fn builder(name: &str, kind: TypeKind) -> TypeSpecBuilder<L> {
        TypeSpecBuilder {
            name: name.to_string(),
            kind,
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            fields: Vec::new(),
            properties: Vec::new(),
            methods: Vec::new(),
            type_params: Vec::new(),
            super_types: Vec::new(),
            impl_types: Vec::new(),
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            extra_members: Vec::new(),
            variants: Vec::new(),
        }
    }

    /// Return the name of this type.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the kind of this type (struct, class, interface, etc.).
    pub fn kind(&self) -> TypeKind {
        self.kind
    }

    /// Emit this type as one or more CodeBlocks.
    ///
    /// Returns a `Vec` because Rust struct + impl = two separate blocks,
    /// while TypeScript class = one block.
    pub fn emit(&self, lang: &L) -> Vec<CodeBlock<L>> {
        if lang.methods_inside_type_body(self.kind) {
            vec![self.emit_inline(lang)]
        } else {
            self.emit_split(lang)
        }
    }

    /// Emit as a single block with methods inside the body (TypeScript class/interface, Rust trait).
    fn emit_inline(&self, lang: &L) -> CodeBlock<L> {
        let mut cb = CodeBlock::<L>::builder();

        self.emit_preamble(&mut cb, lang);
        self.emit_header(&mut cb, lang);

        // Body.
        cb.add("%>", ());
        // Docstring inside body (Python).
        if !self.doc.is_empty() && lang.doc_comment_inside_body() {
            let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
            let doc_str = lang.render_doc_comment(&doc_lines);
            cb.add("%L", doc_str);
            cb.add_line();
        }
        for (i, field) in self.fields.iter().enumerate() {
            if i > 0 {
                // No extra blank line between fields.
            }
            cb.add_code(field.emit(lang, DeclarationContext::Member));
        }
        // Enum variants.
        if !self.variants.is_empty() {
            if !self.fields.is_empty() {
                cb.add_line();
            }
            self.emit_variants(&mut cb, lang);
        }
        let has_body_above = !self.fields.is_empty() || !self.variants.is_empty();
        // Properties (after fields, before methods).
        if !self.properties.is_empty() {
            if has_body_above {
                cb.add_line();
            }
            for (i, prop) in self.properties.iter().enumerate() {
                if i > 0 {
                    cb.add_line();
                }
                for block in prop.emit(lang, DeclarationContext::Member) {
                    cb.add_code(block);
                }
            }
        }
        let has_body_above = has_body_above || !self.properties.is_empty();
        if has_body_above && !self.methods.is_empty() {
            cb.add_line();
        }
        for (i, method) in self.methods.iter().enumerate() {
            if i > 0 {
                cb.add_line();
            }
            cb.add_code(method.emit(lang, DeclarationContext::Member));
        }
        for extra in &self.extra_members {
            cb.add_code(extra.clone());
        }
        cb.add("%<", ());
        let close = lang.block_close();
        if !close.is_empty() {
            let term = lang.type_close_terminator();
            cb.add(&format!("{close}{term}"), ());
            cb.add_line();
        }

        cb.build().unwrap()
    }

    /// Emit as separate struct + impl blocks (Rust struct/enum).
    fn emit_split(&self, lang: &L) -> Vec<CodeBlock<L>> {
        let mut blocks = Vec::new();

        // Block 1: struct/enum definition.
        let mut cb = CodeBlock::<L>::builder();
        self.emit_preamble(&mut cb, lang);
        self.emit_header(&mut cb, lang);

        cb.add("%>", ());
        for field in &self.fields {
            cb.add_code(field.emit(lang, DeclarationContext::Member));
        }
        // Enum variants.
        if !self.variants.is_empty() {
            if !self.fields.is_empty() {
                cb.add_line();
            }
            self.emit_variants(&mut cb, lang);
        }
        for extra in &self.extra_members {
            cb.add_code(extra.clone());
        }
        cb.add("%<", ());
        let close = lang.block_close();
        if !close.is_empty() {
            let term = lang.type_close_terminator();
            cb.add(&format!("{close}{term}"), ());
            cb.add_line();
        }
        blocks.push(cb.build().unwrap());

        // Block 2: impl block (only if methods or properties are non-empty).
        if !self.methods.is_empty() || !self.properties.is_empty() {
            let mut impl_cb = CodeBlock::<L>::builder();
            let mut impl_fmt = String::from("impl");
            let mut impl_args: Vec<Arg<L>> = Vec::new();

            // Type params on impl.
            let tp_str = render_type_params(&self.type_params, lang, &mut impl_args);
            impl_fmt.push_str(&tp_str);
            impl_fmt.push(' ');
            impl_fmt.push_str(&self.name);
            // Repeat bare type param names.
            if !self.type_params.is_empty() {
                impl_fmt.push_str(lang.generic_open());
                for (i, tp) in self.type_params.iter().enumerate() {
                    if i > 0 {
                        impl_fmt.push_str(", ");
                    }
                    impl_fmt.push_str(&tp.name);
                }
                impl_fmt.push_str(lang.generic_close());
            }
            impl_fmt.push_str(lang.block_open());
            impl_cb.add(&impl_fmt, impl_args);
            impl_cb.add_line();

            impl_cb.add("%>", ());
            // Properties before methods.
            for (i, prop) in self.properties.iter().enumerate() {
                if i > 0 {
                    impl_cb.add_line();
                }
                for block in prop.emit(lang, DeclarationContext::Member) {
                    impl_cb.add_code(block);
                }
            }
            if !self.properties.is_empty() && !self.methods.is_empty() {
                impl_cb.add_line();
            }
            for (i, method) in self.methods.iter().enumerate() {
                if i > 0 {
                    impl_cb.add_line();
                }
                impl_cb.add_code(method.emit(lang, DeclarationContext::Member));
            }
            impl_cb.add("%<", ());
            let close = lang.block_close();
            if !close.is_empty() {
                impl_cb.add(close, ());
                impl_cb.add_line();
            }

            blocks.push(impl_cb.build().unwrap());
        }

        blocks
    }

    /// Emit enum variants with language-aware separators.
    fn emit_variants(&self, cb: &mut CodeBlockBuilder<L>, lang: &L) {
        let sep = lang.enum_variant_separator();
        let trailing = lang.enum_variant_trailing_separator();
        let count = self.variants.len();
        let field_term = lang.field_terminator();

        for (i, variant) in self.variants.iter().enumerate() {
            // Emit variant parts directly here rather than calling variant.emit(),
            // because we need to append the separator before the trailing newline.
            for spec in &variant.annotation_specs {
                cb.add_code(spec.emit(lang));
                cb.add_line();
            }
            for ann in &variant.annotations {
                cb.add_code(ann.clone());
                cb.add_line();
            }
            if !variant.doc.is_empty() && !lang.doc_comment_inside_body() {
                let doc_lines: Vec<&str> = variant.doc.iter().map(|s| s.as_str()).collect();
                let doc_str = lang.render_doc_comment(&doc_lines);
                cb.add("%L", doc_str);
                cb.add_line();
            }

            let prefix = lang.enum_variant_prefix();
            let mut fmt = String::new();
            let mut args: Vec<Arg<L>> = Vec::new();
            fmt.push_str(prefix);
            fmt.push_str(&variant.name);

            // Tuple/associated types: Name(Type1, Type2)
            if !variant.associated_types.is_empty() {
                fmt.push('(');
                for (j, ty) in variant.associated_types.iter().enumerate() {
                    if j > 0 {
                        fmt.push_str(", ");
                    }
                    fmt.push_str("%T");
                    args.push(Arg::TypeName(ty.clone()));
                }
                fmt.push(')');
            }

            // Struct fields: Name { field: Type, ... }
            if !variant.fields.is_empty() {
                let is_last = i == count - 1;
                let needs_sep = !sep.is_empty() && (!is_last || trailing);

                fmt.push_str(" {");
                cb.add(&fmt, args);
                cb.add_line();
                cb.add("%>", ());
                for field in &variant.fields {
                    let vis = lang.render_visibility(
                        field.modifiers.visibility,
                        crate::spec::modifiers::DeclarationContext::Member,
                    );
                    let mut f_fmt = String::new();
                    let mut f_args: Vec<Arg<L>> = Vec::new();
                    f_fmt.push_str(vis);
                    if lang.type_before_name() {
                        if !field.field_type.is_empty() {
                            f_fmt.push_str("%T");
                            f_args.push(Arg::TypeName(field.field_type.clone()));
                            f_fmt.push(' ');
                        }
                        f_fmt.push_str(&field.name);
                    } else {
                        f_fmt.push_str(&field.name);
                        if !field.field_type.is_empty() {
                            let type_sep = lang.type_annotation_separator();
                            f_fmt.push_str(type_sep);
                            f_fmt.push_str("%T");
                            f_args.push(Arg::TypeName(field.field_type.clone()));
                        }
                    }
                    f_fmt.push_str(field_term);
                    cb.add(&f_fmt, f_args);
                    cb.add_line();
                }
                cb.add("%<", ());
                if needs_sep {
                    cb.add(&format!("}}{sep}"), ());
                } else {
                    cb.add("}", ());
                }
                cb.add_line();
                continue;
            }

            if let Some(val) = &variant.value {
                fmt.push_str(" = %L");
                args.push(Arg::Code(val.clone()));
            }

            let is_last = i == count - 1;
            if !sep.is_empty() && (!is_last || trailing) {
                fmt.push_str(sep);
            }

            cb.add(&fmt, args);
            cb.add_line();
        }
    }

    /// Emit annotations and doc comment.
    fn emit_preamble(&self, cb: &mut CodeBlockBuilder<L>, lang: &L) {
        for spec in &self.annotation_specs {
            cb.add_code(spec.emit(lang));
            cb.add_line();
        }
        for ann in &self.annotations {
            cb.add_code(ann.clone());
            cb.add_line();
        }
        if !self.doc.is_empty() && !lang.doc_comment_inside_body() {
            let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
            let doc_str = lang.render_doc_comment(&doc_lines);
            cb.add("%L", doc_str);
            cb.add_line();
        }
    }

    /// Emit the type header line: `{vis}{keyword} {name}<params>{extends}{implements} {`.
    fn emit_header(&self, cb: &mut CodeBlockBuilder<L>, lang: &L) {
        let vis = lang.render_visibility(self.modifiers.visibility, DeclarationContext::TopLevel);
        let kw = lang.type_keyword(self.kind);

        let mut fmt = String::new();
        let mut args: Vec<Arg<L>> = Vec::new();

        fmt.push_str(vis);
        if self.modifiers.is_abstract {
            fmt.push_str("abstract ");
        }
        fmt.push_str(kw);
        fmt.push(' ');
        fmt.push_str(&self.name);

        // Type parameters.
        let tp_str = render_type_params(&self.type_params, lang, &mut args);
        fmt.push_str(&tp_str);

        // Super types (extends).
        if !self.super_types.is_empty() {
            let super_kw = lang.super_type_keyword();
            if !super_kw.is_empty() {
                fmt.push_str(super_kw);
                let sep = lang.super_type_separator();
                for (i, st) in self.super_types.iter().enumerate() {
                    if i > 0 {
                        fmt.push_str(sep);
                    }
                    fmt.push_str("%T");
                    args.push(Arg::TypeName(st.clone()));
                }
            }
        }

        // Implements.
        if !self.impl_types.is_empty() {
            let impl_kw = lang.implements_keyword();
            if !impl_kw.is_empty() {
                fmt.push_str(impl_kw);
                for (i, it) in self.impl_types.iter().enumerate() {
                    if i > 0 {
                        fmt.push_str(", ");
                    }
                    fmt.push_str("%T");
                    args.push(Arg::TypeName(it.clone()));
                }
            }
        }

        // Kind suffix (e.g., Go: "type Foo struct").
        let suffix = lang.type_kind_suffix(self.kind);
        if !suffix.is_empty() {
            fmt.push(' ');
            fmt.push_str(suffix);
        }

        // Close bases list (e.g., Python: ")").
        if !self.super_types.is_empty() || !self.impl_types.is_empty() {
            let bases_close = lang.bases_close();
            if !bases_close.is_empty() {
                fmt.push_str(bases_close);
            }
        }

        fmt.push_str(lang.block_open());
        cb.add(&fmt, args);
        cb.add_line();
    }
}

/// Builder for [`TypeSpec`].
#[derive(Debug)]
pub struct TypeSpecBuilder<L: CodeLang> {
    name: String,
    kind: TypeKind,
    modifiers: Modifiers,
    doc: Vec<String>,
    fields: Vec<FieldSpec<L>>,
    properties: Vec<PropertySpec<L>>,
    methods: Vec<FunSpec<L>>,
    type_params: Vec<TypeParamSpec<L>>,
    super_types: Vec<TypeName<L>>,
    impl_types: Vec<TypeName<L>>,
    annotations: Vec<CodeBlock<L>>,
    annotation_specs: Vec<AnnotationSpec<L>>,
    extra_members: Vec<CodeBlock<L>>,
    variants: Vec<EnumVariantSpec<L>>,
}

impl<L: CodeLang> TypeSpecBuilder<L> {
    /// Set the visibility modifier.
    pub fn visibility(&mut self, vis: Visibility) -> &mut Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this type as abstract.
    pub fn is_abstract(&mut self) -> &mut Self {
        self.modifiers.is_abstract = true;
        self
    }

    /// Add a documentation comment line.
    pub fn doc(&mut self, line: &str) -> &mut Self {
        self.doc.push(line.to_string());
        self
    }

    /// Add a field to this type.
    pub fn add_field(&mut self, field: FieldSpec<L>) -> &mut Self {
        self.fields.push(field);
        self
    }

    /// Add a computed property to this type.
    pub fn add_property(&mut self, prop: PropertySpec<L>) -> &mut Self {
        self.properties.push(prop);
        self
    }

    /// Add a method to this type.
    pub fn add_method(&mut self, method: FunSpec<L>) -> &mut Self {
        self.methods.push(method);
        self
    }

    /// Add a type parameter (generic).
    pub fn add_type_param(&mut self, tp: TypeParamSpec<L>) -> &mut Self {
        self.type_params.push(tp);
        self
    }

    /// Add a super type (extends / inherits from).
    pub fn extends(&mut self, super_type: TypeName<L>) -> &mut Self {
        self.super_types.push(super_type);
        self
    }

    /// Add an implemented interface.
    pub fn implements(&mut self, iface: TypeName<L>) -> &mut Self {
        self.impl_types.push(iface);
        self
    }

    /// Add a raw annotation code block.
    pub fn annotation(&mut self, ann: CodeBlock<L>) -> &mut Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured annotation.
    pub fn annotate(&mut self, spec: AnnotationSpec<L>) -> &mut Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Add an extra code block to the type body.
    pub fn extra_member(&mut self, block: CodeBlock<L>) -> &mut Self {
        self.extra_members.push(block);
        self
    }

    /// Add an enum variant. Only meaningful when `kind` is `TypeKind::Enum`.
    pub fn add_variant(&mut self, variant: EnumVariantSpec<L>) -> &mut Self {
        self.variants.push(variant);
        self
    }

    /// Consume the builder and produce a [`TypeSpec`].
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty.
    pub fn build(self) -> TypeSpec<L> {
        assert!(
            !self.name.is_empty(),
            "TypeSpecBuilder::build() failed: 'name' must not be empty (kind: {:?})",
            self.kind,
        );
        TypeSpec {
            name: self.name,
            kind: self.kind,
            modifiers: self.modifiers,
            doc: self.doc,
            fields: self.fields,
            properties: self.properties,
            methods: self.methods,
            type_params: self.type_params,
            super_types: self.super_types,
            impl_types: self.impl_types,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            extra_members: self.extra_members,
            variants: self.variants,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::rust_lang::RustLang;
    use crate::lang::typescript::TypeScript;
    use crate::spec::parameter_spec::ParameterSpec;

    fn render_blocks_ts(blocks: &[CodeBlock<TypeScript>]) -> String {
        let lang = TypeScript::new();
        let imports = crate::import::ImportGroup::new();
        let mut output = String::new();
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
            output.push_str(&renderer.render(block));
        }
        output
    }

    fn render_blocks_rs(blocks: &[CodeBlock<RustLang>]) -> String {
        let lang = RustLang::new();
        let imports = crate::import::ImportGroup::new();
        let mut output = String::new();
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
            output.push_str(&renderer.render(block));
        }
        output
    }

    #[test]
    fn test_ts_class() {
        let mut tb = TypeSpec::<TypeScript>::builder("UserService", TypeKind::Class);
        tb.visibility(Visibility::Public);
        let mut field_b = FieldSpec::builder("name", TypeName::primitive("string"));
        field_b.visibility(Visibility::Private);
        tb.add_field(field_b.build());
        let body = CodeBlock::<TypeScript>::of("return this.name", ()).unwrap();
        let mut fb = FunSpec::builder("getName");
        fb.returns(TypeName::primitive("string"));
        fb.body(body);
        tb.add_method(fb.build());
        let ts = tb.build();

        let blocks = ts.emit(&TypeScript::new());
        let output = render_blocks_ts(&blocks);
        assert!(output.contains("export class UserService {"));
        assert!(output.contains("private name: string;"));
        assert!(output.contains("getName(): string {"));
        assert!(output.contains("return this.name"));
    }

    #[test]
    fn test_ts_interface() {
        let mut tb = TypeSpec::<TypeScript>::builder("Repository", TypeKind::Interface);
        tb.visibility(Visibility::Public);
        tb.add_method({
            let mut fb = FunSpec::builder("findById");
            fb.add_param(ParameterSpec::new("id", TypeName::primitive("string")));
            fb.returns(TypeName::generic(
                TypeName::primitive("Promise"),
                vec![TypeName::primitive("Entity")],
            ));
            fb.build()
        });
        let ts = tb.build();

        let blocks = ts.emit(&TypeScript::new());
        let output = render_blocks_ts(&blocks);
        assert!(output.contains("export interface Repository {"));
        assert!(output.contains("findById(id: string): Promise<Entity>;"));
    }

    #[test]
    fn test_rust_struct_with_impl() {
        let mut tb = TypeSpec::<RustLang>::builder("Config", TypeKind::Struct);
        tb.visibility(Visibility::Public);
        tb.add_field({
            let mut fb = FieldSpec::builder("name", TypeName::primitive("String"));
            fb.visibility(Visibility::Public);
            fb.build()
        });
        let body = CodeBlock::<RustLang>::of("Self { name: name.to_string() }", ()).unwrap();
        let mut fb = FunSpec::<RustLang>::builder("new");
        fb.visibility(Visibility::Public);
        fb.add_param(ParameterSpec::new("name", TypeName::primitive("&str")));
        fb.returns(TypeName::primitive("Self"));
        fb.body(body);
        tb.add_method(fb.build());
        let ts = tb.build();

        let blocks = ts.emit(&RustLang::new());
        let output = render_blocks_rs(&blocks);
        // Should have separate struct and impl blocks.
        assert!(output.contains("pub struct Config {"));
        assert!(output.contains("pub name: String,"));
        assert!(output.contains("impl Config {"));
        assert!(output.contains("pub fn new(name: &str) -> Self {"));
    }

    #[test]
    fn test_ts_class_extends_implements() {
        let mut tb = TypeSpec::<TypeScript>::builder("AdminService", TypeKind::Class);
        tb.visibility(Visibility::Public);
        tb.extends(TypeName::primitive("BaseService"));
        tb.implements(TypeName::primitive("Serializable"));
        let ts = tb.build();

        let blocks = ts.emit(&TypeScript::new());
        let output = render_blocks_ts(&blocks);
        assert!(
            output.contains(
                "export class AdminService extends BaseService implements Serializable {"
            )
        );
    }

    #[test]
    #[should_panic(expected = "TypeSpecBuilder::build() failed: 'name' must not be empty")]
    fn test_build_empty_name_panics() {
        TypeSpec::<TypeScript>::builder("", TypeKind::Class).build();
    }
}
