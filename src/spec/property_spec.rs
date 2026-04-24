//! Property specification with getter/setter support.
//!
//! `PropertySpec` renders computed properties in two styles:
//!
//! - **Accessor** (TS/JS and fallback): `get name(): T { ... }` / `set name(v: T) { ... }`
//! - **Field** (Swift/Kotlin): field with inline `get`/`set` body blocks

use crate::code_block::{Arg, CodeBlock};
use crate::lang::CodeLang;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::modifiers::{DeclarationContext, Modifiers, PropertyStyle, Visibility};
use crate::type_name::TypeName;

/// A setter definition: parameter name + body.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SetterSpec {
    pub(crate) param_name: String,
    pub(crate) body: CodeBlock,
}

/// A computed property with optional getter and setter.
///
/// `PropertySpec` renders computed properties in two styles depending on the
/// target language:
///
/// - **Accessor** (TS/JS and fallback): `get name(): T { ... }` / `set name(v: T) { ... }`
/// - **Field** (Swift/Kotlin): field with inline `get`/`set` body blocks
///
/// Use [`PropertySpec::builder()`] to construct, then add to a
/// [`TypeSpec`](crate::spec::type_spec::TypeSpec) with `add_property()`.
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::spec::property_spec::PropertySpec;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let getter_body = CodeBlock::of("return this._name", ()).unwrap();
///
/// let prop = PropertySpec::builder("name", TypeName::primitive("string"))
///     .getter(getter_body)
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PropertySpec {
    pub(crate) name: String,
    pub(crate) property_type: TypeName,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) getter: Option<CodeBlock>,
    pub(crate) setter: Option<SetterSpec>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
}

impl PropertySpec {
    /// Create a new builder for a property with the given name and type.
    pub fn builder(name: &str, property_type: TypeName) -> PropertySpecBuilder {
        PropertySpecBuilder {
            name: name.to_string(),
            property_type,
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            getter: None,
            setter: None,
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
        }
    }

    /// Return the property name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Emit this property as one or more CodeBlocks.
    ///
    /// Accessor style returns 1–2 blocks (getter, setter).
    /// Field style returns 1 block (field with inline body).
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
    ) -> Result<Vec<CodeBlock>, crate::error::SigilStitchError> {
        match lang.property_style() {
            PropertyStyle::Accessor => self.emit_accessor(lang, ctx),
            PropertyStyle::Field => self.emit_field(lang, ctx),
        }
    }

    /// Emit as accessor methods: `get name(): T { ... }` / `set name(v: T) { ... }`.
    fn emit_accessor(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
    ) -> Result<Vec<CodeBlock>, crate::error::SigilStitchError> {
        let mut blocks = Vec::new();

        if let Some(getter_body) = &self.getter {
            let mut cb = CodeBlock::builder();

            // Annotations + doc on the getter.
            self.emit_preamble(&mut cb, lang)?;

            // Signature: [vis] get [name]()[return_type_sep][type][block_open]
            let vis = lang.render_visibility(self.modifiers.visibility, ctx);
            let mut sig = String::new();
            let mut sig_args: Vec<Arg> = Vec::new();

            sig.push_str(vis);
            if self.modifiers.is_static {
                sig.push_str("static ");
            }
            sig.push_str("get ");
            sig.push_str(&self.name);
            sig.push_str("()");

            if !self.property_type.is_empty() {
                sig.push_str(lang.function_syntax().return_type_separator);
                sig.push_str("%T");
                sig_args.push(Arg::TypeName(self.property_type.clone()));
            }

            sig.push_str(lang.block_syntax().block_open);
            cb.add(&sig, sig_args);
            cb.add_line();
            cb.add("%>", ());
            cb.add_code(getter_body.clone());
            cb.add_line();
            cb.add("%<", ());
            let close = lang.block_syntax().block_close;
            if !close.is_empty() {
                cb.add(close, ());
                cb.add_line();
            }

            blocks.push(cb.build()?);
        }

        if let Some(setter) = &self.setter {
            let mut cb = CodeBlock::builder();

            // Setter signature: [vis] set [name]([param][sep][type])[block_open]
            let vis = lang.render_visibility(self.modifiers.visibility, ctx);
            let mut sig = String::new();
            let mut sig_args: Vec<Arg> = Vec::new();

            sig.push_str(vis);
            if self.modifiers.is_static {
                sig.push_str("static ");
            }
            sig.push_str("set ");
            sig.push_str(&self.name);
            sig.push('(');
            sig.push_str(&lang.escape_reserved(&setter.param_name));

            if !self.property_type.is_empty() {
                sig.push_str(lang.type_decl_syntax().type_annotation_separator);
                sig.push_str("%T");
                sig_args.push(Arg::TypeName(self.property_type.clone()));
            }

            sig.push(')');
            sig.push_str(lang.block_syntax().block_open);
            cb.add(&sig, sig_args);
            cb.add_line();
            cb.add("%>", ());
            cb.add_code(setter.body.clone());
            cb.add_line();
            cb.add("%<", ());
            let close = lang.block_syntax().block_close;
            if !close.is_empty() {
                cb.add(close, ());
                cb.add_line();
            }

            blocks.push(cb.build()?);
        }

        Ok(blocks)
    }

    /// Emit as a field with inline getter/setter body (Swift/Kotlin).
    fn emit_field(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
    ) -> Result<Vec<CodeBlock>, crate::error::SigilStitchError> {
        let mut cb = CodeBlock::builder();

        // Annotations + doc.
        self.emit_preamble(&mut cb, lang)?;

        // Field header: [vis] [var/let] [name]: [type] {
        let vis = lang.render_visibility(self.modifiers.visibility, ctx);
        let has_setter = self.setter.is_some();

        let mut sig = String::new();
        let mut sig_args: Vec<Arg> = Vec::new();

        sig.push_str(vis);
        if self.modifiers.is_static {
            sig.push_str("static ");
        }

        if has_setter {
            sig.push_str(lang.enum_and_annotation().mutable_field_keyword);
        } else {
            sig.push_str(lang.enum_and_annotation().readonly_keyword);
        }

        sig.push_str(&lang.escape_reserved(&self.name));

        if !self.property_type.is_empty() {
            sig.push_str(lang.type_decl_syntax().type_annotation_separator);
            sig.push_str("%T");
            sig_args.push(Arg::TypeName(self.property_type.clone()));
        }

        sig.push_str(lang.block_syntax().block_open);
        cb.add(&sig, sig_args);
        cb.add_line();
        cb.add("%>", ());

        // Getter block.
        if let Some(getter_body) = &self.getter {
            let getter_kw = lang.property_getter_keyword();
            let getter_sig = format!("{getter_kw}{}", lang.block_syntax().block_open);
            cb.add(&getter_sig, ());
            cb.add_line();
            cb.add("%>", ());
            cb.add_code(getter_body.clone());
            cb.add_line();
            cb.add("%<", ());
            let close = lang.block_syntax().block_close;
            if !close.is_empty() {
                cb.add(close, ());
                cb.add_line();
            }
        }

        // Setter block.
        if let Some(setter) = &self.setter {
            let setter_sig = format!(
                "set({}){}",
                setter.param_name,
                lang.block_syntax().block_open
            );
            cb.add(&setter_sig, ());
            cb.add_line();
            cb.add("%>", ());
            cb.add_code(setter.body.clone());
            cb.add_line();
            cb.add("%<", ());
            let close = lang.block_syntax().block_close;
            if !close.is_empty() {
                cb.add(close, ());
                cb.add_line();
            }
        }

        cb.add("%<", ());
        let close = lang.block_syntax().block_close;
        if !close.is_empty() {
            cb.add(close, ());
            cb.add_line();
        }

        Ok(vec![cb.build()?])
    }

    /// Emit annotations and doc comment as a preamble.
    fn emit_preamble(
        &self,
        cb: &mut crate::code_block::CodeBlockBuilder,
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
}

/// Builder for [`PropertySpec`].
#[derive(Debug)]
pub struct PropertySpecBuilder {
    name: String,
    property_type: TypeName,
    modifiers: Modifiers,
    doc: Vec<String>,
    getter: Option<CodeBlock>,
    setter: Option<SetterSpec>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
}

impl PropertySpecBuilder {
    /// Set the getter body.
    pub fn getter(mut self, body: CodeBlock) -> Self {
        self.getter = Some(body);
        self
    }

    /// Set the setter parameter name and body.
    pub fn setter(mut self, param_name: &str, body: CodeBlock) -> Self {
        self.setter = Some(SetterSpec {
            param_name: param_name.to_string(),
            body,
        });
        self
    }

    /// Set the visibility.
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this property as static.
    pub fn is_static(mut self) -> Self {
        self.modifiers.is_static = true;
        self
    }

    /// Add a doc comment line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
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

    /// Build the [`PropertySpec`].
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`](crate::error::SigilStitchError::EmptyName) if `name` is empty.
    pub fn build(self) -> Result<PropertySpec, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "PropertySpecBuilder",
            }
        );
        Ok(PropertySpec {
            name: self.name,
            property_type: self.property_type,
            modifiers: self.modifiers,
            doc: self.doc,
            getter: self.getter,
            setter: self.setter,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_empty_name_errors() {
        let result = PropertySpec::builder("", TypeName::primitive("string")).build();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }
}
