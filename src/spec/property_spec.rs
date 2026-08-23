//! Computed-property specification with read/write accessor behavior.
//!
//! The selected language adapter validates and completely lowers each
//! property. Specs carry semantic behavior and metadata, not accessor spelling
//! or placement policy.

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::capability::{PropertyCapability, PropertyContext};
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::modifiers::{DeclarationContext, Modifiers, TypeKind, Visibility};
use crate::type_name::TypeName;

/// Read-only semantic intent for one computed property.
#[derive(Debug, Clone, Copy)]
pub struct PropertyIntent<'a> {
    property: &'a PropertySpec,
    context: PropertyContext,
    owner_name: Option<&'a str>,
}

impl<'a> PropertyIntent<'a> {
    fn direct(property: &'a PropertySpec, declaration_context: DeclarationContext) -> Self {
        Self {
            property,
            context: PropertyContext::Direct(declaration_context),
            owner_name: None,
        }
    }

    pub(crate) fn type_member(
        property: &'a PropertySpec,
        owner_name: &'a str,
        owner_kind: TypeKind,
    ) -> Self {
        Self {
            property,
            context: PropertyContext::TypeMember(owner_kind),
            owner_name: Some(owner_name),
        }
    }

    /// The semantic property declaration.
    pub fn property(&self) -> &'a PropertySpec {
        self.property
    }

    /// Semantic context in which the property is emitted.
    pub fn context(&self) -> PropertyContext {
        self.context
    }

    /// Name of the owning type, when available.
    pub fn owner_name(&self) -> Option<&'a str> {
        self.owner_name
    }
}

/// Property intent whose intrinsic and target validation succeeded.
///
/// Only sigil-stitch constructs this wrapper, so a lowerer cannot bypass the
/// selected adapter's capability profile and additive validation.
#[derive(Debug, Clone)]
pub struct ValidatedProperty<'a> {
    intent: PropertyIntent<'a>,
}

impl<'a> ValidatedProperty<'a> {
    fn new(intent: PropertyIntent<'a>) -> Self {
        Self { intent }
    }
}

impl<'a> std::ops::Deref for ValidatedProperty<'a> {
    type Target = PropertyIntent<'a>;

    fn deref(&self) -> &Self::Target {
        &self.intent
    }
}

/// A setter definition: parameter name plus implementation body.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SetterSpec {
    pub(crate) param_name: String,
    pub(crate) body: CodeBlock,
}

impl SetterSpec {
    /// Name bound to the assigned value.
    pub fn param_name(&self) -> &str {
        &self.param_name
    }

    /// Implementation body for write access.
    pub fn body(&self) -> &CodeBlock {
        &self.body
    }
}

/// A computed property with optional read and write behavior.
///
/// The selected language adapter decides whether this intent becomes accessor
/// methods, a field-style computed property, or another valid target construct.
/// Use [`PropertySpec::builder()`] to construct a property, then add it to a
/// [`TypeSpec`](crate::spec::type_spec::TypeSpec) with `add_property()`.
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::spec::property_spec::PropertySpec;
///
/// let getter_body = CodeBlock::of("return this._name", ()).unwrap();
/// let property = PropertySpec::builder("name", TypeName::primitive("string"))
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
    /// Create a builder for a property with the given name and value type.
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

    /// Return the explicitly requested property type.
    pub fn property_type(&self) -> &TypeName {
        &self.property_type
    }

    /// Return semantic property modifiers.
    pub fn modifiers(&self) -> &Modifiers {
        &self.modifiers
    }

    /// Return documentation lines supplied by the caller.
    pub fn doc(&self) -> &[String] {
        &self.doc
    }

    /// Return the read-access implementation body, when present.
    pub fn getter(&self) -> Option<&CodeBlock> {
        self.getter.as_ref()
    }

    /// Return the write-access declaration, when present.
    pub fn setter(&self) -> Option<&SetterSpec> {
        self.setter.as_ref()
    }

    /// Return opaque annotation blocks supplied through the escape hatch.
    pub fn annotations(&self) -> &[CodeBlock] {
        &self.annotations
    }

    /// Return structured annotation declarations.
    pub fn annotation_specs(&self) -> &[AnnotationSpec] {
        &self.annotation_specs
    }

    /// Validate and emit this property as one or more structured blocks.
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
        context: DeclarationContext,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        Self::lower_intent(PropertyIntent::direct(self, context), lang)
    }

    pub(crate) fn collect_intrinsic_validation_errors(
        &self,
        context: PropertyContext,
        errors: &mut Vec<SigilStitchError>,
    ) {
        if self.name.is_empty() {
            errors.push(SigilStitchError::EmptyName {
                builder: "PropertySpec",
            });
        }
        if self.getter.is_none() && self.setter.is_none() {
            errors.push(SigilStitchError::MissingPropertyAccessors {
                property_name: self.name.clone(),
                context,
            });
        }
        if self.getter.as_ref().is_some_and(CodeBlock::is_empty) {
            errors.push(SigilStitchError::EmptyPropertyOperand {
                property_name: self.name.clone(),
                context,
                operand: "getter body",
            });
        }
        if let Some(setter) = &self.setter {
            if setter.param_name.is_empty() {
                errors.push(SigilStitchError::EmptyPropertySetterParameter {
                    property_name: self.name.clone(),
                    context,
                });
            }
            if setter.body.is_empty() {
                errors.push(SigilStitchError::EmptyPropertyOperand {
                    property_name: self.name.clone(),
                    context,
                    operand: "setter body",
                });
            }
        }

        let mut invalid_modifiers = Vec::new();
        if self.modifiers.is_abstract {
            invalid_modifiers.push("abstract");
        }
        if self.modifiers.is_readonly {
            invalid_modifiers.push("readonly");
        }
        if self.modifiers.is_async {
            invalid_modifiers.push("async");
        }
        if self.modifiers.is_override {
            invalid_modifiers.push("override");
        }
        if self.modifiers.is_constructor {
            invalid_modifiers.push("constructor");
        }
        if !invalid_modifiers.is_empty() {
            errors.push(SigilStitchError::InvalidPropertyModifiers {
                property_name: self.name.clone(),
                context,
                modifiers: invalid_modifiers,
            });
        }
    }

    pub(crate) fn collect_validation_errors(
        intent: PropertyIntent<'_>,
        lang: &dyn CodeLang,
        errors: &mut Vec<SigilStitchError>,
    ) {
        let property = intent.property();
        property.collect_intrinsic_validation_errors(intent.context(), errors);

        let capabilities = lang.capabilities();
        if !capabilities.supports_property_context(intent.context()) {
            errors.push(SigilStitchError::UnsupportedPropertyContext {
                language: lang.file_extension().to_string(),
                context: intent.context(),
                property_name: property.name.clone(),
                owner_name: intent.owner_name().map(str::to_string),
            });
            return;
        }

        let requested = property.requested_capabilities();
        let unsupported: Vec<_> = requested
            .iter()
            .copied()
            .filter(|capability| {
                !capabilities.supports_property_capability(intent.context(), *capability)
            })
            .collect();
        if !unsupported.is_empty() {
            errors.push(SigilStitchError::UnsupportedPropertyCapabilities {
                language: lang.file_extension().to_string(),
                property_name: property.name.clone(),
                context: intent.context(),
                capabilities: unsupported,
            });
        }

        let missing: Vec<_> = capabilities
            .required_property_capabilities(intent.context())
            .iter()
            .copied()
            .filter(|capability| !requested.contains(capability))
            .collect();
        if !missing.is_empty() {
            errors.push(SigilStitchError::MissingRequiredPropertyCapabilities {
                language: lang.file_extension().to_string(),
                property_name: property.name.clone(),
                context: intent.context(),
                capabilities: missing,
            });
        }

        lang.collect_property_validation_errors(intent, errors);
    }

    pub(crate) fn validate_intent<'a>(
        intent: PropertyIntent<'a>,
        lang: &dyn CodeLang,
    ) -> Result<ValidatedProperty<'a>, SigilStitchError> {
        let mut errors = Vec::new();
        Self::collect_validation_errors(intent, lang, &mut errors);
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        Ok(ValidatedProperty::new(intent))
    }

    pub(crate) fn lower_intent(
        intent: PropertyIntent<'_>,
        lang: &dyn CodeLang,
    ) -> Result<Vec<CodeBlock>, SigilStitchError> {
        let property = Self::validate_intent(intent, lang)?;
        lang.lower_property(property)
    }

    fn requested_capabilities(&self) -> Vec<PropertyCapability> {
        let mut capabilities = Vec::new();
        if !self.property_type.is_empty() {
            capabilities.push(PropertyCapability::ExplicitType);
        }
        if self.getter.is_some() {
            capabilities.push(PropertyCapability::ReadAccessor);
        }
        if self.setter.is_some() {
            capabilities.push(PropertyCapability::WriteAccessor);
        }
        if !self.annotations.is_empty() || !self.annotation_specs.is_empty() {
            capabilities.push(PropertyCapability::Attributes);
        }
        if self.modifiers.is_static {
            capabilities.push(PropertyCapability::StaticProperty);
        }
        capabilities
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
    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.modifiers.visibility = visibility;
        self
    }

    /// Mark this property as static.
    pub fn is_static(mut self) -> Self {
        self.modifiers.is_static = true;
        self
    }

    /// Add a documentation line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }

    /// Add a raw annotation block.
    pub fn annotation(mut self, annotation: CodeBlock) -> Self {
        self.annotations.push(annotation);
        self
    }

    /// Add a structured annotation.
    pub fn annotate(mut self, annotation: AnnotationSpec) -> Self {
        self.annotation_specs.push(annotation);
        self
    }

    /// Build the [`PropertySpec`].
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    pub fn build(self) -> Result<PropertySpec, SigilStitchError> {
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
    fn build_empty_name_errors() {
        let result = PropertySpec::builder("", TypeName::primitive("string")).build();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }

    #[test]
    fn built_in_validation_hooks_accept_and_reject_property_intent() {
        let languages: Vec<Box<dyn CodeLang>> = vec![
            Box::new(crate::lang::javascript::JavaScript::new()),
            Box::new(crate::lang::typescript::TypeScript::new()),
            Box::new(crate::lang::kotlin::Kotlin::new()),
            Box::new(crate::lang::swift::Swift::new()),
            Box::new(crate::lang::php::Php::new()),
            Box::new(crate::lang::scala::Scala::new()),
        ];

        for lang in languages {
            let property_type = if lang.file_extension() == "js" {
                TypeName::primitive("")
            } else {
                TypeName::primitive("Value")
            };
            let accepted = PropertySpec::builder("value", property_type.clone())
                .getter(CodeBlock::of("return stored", ()).unwrap())
                .build()
                .unwrap();
            assert!(
                lang.validate_property(PropertyIntent::direct(
                    &accepted,
                    DeclarationContext::Member,
                ))
                .is_ok(),
                ".{}",
                lang.file_extension()
            );

            let rejected = PropertySpec::builder("bad-name", property_type)
                .getter(CodeBlock::of("return stored", ()).unwrap())
                .build()
                .unwrap();
            assert!(
                lang.validate_property(PropertyIntent::direct(
                    &rejected,
                    DeclarationContext::Member,
                ))
                .is_err(),
                ".{}",
                lang.file_extension()
            );
        }
    }
}
