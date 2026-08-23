//! Function/method specification.

use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::capability::{
    FunctionBodyPolicy, FunctionCapability, FunctionContext, FunctionForm,
};
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::modifiers::{DeclarationContext, Modifiers, Visibility};
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

/// How function parameter lists are formatted.
#[deprecated(note = "legacy 0.6.8 function grammar; implement CodeLang::lower_function instead")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamListStyle {
    /// All params in a single `(name: T, name: T)` list (most languages).
    Tupled,
    /// Each param gets its own wrapper: `(name : T) (name : T)` (OCaml).
    Curried,
}

/// How function signatures are rendered.
#[deprecated(note = "legacy 0.6.8 function grammar; implement CodeLang::lower_function instead")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSignatureStyle {
    /// Single line: `fn add(x: Int, y: Int) -> Int {` (most languages).
    Merged,
    /// Separate type signature + definition (Haskell):
    /// ```text
    /// add :: Int -> Int -> Int
    /// add x y =
    /// ```
    Split,
}

/// A function or method specification.
///
/// `FunSpec` models a function declaration with parameters, return type, body,
/// modifiers (visibility, async, static, abstract, constructor), type parameters,
/// annotations, and doc comments. It emits a language-appropriate `CodeBlock` via
/// [`FunSpec::emit()`].
///
/// Use [`FunSpec::builder()`] to construct. Add to a [`FileSpec`](crate::spec::file_spec::FileSpec)
/// with `add_function()` or to a [`TypeSpec`](crate::spec::type_spec::TypeSpec)
/// with `add_method()`.
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let body = CodeBlock::of("return this.name", ()).unwrap();
///
/// let fun = FunSpec::builder("getName")
///     .returns(TypeName::primitive("string"))
///     .body(body)
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunSpec {
    pub(crate) name: String,
    pub(crate) params: Vec<ParameterSpec>,
    pub(crate) return_type: Option<TypeName>,
    pub(crate) body: Option<CodeBlock>,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) type_params: Vec<TypeParamSpec>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
    /// Receiver parameter (e.g., Go: `func (s *Server) Handle()`).
    pub(crate) receiver: Option<ParameterSpec>,
    /// Suffixes appended after the parameter list (e.g., C++: `const`, `override`, `= 0`).
    pub(crate) suffixes: Vec<String>,
    /// Constructor delegation call (e.g., `super(arg1, arg2)` or `this(arg1)`).
    ///
    /// For body-style languages (TS, Java, Swift): emitted as the first
    /// statement in the constructor body.
    /// For signature-style languages (Kotlin, Dart, C++): emitted after the parameter list
    /// as ` : super(...)` / ` : this(...)`.
    pub(crate) delegation: Option<CodeBlock>,
    /// Where-clause constraints (e.g., Rust `where T: Clone + Send`).
    #[serde(default)]
    pub(crate) where_constraints: Vec<WhereConstraint>,
}

/// Read-only semantic function intent passed to a language for validation.
///
/// Values of this type can only be constructed by `FunSpec` after declaration
/// context and form classification. Language adapters validate whether the
/// complete intent is representable without requiring generic specs to inspect
/// target syntax.
#[derive(Debug, Clone, Copy)]
pub struct FunctionIntent<'a> {
    pub(crate) spec: &'a FunSpec,
    declaration_context: DeclarationContext,
    function_context: FunctionContext,
    form: FunctionForm,
}

impl<'a> FunctionIntent<'a> {
    fn new(
        spec: &'a FunSpec,
        declaration_context: DeclarationContext,
        function_context: FunctionContext,
        form: FunctionForm,
    ) -> Self {
        Self {
            spec,
            declaration_context,
            function_context,
            form,
        }
    }

    /// Declaration name.
    pub fn name(self) -> &'a str {
        &self.spec.name
    }

    /// Declared parameters, excluding an optional receiver.
    pub fn parameters(self) -> &'a [ParameterSpec] {
        &self.spec.params
    }

    /// Explicit return type, when present.
    pub fn return_type(self) -> Option<&'a TypeName> {
        self.spec.return_type.as_ref()
    }

    /// Function body, when present.
    pub fn body(self) -> Option<&'a CodeBlock> {
        self.spec.body.as_ref()
    }

    /// Semantic declaration modifiers.
    pub fn modifiers(self) -> &'a Modifiers {
        &self.spec.modifiers
    }

    /// Documentation lines supplied by the caller.
    pub fn doc(self) -> &'a [String] {
        &self.spec.doc
    }

    /// Declared type parameters.
    pub fn type_params(self) -> &'a [TypeParamSpec] {
        &self.spec.type_params
    }

    /// Opaque annotation blocks supplied through the escape hatch.
    pub fn annotations(self) -> &'a [CodeBlock] {
        &self.spec.annotations
    }

    /// Structured annotation declarations.
    pub fn annotation_specs(self) -> &'a [AnnotationSpec] {
        &self.spec.annotation_specs
    }

    /// Explicit receiver, when the declaration uses receiver-method syntax.
    pub fn receiver(self) -> Option<&'a ParameterSpec> {
        self.spec.receiver.as_ref()
    }

    /// Opaque suffix escape hatches supplied by the caller.
    pub fn suffixes(self) -> &'a [String] {
        &self.spec.suffixes
    }

    /// Constructor delegation expression, when present.
    pub fn delegation(self) -> Option<&'a CodeBlock> {
        self.spec.delegation.as_ref()
    }

    /// Semantic where constraints.
    pub fn where_constraints(self) -> &'a [WhereConstraint] {
        &self.spec.where_constraints
    }

    /// Declaration location selected by the owning spec.
    pub fn declaration_context(self) -> DeclarationContext {
        self.declaration_context
    }

    /// Capability-validation context selected for this declaration.
    pub fn function_context(self) -> FunctionContext {
        self.function_context
    }

    /// Validated semantic function form.
    pub fn form(self) -> FunctionForm {
        self.form
    }
}

/// Function intent whose intrinsic and target-specific validation succeeded.
///
/// Only sigil-stitch constructs this wrapper. Language lowerers therefore
/// receive the complete declaration through a valid-by-construction interface.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedFunction<'a> {
    intent: FunctionIntent<'a>,
}

impl<'a> ValidatedFunction<'a> {
    pub(crate) fn new(intent: FunctionIntent<'a>) -> Self {
        Self { intent }
    }
}

impl<'a> std::ops::Deref for ValidatedFunction<'a> {
    type Target = FunctionIntent<'a>;

    fn deref(&self) -> &Self::Target {
        &self.intent
    }
}

impl FunSpec {
    /// Create a new builder for a function with the given name.
    pub fn builder(name: &str) -> FunSpecBuilder {
        FunSpecBuilder {
            name: name.to_string(),
            params: Vec::new(),
            return_type: None,
            body: None,
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            type_params: Vec::new(),
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            receiver: None,
            suffixes: Vec::new(),
            delegation: None,
            where_constraints: Vec::new(),
        }
    }

    /// Return the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Validate this function against the language function-capability matrix.
    pub fn validate(
        &self,
        lang: &dyn CodeLang,
        declaration_context: DeclarationContext,
    ) -> Result<(), SigilStitchError> {
        self.validate_with_legacy_constructor(lang, declaration_context, true)
    }

    pub(crate) fn validate_in_type(
        &self,
        lang: &dyn CodeLang,
        declaration_context: DeclarationContext,
    ) -> Result<(), SigilStitchError> {
        self.validate_with_legacy_constructor(lang, declaration_context, false)
    }

    fn validate_with_legacy_constructor(
        &self,
        lang: &dyn CodeLang,
        declaration_context: DeclarationContext,
        allow_legacy_constructor: bool,
    ) -> Result<(), SigilStitchError> {
        if allow_legacy_constructor
            && (self.is_implicit_direct_constructor(lang, declaration_context)
                || self.is_legacy_direct_constructor(lang, declaration_context))
        {
            let mut constructor = self.clone();
            constructor.modifiers.is_constructor = true;
            return constructor
                .validate_classified(lang, declaration_context)
                .map(|_| ());
        }
        self.validate_classified(lang, declaration_context)
            .map(|_| ())
    }

    fn validate_classified(
        &self,
        lang: &dyn CodeLang,
        declaration_context: DeclarationContext,
    ) -> Result<ValidatedFunction<'_>, SigilStitchError> {
        let intent = self.classify_intent(lang, declaration_context)?;
        self.validate_intent_default(lang, intent)?;
        lang.validate_function(intent)?;
        Ok(ValidatedFunction::new(intent))
    }

    fn classify_intent(
        &self,
        lang: &dyn CodeLang,
        declaration_context: DeclarationContext,
    ) -> Result<FunctionIntent<'_>, SigilStitchError> {
        let capabilities = lang.capabilities();
        let permissive_validation = capabilities.function_validation_is_permissive();

        let context = match (declaration_context, self.receiver.is_some()) {
            (DeclarationContext::TopLevel, false) => FunctionContext::TopLevel,
            (DeclarationContext::TopLevel, true) => FunctionContext::ReceiverMethod,
            (DeclarationContext::Member, _) if permissive_validation => FunctionContext::Member,
            (DeclarationContext::InterfaceMember, _) if permissive_validation => {
                FunctionContext::InterfaceMember
            }
            (DeclarationContext::Member, false) => FunctionContext::Member,
            (DeclarationContext::InterfaceMember, false) => FunctionContext::InterfaceMember,
            (DeclarationContext::Member | DeclarationContext::InterfaceMember, true) => {
                return Err(SigilStitchError::InvalidFunctionPlacement {
                    function_name: self.name.clone(),
                    context: declaration_context,
                });
            }
        };

        Ok(FunctionIntent::new(
            self,
            declaration_context,
            context,
            lang.function_form(&self.name, self.modifiers.is_constructor),
        ))
    }

    pub(crate) fn validate_intent_default<L: CodeLang + ?Sized>(
        &self,
        lang: &L,
        intent: FunctionIntent<'_>,
    ) -> Result<(), SigilStitchError> {
        let capabilities = lang.capabilities();
        let permissive_validation = capabilities.function_validation_is_permissive();
        let context = intent.function_context();
        let form = intent.form();
        if !permissive_validation
            && context == FunctionContext::ReceiverMethod
            && let Some(receiver) = &self.receiver
        {
            let mut invalid = Vec::new();
            if receiver.param_type.is_empty() {
                invalid.push(FunctionCapability::TypedParameters);
            }
            if receiver.default_value.is_some() {
                invalid.push(FunctionCapability::DefaultParameters);
            }
            if receiver.is_variadic {
                invalid.push(FunctionCapability::VariadicParameters);
            }
            if receiver.is_property || receiver.is_mutable_property {
                invalid.push(FunctionCapability::ConstructorProperties);
            }
            if !invalid.is_empty() {
                return Err(SigilStitchError::InvalidReceiverCapabilities {
                    function_name: self.name.clone(),
                    receiver_name: receiver.name.clone(),
                    capabilities: invalid,
                });
            }
        }
        let language = lang.file_extension().to_string();

        if !permissive_validation
            && form == FunctionForm::Constructor
            && !lang.constructor_name_is_valid(&self.name, None)
        {
            return Err(SigilStitchError::InvalidConstructorName {
                language: language.clone(),
                type_name: None,
                constructor_name: self.name.clone(),
            });
        }

        if !capabilities.supports_function_context(context) {
            return Err(SigilStitchError::UnsupportedFunctionContext {
                language: language.clone(),
                function_name: self.name.clone(),
                context,
            });
        }

        if !capabilities.supports_function_form(context, form) {
            return Err(SigilStitchError::UnsupportedFunctionForm {
                language: language.clone(),
                function_name: self.name.clone(),
                context,
                form,
            });
        }

        if !permissive_validation
            && !lang.function_visibility_is_valid(
                context,
                form,
                self.modifiers.is_static,
                self.modifiers.visibility,
            )
        {
            return Err(SigilStitchError::InvalidFunctionVisibility {
                language: language.clone(),
                function_name: self.name.clone(),
                context,
                form,
                visibility: self.modifiers.visibility,
            });
        }

        if !permissive_validation
            && let Some(maximum) =
                lang.maximum_function_parameters(context, form, self.modifiers.is_static)
            && self.params.len() > maximum
        {
            return Err(SigilStitchError::TooManyFunctionParameters {
                language: language.clone(),
                function_name: self.name.clone(),
                context,
                form,
                maximum,
                actual: self.params.len(),
            });
        }

        if !permissive_validation && !self.modifiers.is_constructor {
            let mut invalid = Vec::new();
            if self.delegation.is_some() {
                invalid.push(FunctionCapability::ConstructorDelegation);
            }
            if self
                .params
                .iter()
                .any(|param| param.is_property || param.is_mutable_property)
            {
                invalid.push(FunctionCapability::ConstructorProperties);
            }
            if !invalid.is_empty() {
                return Err(SigilStitchError::InvalidConstructorFeaturePlacement {
                    function_name: self.name.clone(),
                    capabilities: invalid,
                });
            }
        }

        if !permissive_validation
            && form == FunctionForm::Constructor
            && let Some(parameter) = self
                .params
                .iter()
                .find(|param| param.is_property && param.is_mutable_property)
        {
            return Err(SigilStitchError::ConflictingConstructorPropertyMutability {
                function_name: self.name.clone(),
                parameter_name: parameter.name.clone(),
            });
        }

        if !permissive_validation
            && let Some(parameter) = self
                .params
                .iter()
                .find(|param| param.is_variadic && param.default_value.is_some())
        {
            return Err(SigilStitchError::IncompatibleParameterCapabilities {
                function_name: self.name.clone(),
                parameter_name: parameter.name.clone(),
                capabilities: vec![
                    FunctionCapability::VariadicParameters,
                    FunctionCapability::DefaultParameters,
                ],
            });
        }

        if !permissive_validation
            && let Some(parameter) = self
                .params
                .iter()
                .find(|param| param.is_variadic && (param.is_property || param.is_mutable_property))
        {
            return Err(SigilStitchError::IncompatibleParameterCapabilities {
                function_name: self.name.clone(),
                parameter_name: parameter.name.clone(),
                capabilities: vec![
                    FunctionCapability::VariadicParameters,
                    FunctionCapability::ConstructorProperties,
                ],
            });
        }

        if !permissive_validation {
            let variadic_parameters: Vec<_> = self
                .params
                .iter()
                .enumerate()
                .filter(|(_, parameter)| parameter.is_variadic)
                .collect();
            if variadic_parameters.len() > 1 {
                return Err(SigilStitchError::MultipleVariadicParameters {
                    function_name: self.name.clone(),
                });
            }
            if let Some((index, parameter)) = variadic_parameters.first()
                && *index + 1 != self.params.len()
            {
                return Err(SigilStitchError::VariadicParameterNotLast {
                    function_name: self.name.clone(),
                    parameter_name: parameter.name.clone(),
                });
            }
        }

        if !permissive_validation
            && lang.function_parameters_require_trailing_defaults(context, form)
        {
            let mut saw_default = false;
            for parameter in &self.params {
                if parameter.default_value.is_some() {
                    saw_default = true;
                } else if saw_default {
                    return Err(SigilStitchError::RequiredParameterAfterDefault {
                        function_name: self.name.clone(),
                        parameter_name: parameter.name.clone(),
                    });
                }
            }
        }

        let abstract_capability = lang.abstract_modifier_capability();
        if self.modifiers.is_abstract
            && abstract_capability == FunctionCapability::AbstractMethod
            && self.body.is_some()
            && !permissive_validation
        {
            return Err(SigilStitchError::AbstractFunctionWithBody {
                function_name: self.name.clone(),
            });
        }

        let mut requested = Vec::new();
        let mut request = |capability: FunctionCapability, condition: bool| {
            if condition && !requested.contains(&capability) {
                requested.push(capability);
            }
        };

        request(
            FunctionCapability::ParametricPolymorphism,
            !self.type_params.is_empty(),
        );
        request(
            FunctionCapability::BoundedPolymorphism,
            !self.where_constraints.is_empty()
                || self
                    .type_params
                    .iter()
                    .any(|param| !param.bounds.is_empty() || !param.context_bounds.is_empty()),
        );
        request(
            FunctionCapability::Attributes,
            !self.annotations.is_empty() || !self.annotation_specs.is_empty(),
        );
        request(
            FunctionCapability::ExplicitReturnType,
            self.return_type.is_some(),
        );
        request(
            FunctionCapability::TypedParameters,
            self.params.iter().any(|param| !param.param_type.is_empty())
                || self
                    .receiver
                    .as_ref()
                    .is_some_and(|receiver| !receiver.param_type.is_empty()),
        );
        request(FunctionCapability::AsyncEffect, self.modifiers.is_async);
        request(
            if self.modifiers.is_constructor {
                FunctionCapability::StaticConstructor
            } else {
                match context {
                    FunctionContext::TopLevel => FunctionCapability::StaticFunction,
                    FunctionContext::ReceiverMethod
                    | FunctionContext::Member
                    | FunctionContext::InterfaceMember => FunctionCapability::StaticMethod,
                }
            },
            self.modifiers.is_static,
        );
        request(abstract_capability, self.modifiers.is_abstract);
        request(FunctionCapability::Override, self.modifiers.is_override);
        request(
            FunctionCapability::ConstructorDelegation,
            self.delegation.is_some(),
        );
        request(
            FunctionCapability::DefaultParameters,
            self.params
                .iter()
                .any(|param| param.default_value.is_some()),
        );
        request(
            FunctionCapability::VariadicParameters,
            self.params.iter().any(|param| param.is_variadic),
        );
        request(
            FunctionCapability::ConstructorProperties,
            self.params
                .iter()
                .any(|param| param.is_property || param.is_mutable_property),
        );

        let missing: Vec<_> = requested
            .iter()
            .copied()
            .filter(|capability| {
                !capabilities.supports_function_capability(context, form, *capability)
            })
            .collect();
        if !missing.is_empty() {
            return Err(SigilStitchError::UnsupportedFunctionCapabilities {
                language: language.clone(),
                function_name: self.name.clone(),
                context,
                form,
                capabilities: missing,
            });
        }

        if let Some((first, second)) =
            capabilities.first_incompatible_function_capabilities(context, form, &requested)
        {
            return Err(SigilStitchError::IncompatibleFunctionCapabilities {
                language: language.clone(),
                function_name: self.name.clone(),
                context,
                form,
                capabilities: vec![first, second],
            });
        }

        let mut missing_required: Vec<_> = capabilities
            .required_function_capabilities(context, form)
            .iter()
            .copied()
            .filter(|capability| match capability {
                FunctionCapability::ExplicitReturnType => self.return_type.is_none(),
                FunctionCapability::TypedParameters => {
                    !lang.function_parameters_are_typed(&self.params, context, form)
                }
                capability => !requested.contains(capability),
            })
            .collect();

        if !permissive_validation && lang.requires_complete_function_type_information(context, form)
        {
            let has_signature_metadata = self
                .params
                .iter()
                .any(|parameter| !parameter.param_type.is_empty())
                || !self.type_params.is_empty()
                || !self.where_constraints.is_empty();

            if self.return_type.is_none() && has_signature_metadata {
                if !missing_required.contains(&FunctionCapability::ExplicitReturnType) {
                    missing_required.push(FunctionCapability::ExplicitReturnType);
                }
            } else if self.return_type.is_some()
                && !lang.function_parameters_are_typed(&self.params, context, form)
                && !missing_required.contains(&FunctionCapability::TypedParameters)
            {
                missing_required.push(FunctionCapability::TypedParameters);
            }
        }

        if !missing_required.is_empty() {
            return Err(SigilStitchError::MissingRequiredFunctionCapabilities {
                language: language.clone(),
                function_name: self.name.clone(),
                context,
                form,
                capabilities: missing_required,
            });
        }

        if !permissive_validation
            && self.modifiers.is_constructor
            && let Some(return_type) = &self.return_type
            && !lang.constructor_return_type_is_valid(return_type)
        {
            return Err(SigilStitchError::InvalidConstructorReturnType {
                language,
                function_name: self.name.clone(),
                return_type: format!("{return_type:?}"),
            });
        }

        if !permissive_validation {
            lang.validate_function_type_constraints(
                &self.name,
                &self.type_params,
                &self.where_constraints,
            )?;
            match lang.function_body_policy(context, form, self.modifiers.is_static) {
                FunctionBodyPolicy::Optional => {}
                FunctionBodyPolicy::Required
                    if self.body.is_none() && !self.modifiers.is_abstract =>
                {
                    return Err(SigilStitchError::FunctionBodyRequired {
                        language: language.clone(),
                        function_name: self.name.clone(),
                        context,
                        form,
                    });
                }
                FunctionBodyPolicy::Forbidden if self.body.is_some() => {
                    return Err(SigilStitchError::FunctionBodyForbidden {
                        language: language.clone(),
                        function_name: self.name.clone(),
                        context,
                        form,
                    });
                }
                FunctionBodyPolicy::Required | FunctionBodyPolicy::Forbidden => {}
            }
        }
        Ok(())
    }

    fn is_legacy_direct_constructor(
        &self,
        lang: &dyn CodeLang,
        declaration_context: DeclarationContext,
    ) -> bool {
        let capabilities = lang.capabilities();
        declaration_context == DeclarationContext::Member
            && !capabilities.function_validation_is_permissive()
            && !self.modifiers.is_constructor
            && self.return_type.is_none()
            && lang.function_form(&self.name, false) == FunctionForm::Function
            && capabilities
                .supports_function_form(FunctionContext::Member, FunctionForm::Constructor)
            && capabilities
                .required_function_capabilities(FunctionContext::Member, FunctionForm::Function)
                .contains(&FunctionCapability::ExplicitReturnType)
            && !capabilities
                .required_function_capabilities(FunctionContext::Member, FunctionForm::Constructor)
                .contains(&FunctionCapability::ExplicitReturnType)
    }

    fn is_implicit_direct_constructor(
        &self,
        lang: &dyn CodeLang,
        declaration_context: DeclarationContext,
    ) -> bool {
        let context = match declaration_context {
            DeclarationContext::Member => FunctionContext::Member,
            DeclarationContext::InterfaceMember => FunctionContext::InterfaceMember,
            DeclarationContext::TopLevel => return false,
        };
        lang.capabilities()
            .supports_function_form(context, FunctionForm::Constructor)
            && !self.modifiers.is_constructor
            && if self.modifiers.is_static {
                lang.static_constructor_name_matches(&self.name, None)
            } else {
                lang.constructor_name_matches(&self.name, None)
            }
    }

    /// Emit this function as a CodeBlock.
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        self.emit_with_legacy_constructor(lang, ctx, true)
    }

    pub(crate) fn emit_in_type(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        self.emit_with_legacy_constructor(lang, ctx, false)
    }

    fn emit_with_legacy_constructor(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
        allow_legacy_constructor: bool,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        if allow_legacy_constructor
            && (self.is_implicit_direct_constructor(lang, ctx)
                || self.is_legacy_direct_constructor(lang, ctx))
        {
            let mut constructor = self.clone();
            constructor.modifiers.is_constructor = true;
            return constructor.emit_classified(lang, ctx);
        }
        self.emit_classified(lang, ctx)
    }

    fn emit_classified(
        &self,
        lang: &dyn CodeLang,
        declaration_context: DeclarationContext,
    ) -> Result<CodeBlock, SigilStitchError> {
        let function = self.validate_classified(lang, declaration_context)?;
        lang.lower_function(function)
    }
}
/// Builder for [`FunSpec`].
#[derive(Debug)]
pub struct FunSpecBuilder {
    name: String,
    params: Vec<ParameterSpec>,
    return_type: Option<TypeName>,
    body: Option<CodeBlock>,
    modifiers: Modifiers,
    doc: Vec<String>,
    type_params: Vec<TypeParamSpec>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    receiver: Option<ParameterSpec>,
    suffixes: Vec<String>,
    delegation: Option<CodeBlock>,
    where_constraints: Vec<WhereConstraint>,
}

impl FunSpecBuilder {
    /// Add a parameter to the function signature.
    pub fn add_param(mut self, param: ParameterSpec) -> Self {
        self.params.push(param);
        self
    }

    /// Set the return type.
    pub fn returns(mut self, ret: TypeName) -> Self {
        self.return_type = Some(ret);
        self
    }

    /// Set the function body.
    pub fn body(mut self, body: CodeBlock) -> Self {
        self.body = Some(body);
        self
    }

    /// Set the visibility modifier.
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this function as async.
    pub fn is_async(mut self) -> Self {
        self.modifiers.is_async = true;
        self
    }

    /// Mark this function as static.
    pub fn is_static(mut self) -> Self {
        self.modifiers.is_static = true;
        self
    }

    /// Mark this function as abstract.
    pub fn is_abstract(mut self) -> Self {
        self.modifiers.is_abstract = true;
        self
    }

    /// Mark this function as an override.
    pub fn is_override(mut self) -> Self {
        self.modifiers.is_override = true;
        self
    }

    /// Mark this function as a constructor.
    pub fn is_constructor(mut self) -> Self {
        self.modifiers.is_constructor = true;
        self
    }

    /// Add a documentation comment line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }

    /// Add a generic type parameter.
    pub fn add_type_param(mut self, tp: TypeParamSpec) -> Self {
        self.type_params.push(tp);
        self
    }

    /// Add a raw annotation CodeBlock.
    pub fn annotation(mut self, ann: CodeBlock) -> Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured annotation spec.
    pub fn annotate(mut self, spec: AnnotationSpec) -> Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Set the receiver parameter (e.g., Go's `(s *Server)`).
    pub fn receiver(mut self, recv: ParameterSpec) -> Self {
        self.receiver = Some(recv);
        self
    }

    /// Append a suffix after the parameter list (e.g., C++ `const`, `override`).
    pub fn suffix(mut self, s: &str) -> Self {
        self.suffixes.push(s.to_string());
        self
    }

    /// Set a constructor delegation call (e.g., `super(arg1, arg2)` or `this(arg1)`).
    ///
    /// For body-style languages (TS, JS, Java, Swift), this is emitted as
    /// the first statement in the constructor body.
    /// For signature-style languages (Kotlin, Dart, C++), this appears after the parameter
    /// list: `constructor(x: Int) : this(x, 0) { ... }`.
    pub fn delegation(mut self, call: CodeBlock) -> Self {
        self.delegation = Some(call);
        self
    }

    /// Add a where-clause constraint (e.g., `T: Clone + Send`).
    pub fn add_where_constraint(mut self, subject: TypeName, bounds: Vec<TypeName>) -> Self {
        self.where_constraints
            .push(WhereConstraint { subject, bounds });
        self
    }

    /// Convenience: add a single bound to an existing or new where constraint for
    /// the named type parameter.
    pub fn where_bound(mut self, param_name: &str, bound: TypeName) -> Self {
        if let Some(wc) = self
            .where_constraints
            .iter_mut()
            .find(|wc| wc.subject.simple_name() == Some(param_name))
        {
            wc.bounds.push(bound);
        } else {
            self.where_constraints.push(WhereConstraint {
                subject: TypeName::primitive(param_name),
                bounds: vec![bound],
            });
        }
        self
    }

    /// Consume the builder and produce a [`FunSpec`].
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    pub fn build(self) -> Result<FunSpec, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "FunSpecBuilder",
            }
        );
        Ok(FunSpec {
            name: self.name,
            params: self.params,
            return_type: self.return_type,
            body: self.body,
            modifiers: self.modifiers,
            doc: self.doc,
            type_params: self.type_params,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            receiver: self.receiver,
            suffixes: self.suffixes,
            delegation: self.delegation,
            where_constraints: self.where_constraints,
        })
    }
}

impl crate::spec::emittable::Emittable for FunSpec {
    fn collect_validation_errors(
        &self,
        lang: &dyn CodeLang,
        errors: &mut Vec<crate::error::SigilStitchError>,
    ) {
        if let Err(error) = self.validate(lang, DeclarationContext::TopLevel) {
            errors.push(error);
        }
    }

    fn emit_members(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<Vec<crate::code_block::CodeBlock>, crate::error::SigilStitchError> {
        Ok(vec![self.emit(lang, DeclarationContext::TopLevel)?])
    }
}
