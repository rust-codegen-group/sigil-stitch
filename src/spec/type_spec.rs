//! Type specification for structs, classes, interfaces, traits, enums.

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::enum_variant_spec::EnumVariantSpec;
use crate::spec::field_spec::FieldSpec;
use crate::spec::fun_spec::{
    FunSpec, TypeParamSpec, WhereClauseStyle, WhereConstraint, emit_where_block, render_type_params,
};
use crate::spec::modifiers::{DeclarationContext, Modifiers, TypeKind, Visibility};
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::property_spec::PropertySpec;
use crate::type_name::TypeName;

/// A type declaration (struct, class, interface, trait, enum).
///
/// `TypeSpec` models a complete type declaration with fields, methods, properties,
/// type parameters, supertype relationships, annotations, and enum variants.
/// It emits one or more `CodeBlock`s depending on the language: TypeScript classes
/// produce a single block, while Rust structs produce separate struct + impl blocks
/// (controlled by [`CodeLang::methods_inside_type_body()`](crate::lang::CodeLang::methods_inside_type_body)).
///
/// Use [`TypeSpec::builder()`] to construct, then add to a
/// [`FileSpec`](crate::spec::file_spec::FileSpec) with `add_type()`.
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let body = CodeBlock::of("return this.name", ()).unwrap();
/// let type_spec = TypeSpec::builder("UserService", TypeKind::Class)
///     .visibility(Visibility::Public)
///     .add_field(
///         FieldSpec::builder("name", TypeName::primitive("string")).build().unwrap(),
///     )
///     .add_method(
///         FunSpec::builder("getName")
///             .returns(TypeName::primitive("string"))
///             .body(body)
///             .build().unwrap(),
///     )
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeSpec {
    pub(crate) name: String,
    pub(crate) kind: TypeKind,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    #[serde(default)]
    pub(crate) embedded_types: Vec<TypeName>,
    pub(crate) fields: Vec<FieldSpec>,
    pub(crate) properties: Vec<PropertySpec>,
    pub(crate) methods: Vec<FunSpec>,
    pub(crate) type_params: Vec<TypeParamSpec>,
    pub(crate) super_types: Vec<TypeName>,
    pub(crate) impl_types: Vec<TypeName>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
    pub(crate) extra_members: Vec<CodeBlock>,
    pub(crate) variants: Vec<EnumVariantSpec>,
    /// Primary constructor parameters (Kotlin: `class Foo(val x: Int, val y: String)`).
    pub(crate) primary_constructor: Vec<ParameterSpec>,
    /// Where-clause constraints (e.g., Rust `where T: Clone + Send`).
    #[serde(default)]
    pub(crate) where_constraints: Vec<WhereConstraint>,
}

impl TypeSpec {
    /// Create a new builder for a type declaration with the given name and kind.
    pub fn builder(name: &str, kind: TypeKind) -> TypeSpecBuilder {
        TypeSpecBuilder {
            name: name.to_string(),
            kind,
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            embedded_types: Vec::new(),
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
            primary_constructor: Vec::new(),
            where_constraints: Vec::new(),
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
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<Vec<CodeBlock>, crate::error::SigilStitchError> {
        match self.kind {
            TypeKind::TypeAlias => return Ok(vec![self.emit_type_alias(lang)?]),
            TypeKind::Newtype => return Ok(vec![self.emit_newtype(lang)?]),
            _ => {}
        }
        if lang.methods_inside_type_body(self.kind) {
            Ok(vec![self.emit_inline(lang)?])
        } else {
            self.emit_split(lang)
        }
    }

    /// Emit as a single block with methods inside the body (TypeScript class/interface, Rust trait).
    fn emit_inline(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let mut cb = CodeBlock::builder();

        // Use InterfaceMember context for interface/trait bodies so that
        // languages can suppress visibility modifiers and async keywords.
        let member_ctx = match self.kind {
            TypeKind::Interface | TypeKind::Trait => DeclarationContext::InterfaceMember,
            _ => DeclarationContext::Member,
        };

        self.emit_preamble(&mut cb, lang)?;
        self.emit_header(&mut cb, lang)?;

        // Body.
        cb.add("%>", ());
        // Type body prefix (e.g., Haskell record braces: "Person {").
        let body_prefix = lang.type_body_prefix(&self.name, self.kind);
        let has_body_prefix = !body_prefix.is_empty();
        if has_body_prefix {
            cb.add("%L", body_prefix);
            cb.add_line();
            cb.add("%>", ());
        }
        // Docstring inside body (Python).
        if !self.doc.is_empty() && lang.doc_comment_inside_body() {
            let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
            let doc_str = lang.render_doc_comment(&doc_lines);
            cb.add("%L", doc_str);
            cb.add_line();
        }
        let ea = lang.enum_and_annotation();
        let has_trailing_members =
            !self.fields.is_empty() || !self.properties.is_empty() || !self.methods.is_empty();

        // Embedded types (Go struct composition: unnamed type references).
        for embedded in &self.embedded_types {
            let term = lang.block_syntax().field_terminator;
            cb.add(&format!("%T{term}"), embedded.clone());
            cb.add_line();
        }

        if ea.variants_before_fields {
            // Variants first (Java/Kotlin pattern).
            if !self.variants.is_empty() {
                self.emit_variants(&mut cb, lang, has_trailing_members)?;
            }
            for (i, field) in self.fields.iter().enumerate() {
                if i == 0 && !self.variants.is_empty() {
                    cb.add_line();
                }
                cb.add_code(field.emit(lang, member_ctx)?);
            }
        } else {
            // Fields first, then variants (default).
            for (i, field) in self.fields.iter().enumerate() {
                if i > 0 {
                    // No extra blank line between fields.
                }
                cb.add_code(field.emit(lang, member_ctx)?);
            }
            if !self.variants.is_empty() {
                if !self.fields.is_empty() {
                    cb.add_line();
                }
                self.emit_variants(&mut cb, lang, has_trailing_members)?;
            }
        }
        let has_body_above =
            !self.embedded_types.is_empty() || !self.fields.is_empty() || !self.variants.is_empty();
        // Properties (after fields, before methods).
        if !self.properties.is_empty() {
            if has_body_above {
                cb.add_line();
            }
            for (i, prop) in self.properties.iter().enumerate() {
                if i > 0 {
                    cb.add_line();
                }
                for block in prop.emit(lang, member_ctx)? {
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
            cb.add_code(method.emit(lang, member_ctx)?);
        }
        for extra in &self.extra_members {
            cb.add_code(extra.clone());
        }
        // Type body suffix (e.g., Haskell record closing brace: "}").
        if has_body_prefix {
            cb.add("%<", ());
        }
        let body_suffix = lang.type_body_suffix(&self.name, self.kind);
        if !body_suffix.is_empty() {
            cb.add("%L", body_suffix);
            cb.add_line();
        }
        cb.add("%<", ());
        let block_syn = lang.block_syntax();
        let close = block_syn.block_close;
        let type_close_suffix = self.render_impl_type_suffix(lang);
        if !close.is_empty() {
            let term = block_syn.type_close_terminator;
            cb.add(&format!("{close}{term}"), ());
            if !type_close_suffix.is_empty() {
                cb.add("%L", type_close_suffix);
            }
            cb.add_line();
        } else if !type_close_suffix.is_empty() {
            cb.add("%L", type_close_suffix);
            cb.add_line();
        }

        cb.build()
    }

    /// Emit as separate struct + impl blocks (Rust struct/enum).
    fn emit_split(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<Vec<CodeBlock>, crate::error::SigilStitchError> {
        let mut blocks = Vec::new();

        // Block 1: struct/enum definition.
        let mut cb = CodeBlock::builder();
        self.emit_preamble(&mut cb, lang)?;
        self.emit_header(&mut cb, lang)?;

        cb.add("%>", ());
        // Type body prefix (e.g., Haskell record braces).
        let body_prefix = lang.type_body_prefix(&self.name, self.kind);
        let has_body_prefix = !body_prefix.is_empty();
        if has_body_prefix {
            cb.add("%L", body_prefix);
            cb.add_line();
            cb.add("%>", ());
        }
        // Embedded types (Go struct composition).
        for embedded in &self.embedded_types {
            let term = lang.block_syntax().field_terminator;
            cb.add(&format!("%T{term}"), embedded.clone());
            cb.add_line();
        }
        for field in &self.fields {
            cb.add_code(field.emit(lang, DeclarationContext::Member)?);
        }
        // Enum variants.
        if !self.variants.is_empty() {
            if !self.fields.is_empty() {
                cb.add_line();
            }
            let has_trailing = !self.extra_members.is_empty();
            self.emit_variants(&mut cb, lang, has_trailing)?;
        }
        for extra in &self.extra_members {
            cb.add_code(extra.clone());
        }
        // Type body suffix (e.g., Haskell record closing brace).
        if has_body_prefix {
            cb.add("%<", ());
        }
        let body_suffix = lang.type_body_suffix(&self.name, self.kind);
        if !body_suffix.is_empty() {
            cb.add("%L", body_suffix);
            cb.add_line();
        }
        cb.add("%<", ());
        let block_syn = lang.block_syntax();
        let close = block_syn.block_close;
        let type_close_suffix = self.render_impl_type_suffix(lang);
        if !close.is_empty() {
            let term = block_syn.type_close_terminator;
            cb.add(&format!("{close}{term}"), ());
            if !type_close_suffix.is_empty() {
                cb.add("%L", type_close_suffix);
            }
            cb.add_line();
        } else if !type_close_suffix.is_empty() {
            cb.add("%L", type_close_suffix);
            cb.add_line();
        }
        blocks.push(cb.build()?);

        // Block 2: impl block (only if methods or properties are non-empty).
        if !self.methods.is_empty() || !self.properties.is_empty() {
            let mut impl_cb = CodeBlock::builder();
            let mut impl_fmt = String::from("impl");
            let mut impl_args: Vec<Arg> = Vec::new();

            // Type params on impl.
            let tp_str = render_type_params(&self.type_params, lang, &mut impl_args);
            impl_fmt.push_str(&tp_str);
            impl_fmt.push(' ');
            impl_fmt.push_str(&self.name);
            // Repeat bare type param names.
            let gen_syn = lang.generic_syntax();
            if !self.type_params.is_empty() {
                impl_fmt.push_str(gen_syn.open);
                for (i, tp) in self.type_params.iter().enumerate() {
                    if i > 0 {
                        impl_fmt.push_str(", ");
                    }
                    impl_fmt.push_str(&tp.name);
                }
                impl_fmt.push_str(gen_syn.close);
            }
            // Where clause on impl block.
            if !self.where_constraints.is_empty()
                && lang.function_syntax().where_clause_style == WhereClauseStyle::WhereBlock
            {
                emit_where_block(&mut impl_fmt, &mut impl_args, &self.where_constraints, lang);
                impl_fmt.push_str("\n{");
            } else {
                impl_fmt.push_str(lang.block_syntax().block_open);
            }
            impl_cb.add(&impl_fmt, impl_args);
            impl_cb.add_line();

            impl_cb.add("%>", ());
            // Properties before methods.
            for (i, prop) in self.properties.iter().enumerate() {
                if i > 0 {
                    impl_cb.add_line();
                }
                for block in prop.emit(lang, DeclarationContext::Member)? {
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
                impl_cb.add_code(method.emit(lang, DeclarationContext::Member)?);
            }
            impl_cb.add("%<", ());
            let close = lang.block_syntax().block_close;
            if !close.is_empty() {
                impl_cb.add(close, ());
                impl_cb.add_line();
            }

            blocks.push(impl_cb.build()?);
        }

        Ok(blocks)
    }

    /// Emit a type alias declaration: `type Name = Target;`.
    fn emit_type_alias(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let mut cb = CodeBlock::builder();
        let mut args: Vec<Arg> = Vec::new();

        self.emit_preamble(&mut cb, lang)?;

        let vis = lang.render_visibility(self.modifiers.visibility, DeclarationContext::TopLevel);
        let kw = lang.type_keyword(self.kind);
        let tp_str = render_type_params(&self.type_params, lang, &mut args);

        let target = self
            .super_types
            .first()
            .cloned()
            .unwrap_or_else(|| TypeName::primitive(""));

        let semi = if lang.block_syntax().uses_semicolons {
            ";"
        } else {
            ""
        };

        let fmt = if lang.type_decl_syntax().type_alias_target_first {
            // C typedef: `typedef target name;`
            args.push(Arg::TypeName(target));
            format!("{kw} %T {}{tp_str}{semi}", self.name)
        } else {
            // Normal: `{vis}type name<params> = target;`
            args.push(Arg::TypeName(target));
            format!("{vis}{kw} {}{tp_str} = %T{semi}", self.name)
        };

        cb.add(&fmt, args);
        cb.add_line();
        cb.build()
    }

    /// Emit a newtype wrapper declaration.
    fn emit_newtype(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let mut cb = CodeBlock::builder();

        self.emit_preamble(&mut cb, lang)?;

        let vis = lang.render_visibility(self.modifiers.visibility, DeclarationContext::TopLevel);
        let target = self
            .super_types
            .first()
            .cloned()
            .unwrap_or_else(|| TypeName::primitive(""));

        let resolve = |_module: &str, name: &str| name.to_string();
        let inner_str = target.render(80, &resolve).unwrap_or_default();

        let mut tp_args: Vec<Arg> = Vec::new();
        let tp_str = render_type_params(&self.type_params, lang, &mut tp_args);
        let name_with_params = format!("{}{tp_str}", self.name);

        let line = lang.render_newtype_line(vis, &name_with_params, &inner_str);

        // Build format string: the line is literal, but type param bounds need %T resolution.
        if tp_args.is_empty() {
            cb.add("%L", line);
        } else {
            // The line contains type param bound placeholders — pass args through.
            cb.add(&line, tp_args);
        }
        cb.add_line();
        cb.build()
    }

    /// Emit enum variants with language-aware separators.
    fn emit_variants(
        &self,
        cb: &mut CodeBlockBuilder,
        lang: &dyn CodeLang,
        has_trailing_members: bool,
    ) -> Result<(), crate::error::SigilStitchError> {
        let count = self.variants.len();
        for (i, variant) in self.variants.iter().enumerate() {
            let ctx = crate::spec::enum_variant_spec::VariantContext {
                is_first: i == 0,
                is_last: i == count - 1,
                has_trailing_members,
            };
            variant.emit_into(cb, lang, ctx)?;
        }
        Ok(())
    }

    /// Render impl_types to strings and pass them to `lang.render_type_close_suffix()`.
    fn render_impl_type_suffix(&self, lang: &dyn CodeLang) -> String {
        if self.impl_types.is_empty() {
            let empty: Vec<String> = Vec::new();
            return lang.render_type_close_suffix(self.kind, &empty);
        }
        let resolve = |_module: &str, name: &str| name.to_string();
        let impl_names: Vec<String> = self
            .impl_types
            .iter()
            .filter_map(|t| t.render(80, &resolve).ok())
            .collect();
        lang.render_type_close_suffix(self.kind, &impl_names)
    }

    /// Emit annotations and doc comment.
    fn emit_preamble(
        &self,
        cb: &mut CodeBlockBuilder,
        lang: &dyn CodeLang,
    ) -> Result<(), crate::error::SigilStitchError> {
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

        Ok(())
    }

    /// Emit the type header line: `{vis}{keyword} {name}<params>(primary ctor){extends}{implements} {`.
    fn emit_header(
        &self,
        cb: &mut CodeBlockBuilder,
        lang: &dyn CodeLang,
    ) -> Result<(), crate::error::SigilStitchError> {
        let vis = lang.render_visibility(self.modifiers.visibility, DeclarationContext::TopLevel);
        let kw = lang.type_keyword(self.kind);

        let mut fmt = String::new();
        let mut args: Vec<Arg> = Vec::new();

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

        let tds = lang.type_decl_syntax();

        // Primary constructor parameters (Kotlin: `class Foo(val x: Int, val y: String)`).
        if !self.primary_constructor.is_empty() && tds.supports_primary_constructor {
            fmt.push('(');
            fmt.push_str("%L");
            let params_block = self.build_primary_constructor_block(lang)?;
            args.push(Arg::Code(params_block));
            fmt.push(')');
        }

        // Super types (extends).
        if !self.super_types.is_empty() {
            let super_kw = tds.super_type_keyword;
            if !super_kw.is_empty() {
                fmt.push_str(super_kw);
                let sep = tds.super_type_separator;
                let subsequent_sep = tds.super_type_subsequent_separator;
                for (i, st) in self.super_types.iter().enumerate() {
                    if i > 0 {
                        fmt.push_str(subsequent_sep.unwrap_or(sep));
                    }
                    fmt.push_str("%T");
                    args.push(Arg::TypeName(st.clone()));
                }
            }
        }

        // Implements.
        if !self.impl_types.is_empty() {
            let impl_kw = tds.implements_keyword;
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
            let bases_close = lang.block_syntax().bases_close;
            if !bases_close.is_empty() {
                fmt.push_str(bases_close);
            }
        }

        // Where clause (Rust-style).
        if !self.where_constraints.is_empty()
            && lang.function_syntax().where_clause_style == WhereClauseStyle::WhereBlock
        {
            emit_where_block(&mut fmt, &mut args, &self.where_constraints, lang);
            fmt.push_str("\n{");
        } else {
            fmt.push_str(lang.type_header_block_open(self.kind));
        }
        cb.add(&fmt, args);
        cb.add_line();
        Ok(())
    }

    /// Build a CodeBlock for primary constructor parameters.
    fn build_primary_constructor_block(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let mut pb = CodeBlock::builder();
        for (i, param) in self.primary_constructor.iter().enumerate() {
            if i > 0 {
                pb.add(",%W", ());
            }
            param.emit_into(&mut pb, lang);
        }
        pb.build()
    }
}

/// Builder for [`TypeSpec`].
#[derive(Debug)]
pub struct TypeSpecBuilder {
    name: String,
    kind: TypeKind,
    modifiers: Modifiers,
    doc: Vec<String>,
    embedded_types: Vec<TypeName>,
    fields: Vec<FieldSpec>,
    properties: Vec<PropertySpec>,
    methods: Vec<FunSpec>,
    type_params: Vec<TypeParamSpec>,
    super_types: Vec<TypeName>,
    impl_types: Vec<TypeName>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    extra_members: Vec<CodeBlock>,
    variants: Vec<EnumVariantSpec>,
    primary_constructor: Vec<ParameterSpec>,
    where_constraints: Vec<WhereConstraint>,
}

impl TypeSpecBuilder {
    /// Set the visibility modifier.
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this type as abstract.
    pub fn is_abstract(mut self) -> Self {
        self.modifiers.is_abstract = true;
        self
    }

    /// Add a documentation comment line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }

    /// Add an embedded type (Go struct composition).
    ///
    /// Embedded types render as unnamed type references inside the struct body:
    /// ```go
    /// type UserAdmin struct {
    ///     User
    ///     Admin
    /// }
    /// ```
    pub fn add_embedded(mut self, type_name: TypeName) -> Self {
        self.embedded_types.push(type_name);
        self
    }

    /// Add a field to this type.
    pub fn add_field(mut self, field: FieldSpec) -> Self {
        self.fields.push(field);
        self
    }

    /// Add a computed property to this type.
    pub fn add_property(mut self, prop: PropertySpec) -> Self {
        self.properties.push(prop);
        self
    }

    /// Add a method to this type.
    pub fn add_method(mut self, method: FunSpec) -> Self {
        self.methods.push(method);
        self
    }

    /// Add a type parameter (generic).
    pub fn add_type_param(mut self, tp: TypeParamSpec) -> Self {
        self.type_params.push(tp);
        self
    }

    /// Add a super type (extends / inherits from).
    pub fn extends(mut self, super_type: TypeName) -> Self {
        self.super_types.push(super_type);
        self
    }

    /// Add an implemented interface.
    pub fn implements(mut self, iface: TypeName) -> Self {
        self.impl_types.push(iface);
        self
    }

    /// Add a raw annotation code block.
    pub fn annotation(mut self, ann: CodeBlock) -> Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured annotation.
    pub fn annotate(mut self, spec: AnnotationSpec) -> Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Add an extra code block to the type body.
    pub fn extra_member(mut self, block: CodeBlock) -> Self {
        self.extra_members.push(block);
        self
    }

    /// Add an enum variant. Only meaningful when `kind` is `TypeKind::Enum`.
    pub fn add_variant(mut self, variant: EnumVariantSpec) -> Self {
        self.variants.push(variant);
        self
    }

    /// Add a primary constructor parameter.
    ///
    /// When the language supports primary constructors (`supports_primary_constructor()`),
    /// these parameters are rendered in the type header after the name:
    /// `class Foo(val x: Int, val y: String)`.
    ///
    /// For languages that don't support primary constructors, these are ignored.
    pub fn add_primary_constructor_param(mut self, param: ParameterSpec) -> Self {
        self.primary_constructor.push(param);
        self
    }

    /// Add a where-clause constraint (e.g., `T: Clone + Send`).
    pub fn add_where_constraint(mut self, subject: TypeName, bounds: Vec<TypeName>) -> Self {
        self.where_constraints
            .push(WhereConstraint { subject, bounds });
        self
    }

    /// Consume the builder and produce a [`TypeSpec`].
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    /// Returns [`SigilStitchError::DuplicateFieldName`] if any two fields share the same name.
    pub fn build(self) -> Result<TypeSpec, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "TypeSpecBuilder",
            }
        );

        // Check for duplicate field names.
        let mut seen = std::collections::HashSet::new();
        for field in &self.fields {
            if !seen.insert(field.name()) {
                return Err(crate::error::SigilStitchError::DuplicateFieldName {
                    type_name: self.name.clone(),
                    field_name: field.name().to_string(),
                });
            }
        }

        // Validate TypeAlias / Newtype constraints.
        if matches!(self.kind, TypeKind::TypeAlias | TypeKind::Newtype) {
            let kind_str = if self.kind == TypeKind::TypeAlias {
                "TypeAlias"
            } else {
                "Newtype"
            };
            if self.super_types.len() != 1 {
                return Err(crate::error::SigilStitchError::InvalidTypeAlias {
                    kind: kind_str,
                    type_name: self.name.clone(),
                    reason: format!(
                        "expected exactly 1 super_type (the target type), got {}",
                        self.super_types.len()
                    ),
                });
            }
            if !self.fields.is_empty()
                || !self.methods.is_empty()
                || !self.variants.is_empty()
                || !self.properties.is_empty()
            {
                return Err(crate::error::SigilStitchError::InvalidTypeAlias {
                    kind: kind_str,
                    type_name: self.name.clone(),
                    reason: "must not have fields, methods, variants, or properties".to_string(),
                });
            }
        }

        Ok(TypeSpec {
            name: self.name,
            kind: self.kind,
            modifiers: self.modifiers,
            doc: self.doc,
            embedded_types: self.embedded_types,
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
            primary_constructor: self.primary_constructor,
            where_constraints: self.where_constraints,
        })
    }
}

impl crate::spec::emittable::Emittable for TypeSpec {
    fn emit_members(&self, lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
        self.emit(lang)
    }
}
