//! Type specification for structs, classes, interfaces, traits, enums.

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::capability::{FunctionCapability, FunctionForm, TypeCapability};
use crate::spec::annotation_spec::{AnnotationNameRef, AnnotationSpec};
use crate::spec::enum_variant_spec::{
    ConstructorArity, EnumVariantSpec, ValidatedVariants, VariantOwnerContext,
};
use crate::spec::field_spec::{FieldSequenceIntent, FieldSpec, ValidatedFields};
use crate::spec::fun_spec::{FunSpec, ValidatedFunction};
use crate::spec::modifiers::{Modifiers, TypeKind, Visibility};
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::property_spec::{PropertyIntent, PropertySpec, ValidatedProperty};
use crate::spec::type_members_intent::TypeMembersIntent;
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

/// A type declaration (struct, class, interface, trait, enum).
///
/// `TypeSpec` models a complete type declaration with fields, methods, properties,
/// type parameters, supertype relationships, annotations, and enum variants.
/// It emits one or more non-empty `CodeBlock`s through the selected language's
/// complete [`CodeLang::lower_type()`] implementation. TypeScript classes
/// produce a single block, while Rust may produce a declaration plus an `impl`
/// block.
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
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) closed_sum: bool,
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

fn is_false(value: &bool) -> bool {
    !*value
}

/// Read-only semantic intent for one complete type declaration.
///
/// Only `TypeSpec` constructs this view. It exposes declaration facts without
/// target keywords, placement choices, ordering flags, or rendered type names.
#[derive(Debug, Clone, Copy)]
pub struct TypeIntent<'a> {
    spec: &'a TypeSpec,
}

impl<'a> TypeIntent<'a> {
    fn new(spec: &'a TypeSpec) -> Self {
        Self { spec }
    }

    /// Declaration name.
    pub fn name(self) -> &'a str {
        &self.spec.name
    }

    /// Semantic declaration kind.
    pub fn kind(self) -> TypeKind {
        self.spec.kind
    }

    /// Whether this declaration is a closed sum.
    pub fn is_closed_sum(self) -> bool {
        self.spec.closed_sum
    }

    /// Type-level semantic modifiers.
    pub fn modifiers(self) -> &'a Modifiers {
        &self.spec.modifiers
    }

    /// Documentation lines supplied by the caller.
    pub fn doc(self) -> &'a [String] {
        &self.spec.doc
    }

    /// Structurally embedded types.
    pub fn embedded_types(self) -> &'a [TypeName] {
        &self.spec.embedded_types
    }

    /// Declared fields.
    pub fn fields(self) -> &'a [FieldSpec] {
        &self.spec.fields
    }

    /// Declared computed properties.
    pub fn properties(self) -> &'a [PropertySpec] {
        &self.spec.properties
    }

    /// Declared methods before owner-aware classification.
    pub fn methods(self) -> &'a [FunSpec] {
        &self.spec.methods
    }

    /// Declared type parameters.
    pub fn type_params(self) -> &'a [TypeParamSpec] {
        &self.spec.type_params
    }

    /// Alias or newtype target, when this declaration has one.
    pub fn target_type(self) -> Option<&'a TypeName> {
        matches!(self.spec.kind, TypeKind::TypeAlias | TypeKind::Newtype)
            .then(|| self.spec.super_types.first())
            .flatten()
    }

    /// Nominal supertypes. Alias/newtype backing storage is excluded.
    pub fn nominal_super_types(self) -> &'a [TypeName] {
        if matches!(self.spec.kind, TypeKind::TypeAlias | TypeKind::Newtype) {
            &[]
        } else {
            &self.spec.super_types
        }
    }

    /// Implemented contracts or derived classes.
    pub fn implemented_types(self) -> &'a [TypeName] {
        &self.spec.impl_types
    }

    /// Opaque annotation blocks supplied through the escape hatch.
    pub fn annotations(self) -> &'a [CodeBlock] {
        &self.spec.annotations
    }

    /// Structured annotation declarations.
    pub fn annotation_specs(self) -> &'a [AnnotationSpec] {
        &self.spec.annotation_specs
    }

    /// Opaque member blocks supplied through the escape hatch.
    pub fn extra_members(self) -> &'a [CodeBlock] {
        &self.spec.extra_members
    }

    /// Declared variants.
    pub fn variants(self) -> &'a [EnumVariantSpec] {
        &self.spec.variants
    }

    /// Primary-constructor parameters.
    pub fn primary_constructor_parameters(self) -> &'a [ParameterSpec] {
        &self.spec.primary_constructor
    }

    /// Explicit declaration constraints.
    pub fn where_constraints(self) -> &'a [WhereConstraint] {
        &self.spec.where_constraints
    }
}

/// Complete type intent whose type-level and child validation succeeded.
///
/// Unlike smaller validated wrappers, this type deliberately does not
/// dereference to `TypeIntent`: complete type lowerers must compose the
/// validated child declarations exposed here.
#[derive(Debug, Clone)]
pub struct ValidatedType<'a> {
    intent: TypeIntent<'a>,
    fields: Option<ValidatedFields<'a>>,
    properties: Vec<ValidatedProperty<'a>>,
    methods: Vec<ValidatedFunction<'a>>,
    variants: Option<ValidatedVariants<'a>>,
}

impl<'a> ValidatedType<'a> {
    fn new(
        intent: TypeIntent<'a>,
        fields: Option<ValidatedFields<'a>>,
        properties: Vec<ValidatedProperty<'a>>,
        methods: Vec<ValidatedFunction<'a>>,
        variants: Option<ValidatedVariants<'a>>,
    ) -> Self {
        Self {
            intent,
            fields,
            properties,
            methods,
            variants,
        }
    }

    /// Declaration name.
    pub fn name(&self) -> &'a str {
        self.intent.name()
    }

    /// Semantic declaration kind.
    pub fn kind(&self) -> TypeKind {
        self.intent.kind()
    }

    /// Whether this declaration is a closed sum.
    pub fn is_closed_sum(&self) -> bool {
        self.intent.is_closed_sum()
    }

    /// Type-level semantic modifiers.
    pub fn modifiers(&self) -> &'a Modifiers {
        self.intent.modifiers()
    }

    /// Documentation lines supplied by the caller.
    pub fn doc(&self) -> &'a [String] {
        self.intent.doc()
    }

    /// Structurally embedded types.
    pub fn embedded_types(&self) -> &'a [TypeName] {
        self.intent.embedded_types()
    }

    /// Validated field sequence, when non-empty.
    pub fn fields(&self) -> Option<&ValidatedFields<'a>> {
        self.fields.as_ref()
    }

    /// Validated computed properties in declaration order.
    pub fn properties(&self) -> &[ValidatedProperty<'a>] {
        &self.properties
    }

    /// Validated, owner-classified methods in declaration order.
    pub fn methods(&self) -> &[ValidatedFunction<'a>] {
        &self.methods
    }

    /// Declared type parameters.
    pub fn type_params(&self) -> &'a [TypeParamSpec] {
        self.intent.type_params()
    }

    /// Alias or newtype target, when present.
    pub fn target_type(&self) -> Option<&'a TypeName> {
        self.intent.target_type()
    }

    /// Nominal supertypes, excluding alias/newtype backing storage.
    pub fn nominal_super_types(&self) -> &'a [TypeName] {
        self.intent.nominal_super_types()
    }

    /// Implemented contracts or derived classes.
    pub fn implemented_types(&self) -> &'a [TypeName] {
        self.intent.implemented_types()
    }

    /// Opaque annotation blocks supplied through the escape hatch.
    pub fn annotations(&self) -> &'a [CodeBlock] {
        self.intent.annotations()
    }

    /// Structured annotation declarations.
    pub fn annotation_specs(&self) -> &'a [AnnotationSpec] {
        self.intent.annotation_specs()
    }

    /// Opaque member blocks supplied through the escape hatch.
    pub fn extra_members(&self) -> &'a [CodeBlock] {
        self.intent.extra_members()
    }

    /// Validated variant sequence, including an intentional empty closed sum.
    pub fn variants(&self) -> Option<&ValidatedVariants<'a>> {
        self.variants.as_ref()
    }

    /// Primary-constructor parameters.
    pub fn primary_constructor_parameters(&self) -> &'a [ParameterSpec] {
        self.intent.primary_constructor_parameters()
    }

    /// Explicit declaration constraints.
    pub fn where_constraints(&self) -> &'a [WhereConstraint] {
        self.intent.where_constraints()
    }
}

impl TypeSpec {
    /// Create a new builder for a type declaration with the given name and kind.
    pub fn builder(name: &str, kind: TypeKind) -> TypeSpecBuilder {
        Self::builder_with_closed_sum(name, kind, false)
    }

    /// Create a builder for a type with a complete set of named cases.
    pub fn closed_sum(name: &str) -> TypeSpecBuilder {
        Self::builder_with_closed_sum(name, TypeKind::Enum, true)
    }

    fn builder_with_closed_sum(name: &str, kind: TypeKind, closed_sum: bool) -> TypeSpecBuilder {
        TypeSpecBuilder {
            name: name.to_string(),
            kind,
            closed_sum,
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

    /// Whether this declaration is a closed sum.
    pub fn is_closed_sum(&self) -> bool {
        self.closed_sum
    }

    /// Validate this type against the language capability matrix.
    ///
    /// Type-level validation and method validation are checked in declaration
    /// order; the first error is returned. For file-level aggregation use the
    /// crate-private collector instead.
    pub fn validate(&self, lang: &dyn CodeLang) -> Result<(), crate::error::SigilStitchError> {
        let mut errors = Vec::new();
        self.collect_validation_errors(lang, &mut errors);
        match errors.into_iter().next() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Push every type-level and method validation error into `errors`.
    pub(crate) fn collect_validation_errors(
        &self,
        lang: &dyn CodeLang,
        errors: &mut Vec<SigilStitchError>,
    ) {
        let intent = TypeIntent::new(self);
        self.collect_intrinsic_type_validation_errors(errors);
        self.collect_type_capability_errors(lang, errors);
        lang.collect_type_validation_errors(intent, errors);

        if self.closed_sum || !self.variants.is_empty() {
            let has_non_variant_members = !self.fields.is_empty()
                || !self.properties.is_empty()
                || !self.methods.is_empty()
                || !self.embedded_types.is_empty()
                || !self.extra_members.is_empty();
            EnumVariantSpec::collect_sequence_validation_errors(
                &self.name,
                self.kind,
                self.closed_sum,
                &self.variants,
                self.variant_owner_context(lang, has_non_variant_members),
                lang,
                errors,
            );
        }

        if !self.fields.is_empty() {
            let intent = self.field_intent();
            if lang
                .capabilities()
                .supports_type_capability(self.kind, TypeCapability::RecordFields)
            {
                FieldSpec::collect_sequence_validation_errors(intent, lang, errors);
            } else {
                FieldSpec::collect_sequence_intrinsic_validation_errors(intent, errors);
            }
        }

        for property in &self.properties {
            let intent = self.property_intent(property);
            if lang
                .capabilities()
                .supports_type_capability(self.kind, TypeCapability::AccessorMethods)
            {
                PropertySpec::collect_validation_errors(intent, lang, errors);
            } else {
                property.collect_intrinsic_validation_errors(intent.context(), errors);
            }
        }

        let declaration_context = lang.type_member_declaration_context(self.kind);
        for method in &self.methods {
            match method.validate_in_type(lang, declaration_context, &self.name) {
                Ok(method) => {
                    let capabilities = lang.capabilities();
                    let form = method.form();
                    if !capabilities.function_validation_is_permissive()
                        && !matches!(self.kind, TypeKind::Interface | TypeKind::Trait)
                        && !self.modifiers.is_abstract
                        && method.modifiers().is_abstract
                        && lang.abstract_modifier_capability() == FunctionCapability::AbstractMethod
                        && capabilities.supports_function_capability(
                            crate::lang::capability::FunctionContext::Member,
                            form,
                            FunctionCapability::AbstractMethod,
                        )
                    {
                        errors.push(SigilStitchError::AbstractMethodInConcreteType {
                            language: lang.file_extension().to_string(),
                            type_name: self.name.clone(),
                            function_name: method.name().to_string(),
                        });
                    }
                }
                Err(error) => errors.push(error),
            }
        }

        let members = self.type_members_intent();
        members.collect_intrinsic_validation_errors(errors);
        lang.collect_type_members_validation_errors(members, errors);
    }

    pub(crate) fn validate_complete<'a>(
        &'a self,
        lang: &dyn CodeLang,
    ) -> Result<ValidatedType<'a>, SigilStitchError> {
        let mut errors = Vec::new();
        self.collect_validation_errors(lang, &mut errors);
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }

        let fields = (!self.fields.is_empty())
            .then(|| FieldSpec::validate_sequence(self.field_intent(), lang))
            .transpose()?;
        let properties = self
            .properties
            .iter()
            .map(|property| PropertySpec::validate_intent(self.property_intent(property), lang))
            .collect::<Result<Vec<_>, _>>()?;
        let declaration_context = lang.type_member_declaration_context(self.kind);
        let methods = self
            .methods
            .iter()
            .map(|method| method.validate_in_type(lang, declaration_context, &self.name))
            .collect::<Result<Vec<_>, _>>()?;
        let has_non_variant_members = !self.fields.is_empty()
            || !self.properties.is_empty()
            || !self.methods.is_empty()
            || !self.embedded_types.is_empty()
            || !self.extra_members.is_empty();
        let variants = (self.closed_sum || !self.variants.is_empty())
            .then(|| {
                EnumVariantSpec::validate_sequence(
                    &self.name,
                    self.kind,
                    self.closed_sum,
                    &self.variants,
                    self.variant_owner_context(lang, has_non_variant_members),
                    lang,
                )
            })
            .transpose()?;

        Ok(ValidatedType::new(
            TypeIntent::new(self),
            fields,
            properties,
            methods,
            variants,
        ))
    }

    /// Structured constructor arities that enum-entry lowering may rely on.
    fn variant_constructor_arities(&self, lang: &dyn CodeLang) -> Vec<ConstructorArity> {
        let mut arities = Vec::new();
        if !self.primary_constructor.is_empty() {
            arities.push(ConstructorArity::from_parameters(&self.primary_constructor));
        }

        let declaration_context = lang.type_member_declaration_context(self.kind);
        for method in &self.methods {
            if method
                .intent_in_type(lang, declaration_context, &self.name)
                .is_ok_and(|intent| intent.form() == FunctionForm::Constructor)
            {
                arities.push(ConstructorArity::from_parameters(&method.params));
            }
        }
        arities
    }

    fn variant_owner_context(
        &self,
        lang: &dyn CodeLang,
        has_non_variant_members: bool,
    ) -> VariantOwnerContext {
        VariantOwnerContext::new(
            has_non_variant_members,
            self.variant_constructor_arities(lang),
            !self.extra_members.is_empty(),
        )
    }

    fn field_intent(&self) -> FieldSequenceIntent<'_> {
        FieldSequenceIntent::type_members(&self.fields, &self.name, self.kind)
    }

    fn property_intent<'a>(&'a self, property: &'a PropertySpec) -> PropertyIntent<'a> {
        PropertyIntent::type_member(property, &self.name, self.kind)
    }

    fn type_members_intent(&self) -> TypeMembersIntent<'_> {
        TypeMembersIntent::new(
            &self.name,
            self.kind,
            &self.fields,
            &self.properties,
            &self.methods,
        )
    }

    fn collect_intrinsic_type_validation_errors(&self, errors: &mut Vec<SigilStitchError>) {
        if self.name.is_empty() {
            errors.push(SigilStitchError::EmptyName {
                builder: "TypeSpec",
            });
        }

        let mut invalid_modifiers = Vec::new();
        if self.modifiers.is_static {
            invalid_modifiers.push("static");
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
            errors.push(SigilStitchError::InvalidTypeModifiers {
                type_name: self.name.clone(),
                modifiers: invalid_modifiers,
            });
        }

        if self.closed_sum && self.kind != TypeKind::Enum {
            errors.push(SigilStitchError::InvalidTypeDeclaration {
                type_name: self.name.clone(),
                reason: "closed-sum intent requires the enum declaration carrier".to_string(),
            });
        }
        if self.closed_sum {
            if !self.extra_members.is_empty() {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: self.name.clone(),
                    reason: "closed-sum declarations must not contain opaque members that could add unvalidated cases"
                        .to_string(),
                });
            }
            for variant in &self.variants {
                if variant.legacy_value().is_some() {
                    errors.push(SigilStitchError::InvalidTypeDeclaration {
                        type_name: self.name.clone(),
                        reason: format!(
                            "closed-sum case {:?} must not declare a legacy value",
                            variant.name()
                        ),
                    });
                }
                if variant.discriminant().is_some() {
                    errors.push(SigilStitchError::InvalidTypeDeclaration {
                        type_name: self.name.clone(),
                        reason: format!(
                            "closed-sum case {:?} must not declare a discriminant",
                            variant.name()
                        ),
                    });
                }
                if !variant.constructor_arguments().is_empty() {
                    errors.push(SigilStitchError::InvalidTypeDeclaration {
                        type_name: self.name.clone(),
                        reason: format!(
                            "closed-sum case {:?} must not declare enum constructor arguments",
                            variant.name()
                        ),
                    });
                }
            }
        }

        for (index, annotation) in self.annotations.iter().enumerate() {
            if annotation.is_empty() {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: self.name.clone(),
                    reason: format!("opaque annotation {index} is empty"),
                });
            }
        }
        for (index, annotation) in self.annotation_specs.iter().enumerate() {
            let name_is_empty = match annotation.name() {
                AnnotationNameRef::Simple(name) => name.is_empty(),
                AnnotationNameRef::Importable(type_name) => type_name.is_empty(),
            };
            if name_is_empty {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: self.name.clone(),
                    reason: format!("structured annotation {index} has an empty name"),
                });
            }
        }
        for (index, member) in self.extra_members.iter().enumerate() {
            if member.is_empty() {
                errors.push(SigilStitchError::InvalidTypeDeclaration {
                    type_name: self.name.clone(),
                    reason: format!("opaque extra member {index} is empty"),
                });
            }
        }

        let mut seen_type_params = std::collections::HashSet::new();
        let mut reported_type_params = std::collections::HashSet::new();
        for parameter in &self.type_params {
            if parameter.name().is_empty() {
                errors.push(SigilStitchError::InvalidTypeParameter {
                    type_name: self.name.clone(),
                    parameter_name: String::new(),
                    reason: "parameter name is empty".to_string(),
                });
            }
            if !seen_type_params.insert(parameter.name())
                && reported_type_params.insert(parameter.name())
            {
                errors.push(SigilStitchError::DuplicateTypeParameterName {
                    type_name: self.name.clone(),
                    parameter_name: parameter.name().to_string(),
                });
            }
            if parameter.bounds().iter().any(TypeName::is_empty)
                || parameter.context_bounds().iter().any(TypeName::is_empty)
            {
                errors.push(SigilStitchError::InvalidTypeParameter {
                    type_name: self.name.clone(),
                    parameter_name: parameter.name().to_string(),
                    reason: "bounds must not contain an empty type".to_string(),
                });
            }
        }
        for constraint in &self.where_constraints {
            if constraint.subject().is_empty() || constraint.bounds().is_empty() {
                errors.push(SigilStitchError::InvalidTypeParameter {
                    type_name: self.name.clone(),
                    parameter_name: format!("{:?}", constraint.subject()),
                    reason: "where constraints require a non-empty subject and at least one bound"
                        .to_string(),
                });
            } else if constraint.bounds().iter().any(TypeName::is_empty) {
                errors.push(SigilStitchError::InvalidTypeParameter {
                    type_name: self.name.clone(),
                    parameter_name: format!("{:?}", constraint.subject()),
                    reason: "where-constraint bounds must not contain an empty type".to_string(),
                });
            }
        }

        if matches!(self.kind, TypeKind::TypeAlias | TypeKind::Newtype) {
            let kind = if self.kind == TypeKind::TypeAlias {
                "TypeAlias"
            } else {
                "Newtype"
            };
            if self.super_types.len() != 1
                || self.super_types.first().is_some_and(TypeName::is_empty)
            {
                errors.push(SigilStitchError::InvalidTypeAlias {
                    kind,
                    type_name: self.name.clone(),
                    reason: format!(
                        "expected exactly one non-empty target type, got {}",
                        self.super_types.len()
                    ),
                });
            }

            let mut forbidden = Vec::new();
            if !self.embedded_types.is_empty() {
                forbidden.push("embedded types");
            }
            if !self.fields.is_empty() {
                forbidden.push("fields");
            }
            if !self.properties.is_empty() {
                forbidden.push("properties");
            }
            if !self.methods.is_empty() {
                forbidden.push("methods");
            }
            if !self.variants.is_empty() {
                forbidden.push("variants");
            }
            if !self.primary_constructor.is_empty() {
                forbidden.push("primary-constructor parameters");
            }
            if !self.extra_members.is_empty() {
                forbidden.push("opaque members");
            }
            if self.kind == TypeKind::TypeAlias && !self.impl_types.is_empty() {
                forbidden.push("implemented contracts");
            }
            if !forbidden.is_empty() {
                errors.push(SigilStitchError::InvalidTypeAlias {
                    kind,
                    type_name: self.name.clone(),
                    reason: format!("must not declare {}", forbidden.join(", ")),
                });
            }
        }

        if !self.closed_sum && self.kind == TypeKind::Enum && !self.primary_constructor.is_empty() {
            let constructor_arity = ConstructorArity::from_parameters(&self.primary_constructor);
            for variant in &self.variants {
                let argument_count = if variant.constructor_arguments().is_empty() {
                    usize::from(variant.legacy_value().is_some())
                } else {
                    variant.constructor_arguments().len()
                };
                if !constructor_arity.accepts(argument_count) {
                    errors.push(SigilStitchError::InvalidEnum {
                        type_name: self.name.clone(),
                        reason: format!(
                            "variant {:?} has a constructor-argument count incompatible with the primary constructor",
                            variant.name()
                        ),
                    });
                }
            }
        }
    }

    fn collect_type_capability_errors(
        &self,
        lang: &dyn CodeLang,
        errors: &mut Vec<SigilStitchError>,
    ) {
        let capabilities = lang.capabilities();
        let language = lang.file_extension().to_string();

        if !capabilities.supports_type_kind(self.kind) {
            if self.closed_sum {
                errors.push(SigilStitchError::UnsupportedTypeCapabilities {
                    language,
                    type_name: self.name.clone(),
                    capabilities: vec![TypeCapability::ClosedSum],
                });
            } else {
                errors.push(SigilStitchError::UnsupportedTypeKind {
                    language,
                    kind: self.kind,
                    type_name: self.name.clone(),
                });
            }
            return;
        }

        let mut missing = Vec::new();
        let require = |capability: TypeCapability, condition: bool, missing: &mut Vec<_>| {
            if condition && !capabilities.supports_type_capability(self.kind, capability) {
                missing.push(capability);
            }
        };

        require(
            TypeCapability::RecordFields,
            !self.fields.is_empty(),
            &mut missing,
        );
        require(
            TypeCapability::AccessorMethods,
            !self.properties.is_empty(),
            &mut missing,
        );
        require(
            TypeCapability::Methods,
            !self.methods.is_empty(),
            &mut missing,
        );
        require(
            TypeCapability::StructuralEmbedding,
            !self.embedded_types.is_empty(),
            &mut missing,
        );
        require(
            TypeCapability::NominalSubtyping,
            !matches!(self.kind, TypeKind::TypeAlias | TypeKind::Newtype)
                && !self.super_types.is_empty(),
            &mut missing,
        );
        require(
            TypeCapability::InterfaceImplementation,
            !self.impl_types.is_empty(),
            &mut missing,
        );
        require(
            TypeCapability::ParametricPolymorphism,
            !self.type_params.is_empty(),
            &mut missing,
        );
        require(
            TypeCapability::BoundedPolymorphism,
            !self.where_constraints.is_empty()
                || self
                    .type_params
                    .iter()
                    .any(|param| !param.bounds().is_empty() || !param.context_bounds().is_empty()),
            &mut missing,
        );
        require(
            TypeCapability::HigherKindedPolymorphism,
            self.type_params.iter().any(|param| param.kind().is_some()),
            &mut missing,
        );
        require(
            TypeCapability::PrimaryConstructorParameters,
            !self.primary_constructor.is_empty(),
            &mut missing,
        );
        require(
            TypeCapability::Variants,
            !self.closed_sum && !self.variants.is_empty(),
            &mut missing,
        );
        if self.closed_sum
            && (capabilities.type_validation_is_permissive()
                || !capabilities.supports_type_capability(self.kind, TypeCapability::ClosedSum))
        {
            missing.push(TypeCapability::ClosedSum);
        }
        require(
            TypeCapability::Attributes,
            !self.annotations.is_empty() || !self.annotation_specs.is_empty(),
            &mut missing,
        );
        if !missing.is_empty() {
            errors.push(SigilStitchError::UnsupportedTypeCapabilities {
                language,
                type_name: self.name.clone(),
                capabilities: missing,
            });
        }
    }

    /// Emit this type through the selected language's complete type lowerer.
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<Vec<CodeBlock>, crate::error::SigilStitchError> {
        let type_ = self.validate_complete(lang)?;
        let blocks = lang.lower_type(type_)?;
        if blocks.is_empty() || blocks.iter().any(CodeBlock::is_empty) {
            return Err(SigilStitchError::EmptyTypeLowering {
                language: lang.file_extension().to_string(),
                kind: self.kind,
                type_name: self.name.clone(),
            });
        }
        Ok(blocks)
    }
}

/// Builder for [`TypeSpec`].
#[derive(Debug)]
pub struct TypeSpecBuilder {
    name: String,
    kind: TypeKind,
    closed_sum: bool,
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
    /// Kotlin and Scala lower these parameters in the type header. Use
    /// [`ParameterSpecBuilder::is_property()`](crate::spec::parameter_spec::ParameterSpecBuilder::is_property)
    /// or
    /// [`ParameterSpecBuilder::is_mutable_property()`](crate::spec::parameter_spec::ParameterSpecBuilder::is_mutable_property)
    /// to request property promotion; the parameter name itself must contain
    /// only the identifier. Languages without primary constructors reject the
    /// capability instead of ignoring it.
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
    /// Basic alias and newtype target/member-shape errors are also rejected
    /// eagerly; complete intrinsic and target validation runs at emission.
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

        // Validate enum consistency between primary-constructor parameters and
        // enum-entry constructor arguments.
        if !self.closed_sum && self.kind == TypeKind::Enum && !self.primary_constructor.is_empty() {
            let constructor_arity = ConstructorArity::from_parameters(&self.primary_constructor);
            if let Some(variant) = self.variants.iter().find(|variant| {
                let argument_count = if variant.constructor_arguments.is_empty() {
                    usize::from(variant.value.is_some())
                } else {
                    variant.constructor_arguments.len()
                };
                !constructor_arity.accepts(argument_count)
            }) {
                return Err(crate::error::SigilStitchError::InvalidEnum {
                    type_name: self.name.clone(),
                    reason: format!(
                        "variant {:?} has a constructor-argument count incompatible with the primary constructor",
                        variant.name
                    ),
                });
            }
        }

        Ok(TypeSpec {
            name: self.name,
            kind: self.kind,
            closed_sum: self.closed_sum,
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
    fn collect_validation_errors(&self, lang: &dyn CodeLang, errors: &mut Vec<SigilStitchError>) {
        TypeSpec::collect_validation_errors(self, lang, errors);
    }

    fn emit_members(&self, lang: &dyn CodeLang) -> Result<Vec<CodeBlock>, SigilStitchError> {
        self.emit(lang)
    }
}
