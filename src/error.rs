//! Error types for sigil-stitch.

use snafu::prelude::*;

use crate::lang::capability::{
    FieldCapability, FieldContext, FunctionCapability, FunctionContext, FunctionForm,
    PropertyCapability, PropertyContext, TypeCapability, VariantCapability,
};
use crate::spec::modifiers::{DeclarationContext, TypeKind, Visibility};

/// Semantic source of a target-emitted type member name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeMemberNameOrigin {
    /// A stored field declaration.
    StoredField {
        /// Semantic field name supplied by the caller.
        field_name: String,
    },
    /// Read behavior lowered from a computed property.
    PropertyReadAccessor {
        /// The semantic property name.
        property_name: String,
    },
    /// Write behavior lowered from a computed property.
    PropertyWriteAccessor {
        /// The semantic property name.
        property_name: String,
    },
    /// A method declared explicitly on the owning type.
    ExplicitMethod {
        /// The semantic method name.
        method_name: String,
    },
}

impl std::fmt::Display for TypeMemberNameOrigin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoredField { field_name } => {
                write!(formatter, "stored field {field_name:?}")
            }
            Self::PropertyReadAccessor { property_name } => {
                write!(formatter, "read accessor of property {property_name:?}")
            }
            Self::PropertyWriteAccessor { property_name } => {
                write!(formatter, "write accessor of property {property_name:?}")
            }
            Self::ExplicitMethod { method_name } => {
                write!(formatter, "explicit method {method_name:?}")
            }
        }
    }
}

/// Errors returned by sigil-stitch operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum SigilStitchError {
    /// Format string argument count mismatch.
    #[snafu(display(
        "format string {format:?} expects {expected} args but got {actual}\n  \
         specifiers: {expected_specifiers:?}\n  \
         arg kinds:  {actual_arg_kinds:?}"
    ))]
    FormatArgCount {
        /// The format string that was passed.
        format: String,
        /// Number of argument slots in the format string.
        expected: usize,
        /// Number of arguments actually provided.
        actual: usize,
        /// The sequence of specifier names from the format string (e.g., `["%T", "%S", "%L"]`).
        expected_specifiers: Vec<String>,
        /// The variant names of the provided args (e.g., `["TypeName", "Literal", "Literal"]`).
        actual_arg_kinds: Vec<String>,
    },

    /// A required name or filename field was empty.
    #[snafu(display("{builder}::build() failed: 'name' must not be empty"))]
    EmptyName {
        /// The builder type that detected the error.
        builder: &'static str,
    },

    /// Unbalanced structural indentation markers.
    #[snafu(display(
        "unbalanced structural indentation: depth is {depth} (expected 0). \
         Check %> / %< markers and begin_control_flow / end_control_flow calls."
    ))]
    UnbalancedIndent {
        /// The structural indent depth at validation time.
        depth: i32,
    },

    /// A structural indentation marker reached output as raw literal text.
    #[snafu(display(
        "unresolved indentation marker '{marker}' in {context}. \
         Pass structured fragments as CodeBlock/CodeFragment instead of raw %L text."
    ))]
    UnresolvedIndentMarker {
        /// The unresolved marker, e.g. `%>` or `%<`.
        marker: String,
        /// Where the marker was found.
        context: String,
    },

    /// Error during code rendering.
    #[snafu(display("{context}: {message}"))]
    Render {
        /// What was being rendered.
        context: String,
        /// The error message.
        message: String,
    },

    /// Error in template parsing or application.
    #[snafu(display("template error: {message}"))]
    Template {
        /// The error message.
        message: String,
    },

    /// I/O error (e.g., writing project files).
    #[snafu(display("{context}"))]
    Io {
        /// The underlying I/O error.
        source: std::io::Error,
        /// What was being done when the error occurred.
        context: String,
    },

    /// Module path validation failure.
    #[snafu(display("invalid module path: {message}"))]
    InvalidModulePath {
        /// The error message.
        message: String,
    },

    /// Invalid format specifier in a format string.
    #[snafu(display("invalid format specifier '%{specifier}' in format string {format:?}"))]
    InvalidFormatSpecifier {
        /// The format string that contained the invalid specifier.
        format: String,
        /// The unrecognized character after `%`.
        specifier: char,
    },

    /// Duplicate field name in a type specification.
    #[snafu(display("duplicate field name {field_name:?} in type {type_name:?}"))]
    DuplicateFieldName {
        /// The name of the type that contains the duplicate.
        type_name: String,
        /// The duplicated field name.
        field_name: String,
    },

    /// Invalid TypeAlias or Newtype declaration.
    #[snafu(display("invalid {kind} {type_name:?}: {reason}"))]
    InvalidTypeAlias {
        /// The kind of declaration ("TypeAlias" or "Newtype").
        kind: &'static str,
        /// The type name.
        type_name: String,
        /// The reason the declaration is invalid.
        reason: String,
    },

    /// Duplicate filename in a project specification.
    #[snafu(display("duplicate filename {filename:?} in ProjectSpec (appears {count} times)"))]
    DuplicateFileName {
        /// The duplicated filename.
        filename: String,
        /// How many times it appeared.
        count: usize,
    },

    /// FileSpec has no language set (e.g. after deserialization).
    #[snafu(display(
        "FileSpec {filename:?} has no language — call .with_lang() after deserialization \
         or use FileSpec::builder_with() to set one"
    ))]
    MissingLang {
        /// The filename of the FileSpec.
        filename: String,
    },

    /// Invalid enum declaration.
    #[snafu(display("invalid enum {type_name:?}: {reason}"))]
    InvalidEnum {
        /// The type name.
        type_name: String,
        /// The reason the declaration is invalid.
        reason: String,
    },

    /// A format argument does not match the corresponding specifier.
    #[snafu(display(
        "format string {format:?} argument {index} expects {expected} but got {actual}"
    ))]
    FormatArgKind {
        /// The format string that was passed.
        format: String,
        /// Zero-based argument index.
        index: usize,
        /// The expected specifier and argument kind.
        expected: String,
        /// The provided argument variant.
        actual: String,
    },

    /// A format string ends with a bare `%` marker.
    #[snafu(display("trailing format marker '%' at byte {offset} in format string {format:?}"))]
    TrailingFormatMarker {
        /// The format string that contained the marker.
        format: String,
        /// Byte offset of the trailing `%`.
        offset: usize,
    },

    /// A language does not support the requested type declaration kind.
    #[snafu(display("language {language:?} does not support {kind:?} declaration {type_name:?}"))]
    UnsupportedTypeKind {
        /// The language file extension.
        language: String,
        /// The unsupported declaration kind.
        kind: TypeKind,
        /// The type being emitted.
        type_name: String,
    },

    /// A language does not support one or more semantic type capabilities.
    #[snafu(display(
        "language {language:?} does not support {capabilities:?} for type {type_name:?}"
    ))]
    UnsupportedTypeCapabilities {
        /// The language file extension.
        language: String,
        /// The type being emitted.
        type_name: String,
        /// The unsupported semantic capabilities.
        capabilities: Vec<TypeCapability>,
    },

    /// A strict adapter declared type support but omitted complete type lowering.
    #[snafu(display(
        "language {language:?} has no complete lowerer for {kind:?} type {type_name:?}"
    ))]
    MissingTypeLowerer {
        /// The language file extension.
        language: String,
        /// The declaration kind.
        kind: TypeKind,
        /// The type being emitted.
        type_name: String,
    },

    /// A complete type lowerer returned no declaration output.
    #[snafu(display(
        "language {language:?} returned empty output for {kind:?} type {type_name:?}"
    ))]
    EmptyTypeLowering {
        /// The language file extension.
        language: String,
        /// The declaration kind.
        kind: TypeKind,
        /// The type being emitted.
        type_name: String,
    },

    /// A type declaration used modifiers that have no type-level meaning.
    #[snafu(display("type {type_name:?} cannot use modifiers {modifiers:?}"))]
    InvalidTypeModifiers {
        /// The type being validated.
        type_name: String,
        /// Rejected modifier names.
        modifiers: Vec<&'static str>,
    },

    /// A type declaration contains malformed semantic input.
    #[snafu(display("invalid type {type_name:?}: {reason}"))]
    InvalidTypeDeclaration {
        /// The type being validated.
        type_name: String,
        /// Why the declaration is invalid.
        reason: String,
    },

    /// A type parameter name was declared more than once.
    #[snafu(display("duplicate type parameter name {parameter_name:?} in type {type_name:?}"))]
    DuplicateTypeParameterName {
        /// The owning type.
        type_name: String,
        /// The duplicated parameter name.
        parameter_name: String,
    },

    /// A type parameter or constraint is malformed.
    #[snafu(display("invalid type parameter {parameter_name:?} on type {type_name:?}: {reason}"))]
    InvalidTypeParameter {
        /// The owning type.
        type_name: String,
        /// The parameter or constraint subject.
        parameter_name: String,
        /// Why the declaration is invalid.
        reason: String,
    },

    /// A FileSpec contains one or more invalid spec members.
    ///
    /// Validation is collected rather than fail-fast: every invalid
    /// [`TypeSpec`](crate::spec::type_spec::TypeSpec) is checked and all
    /// resulting errors are returned together.
    #[snafu(display("FileSpec {filename:?} has {error_count} validation error(s): {errors:?}"))]
    FileSpecValidation {
        /// The filename of the invalid FileSpec.
        filename: String,
        /// The number of collected validation errors. Equal to `errors.len()`.
        error_count: usize,
        /// The collected member validation errors.
        errors: Vec<SigilStitchError>,
    },

    /// A type kind used an abstract modifier that its language does not permit.
    #[snafu(display(
        "language {language:?} does not allow an abstract modifier on {kind:?} type {type_name:?}"
    ))]
    InvalidAbstractType {
        /// The language file extension.
        language: String,
        /// The rejected type kind.
        kind: TypeKind,
        /// The type being emitted.
        type_name: String,
    },

    /// A receiver parameter was used outside a top-level receiver method.
    #[snafu(display(
        "function {function_name:?} cannot use a receiver in {context:?} declaration context"
    ))]
    InvalidFunctionPlacement {
        /// The function being emitted.
        function_name: String,
        /// The declaration context supplied by the caller.
        context: DeclarationContext,
    },

    /// A function form used a visibility that the language does not permit.
    #[snafu(display(
        "language {language:?} does not allow {visibility:?} visibility for {form:?} {function_name:?} in {context:?} context"
    ))]
    InvalidFunctionVisibility {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The semantic function context.
        context: FunctionContext,
        /// The declaration form being emitted.
        form: FunctionForm,
        /// The rejected visibility.
        visibility: Visibility,
    },

    /// A receiver used parameter-only features that receiver syntax cannot represent.
    #[snafu(display(
        "receiver {receiver_name:?} on function {function_name:?} cannot use {capabilities:?}"
    ))]
    InvalidReceiverCapabilities {
        /// The function being emitted.
        function_name: String,
        /// The receiver parameter.
        receiver_name: String,
        /// Parameter capabilities that cannot be represented on a receiver.
        capabilities: Vec<FunctionCapability>,
    },

    /// Constructor-only features were attached to a non-constructor function.
    #[snafu(display(
        "function {function_name:?} cannot use constructor-only features {capabilities:?} without being a constructor"
    ))]
    InvalidConstructorFeaturePlacement {
        /// The function being emitted.
        function_name: String,
        /// Constructor-only features used by the function.
        capabilities: Vec<FunctionCapability>,
    },

    /// An abstract function declaration included an implementation body.
    #[snafu(display("abstract function {function_name:?} cannot have a body"))]
    AbstractFunctionWithBody {
        /// The function being emitted.
        function_name: String,
    },

    /// A function constraint did not target a declared type parameter as required.
    #[snafu(display(
        "language {language:?} cannot lower constraint for {subject:?} on function {function_name:?}; the subject must name a declared type parameter"
    ))]
    InvalidFunctionConstraintSubject {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The constraint subject that could not be attached to a declared type parameter.
        subject: String,
    },

    /// A target language cannot represent a function type parameter.
    #[snafu(display(
        "language {language:?} rejects type parameter {parameter_name:?} on function {function_name:?}: {reason}"
    ))]
    InvalidFunctionTypeParameter {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The rejected parameter name.
        parameter_name: String,
        /// Why the parameter cannot be represented.
        reason: String,
    },

    /// A language does not support the requested function context.
    #[snafu(display(
        "language {language:?} does not support function {function_name:?} in {context:?} context"
    ))]
    UnsupportedFunctionContext {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The unsupported semantic function context.
        context: FunctionContext,
    },

    /// A language does not support the requested function declaration form.
    #[snafu(display(
        "language {language:?} does not support {form:?} {function_name:?} in {context:?} context"
    ))]
    UnsupportedFunctionForm {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The semantic function context.
        context: FunctionContext,
        /// The unsupported declaration form.
        form: FunctionForm,
    },

    /// A profile forbids a requested pair of otherwise supported capabilities.
    #[snafu(display(
        "language {language:?} does not allow {capabilities:?} together for {form:?} {function_name:?} in {context:?} context"
    ))]
    IncompatibleFunctionCapabilities {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The semantic function context.
        context: FunctionContext,
        /// The declaration form being emitted.
        form: FunctionForm,
        /// The incompatible pair.
        capabilities: Vec<FunctionCapability>,
    },

    /// A parameter combined features that cannot coexist on one parameter.
    #[snafu(display(
        "parameter {parameter_name:?} on function {function_name:?} cannot combine {capabilities:?}"
    ))]
    IncompatibleParameterCapabilities {
        /// The function being emitted.
        function_name: String,
        /// The parameter with the invalid combination.
        parameter_name: String,
        /// The incompatible parameter capabilities.
        capabilities: Vec<FunctionCapability>,
    },

    /// A function profile requires syntax that the declaration omitted.
    #[snafu(display(
        "language {language:?} requires {capabilities:?} for {form:?} {function_name:?} in {context:?} context"
    ))]
    MissingRequiredFunctionCapabilities {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The semantic function context.
        context: FunctionContext,
        /// The declaration form being emitted.
        form: FunctionForm,
        /// Required capabilities omitted by the declaration.
        capabilities: Vec<FunctionCapability>,
    },

    /// A concrete function declaration omitted its required body.
    #[snafu(display(
        "language {language:?} requires a body for {form:?} {function_name:?} in {context:?} context"
    ))]
    FunctionBodyRequired {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The semantic function context.
        context: FunctionContext,
        /// The declaration form being emitted.
        form: FunctionForm,
    },

    /// A function declaration supplied a body in a bodyless context.
    #[snafu(display(
        "language {language:?} forbids a body for {form:?} {function_name:?} in {context:?} context"
    ))]
    FunctionBodyForbidden {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The semantic function context.
        context: FunctionContext,
        /// The declaration form being emitted.
        form: FunctionForm,
    },

    /// A function declared more than one variadic/rest parameter.
    #[snafu(display("function {function_name:?} cannot declare more than one variadic parameter"))]
    MultipleVariadicParameters {
        /// The function being emitted.
        function_name: String,
    },

    /// A variadic/rest parameter was followed by another parameter.
    #[snafu(display(
        "variadic parameter {parameter_name:?} on function {function_name:?} must be last"
    ))]
    VariadicParameterNotLast {
        /// The function being emitted.
        function_name: String,
        /// The misplaced variadic parameter.
        parameter_name: String,
    },

    /// A required parameter followed a parameter with a default value.
    #[snafu(display(
        "required parameter {parameter_name:?} cannot follow a defaulted parameter on function {function_name:?}"
    ))]
    RequiredParameterAfterDefault {
        /// The function being emitted.
        function_name: String,
        /// The required parameter that appeared after a defaulted parameter.
        parameter_name: String,
    },

    /// A constructor return annotation violated a language-specific restriction.
    #[snafu(display(
        "language {language:?} does not allow return type {return_type:?} on constructor {function_name:?}"
    ))]
    InvalidConstructorReturnType {
        /// The language file extension.
        language: String,
        /// The constructor being emitted.
        function_name: String,
        /// The rejected return type.
        return_type: String,
    },

    /// A language does not support one or more function capabilities.
    #[snafu(display(
        "language {language:?} does not support {capabilities:?} for {form:?} {function_name:?} in {context:?} context"
    ))]
    UnsupportedFunctionCapabilities {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The semantic function context.
        context: FunctionContext,
        /// The declaration form being emitted.
        form: FunctionForm,
        /// The unsupported function capabilities.
        capabilities: Vec<FunctionCapability>,
    },

    /// A function form declared more parameters than its profile permits.
    #[snafu(display(
        "language {language:?} allows at most {maximum} parameter(s) for {form:?} {function_name:?} in {context:?} context, but got {actual}"
    ))]
    TooManyFunctionParameters {
        /// The language file extension.
        language: String,
        /// The function being emitted.
        function_name: String,
        /// The semantic function context.
        context: FunctionContext,
        /// The declaration form being emitted.
        form: FunctionForm,
        /// Maximum parameter count accepted by the profile.
        maximum: usize,
        /// Actual parameter count.
        actual: usize,
    },

    /// A destructor name did not match its declaring type.
    #[snafu(display(
        "language {language:?} requires destructor {destructor_name:?} to be named after declaring type {type_name:?}"
    ))]
    InvalidDestructorName {
        /// The language file extension.
        language: String,
        /// The declaring type.
        type_name: String,
        /// The rejected destructor name.
        destructor_name: String,
    },

    /// A constructor name did not match the language's naming rule.
    #[snafu(display("language {language:?} does not allow constructor name {constructor_name:?}"))]
    InvalidConstructorName {
        /// The language file extension.
        language: String,
        /// The declaring type, when validation has owner context.
        type_name: Option<String>,
        /// The rejected constructor name.
        constructor_name: String,
    },

    /// An abstract method was declared in a non-abstract concrete type.
    #[snafu(display(
        "language {language:?} requires type {type_name:?} to be abstract because method {function_name:?} is abstract"
    ))]
    AbstractMethodInConcreteType {
        /// The language file extension.
        language: String,
        /// The containing concrete type.
        type_name: String,
        /// The abstract method.
        function_name: String,
    },

    /// A constructor parameter was marked as both readonly and mutable.
    #[snafu(display(
        "constructor parameter {parameter_name:?} on function {function_name:?} cannot be both readonly and mutable"
    ))]
    ConflictingConstructorPropertyMutability {
        /// The constructor being emitted.
        function_name: String,
        /// The parameter with contradictory property markers.
        parameter_name: String,
    },

    /// A strict language needs an owning declaration to validate enum variants.
    #[snafu(display(
        "language {language:?} cannot emit variant {variant_name:?} without its owning type and complete variant sequence"
    ))]
    VariantOwnerRequired {
        /// The language file extension.
        language: String,
        /// The ownerless variant being emitted.
        variant_name: String,
    },

    /// A language does not support variant declarations for this owner kind.
    #[snafu(display(
        "language {language:?} does not support variants owned by {owner_kind:?} type {type_name:?}"
    ))]
    UnsupportedVariantOwner {
        /// The language file extension.
        language: String,
        /// The containing type name.
        type_name: String,
        /// The rejected owning type kind.
        owner_kind: TypeKind,
    },

    /// A language cannot represent one or more requested variant capabilities.
    #[snafu(display(
        "language {language:?} does not support {capabilities:?} for variant {variant_name:?} in {owner_kind:?} type {type_name:?}"
    ))]
    UnsupportedVariantCapabilities {
        /// The language file extension.
        language: String,
        /// The containing type name.
        type_name: String,
        /// The variant being emitted.
        variant_name: String,
        /// The containing type kind.
        owner_kind: TypeKind,
        /// Unsupported semantic capabilities.
        capabilities: Vec<VariantCapability>,
    },

    /// One variant combines mutually exclusive semantic forms.
    #[snafu(display("variant {variant_name:?} cannot combine semantic forms {capabilities:?}"))]
    IncompatibleVariantCapabilities {
        /// The invalid variant.
        variant_name: String,
        /// The mutually exclusive capabilities.
        capabilities: Vec<VariantCapability>,
    },

    /// A legacy `.value()` request has no validity-preserving interpretation.
    #[snafu(display(
        "language {language:?} cannot safely interpret legacy value on variant {variant_name:?}; use discriminant() or constructor_argument()"
    ))]
    UnsupportedLegacyVariantValue {
        /// The language file extension.
        language: String,
        /// The variant carrying the ambiguous value.
        variant_name: String,
    },

    /// A named variant payload field uses semantics the target cannot represent.
    #[snafu(display(
        "language {language:?} cannot represent field {field_name:?} on record payload variant {variant_name:?}: {reason}"
    ))]
    InvalidVariantRecordField {
        /// The language file extension.
        language: String,
        /// The variant carrying the field.
        variant_name: String,
        /// The rejected field.
        field_name: String,
        /// Target-local reason for rejection.
        reason: String,
    },

    /// An owner-aware variant sequence repeats one variant name.
    #[snafu(display("duplicate variant name {variant_name:?} in type {type_name:?}"))]
    DuplicateVariantName {
        /// The containing type.
        type_name: String,
        /// The duplicated variant name.
        variant_name: String,
    },

    /// One record-payload variant repeats a field name.
    #[snafu(display(
        "duplicate record-payload field name {field_name:?} in variant {variant_name:?}"
    ))]
    DuplicateVariantRecordFieldName {
        /// The variant carrying the duplicate field.
        variant_name: String,
        /// The duplicated field name.
        field_name: String,
    },

    /// An enum entry passes constructor arguments without a declared constructor.
    #[snafu(display(
        "language {language:?} cannot pass constructor arguments for variant {variant_name:?} in type {type_name:?} without a declared constructor or an opaque member that can provide one"
    ))]
    MissingVariantConstructor {
        /// The language file extension.
        language: String,
        /// The containing type.
        type_name: String,
        /// The variant carrying constructor arguments.
        variant_name: String,
    },

    /// An enum entry's argument count matches none of the declared constructors.
    #[snafu(display(
        "language {language:?} cannot pass {argument_count} constructor arguments for variant {variant_name:?} in type {type_name:?}; no declared constructor accepts that count"
    ))]
    IncompatibleVariantConstructorArguments {
        /// The language file extension.
        language: String,
        /// The containing type.
        type_name: String,
        /// The variant carrying constructor arguments.
        variant_name: String,
        /// Number of arguments supplied by the enum entry.
        argument_count: usize,
    },

    /// A variant operand was present but contained no semantic value.
    #[snafu(display("variant {variant_name:?} has an empty {operand}"))]
    EmptyVariantOperand {
        /// The invalid variant.
        variant_name: String,
        /// The empty operand and, where relevant, its index or field name.
        operand: String,
    },

    /// A target cannot preserve one variant annotation form's semantics.
    #[snafu(display(
        "language {language:?} cannot represent annotation metadata on variant {variant_name:?}: {reason}"
    ))]
    InvalidVariantAnnotation {
        /// The language file extension.
        language: String,
        /// The variant carrying the annotation.
        variant_name: String,
        /// Target-local reason for rejection.
        reason: String,
    },

    /// A strict language needs an owning declaration context for field lowering.
    #[snafu(display(
        "language {language:?} does not support field sequence context {context:?} for owner {owner_name:?}"
    ))]
    UnsupportedFieldContext {
        /// The language file extension.
        language: String,
        /// The rejected semantic field context.
        context: FieldContext,
        /// The owning type, when available.
        owner_name: Option<String>,
    },

    /// A field requested capabilities the selected context cannot represent.
    #[snafu(display(
        "language {language:?} does not support {capabilities:?} for field {field_name:?} in {context:?} context"
    ))]
    UnsupportedFieldCapabilities {
        /// The language file extension.
        language: String,
        /// The rejected field.
        field_name: String,
        /// The semantic field context.
        context: FieldContext,
        /// Unsupported semantic capabilities.
        capabilities: Vec<FieldCapability>,
    },

    /// A field omitted capabilities required by the selected context.
    #[snafu(display(
        "language {language:?} requires {capabilities:?} for field {field_name:?} in {context:?} context"
    ))]
    MissingRequiredFieldCapabilities {
        /// The language file extension.
        language: String,
        /// The rejected field.
        field_name: String,
        /// The semantic field context.
        context: FieldContext,
        /// Required semantic capabilities omitted by the field.
        capabilities: Vec<FieldCapability>,
    },

    /// Deserialized function-only modifiers were attached to a field.
    #[snafu(display(
        "field {field_name:?} in {context:?} context cannot use modifiers {modifiers:?}"
    ))]
    InvalidFieldModifiers {
        /// The rejected field.
        field_name: String,
        /// The semantic field context.
        context: FieldContext,
        /// Invalid function-only modifier names.
        modifiers: Vec<&'static str>,
    },

    /// A field operand was present but contained no semantic value.
    #[snafu(display("field {field_name:?} in {context:?} context has an empty {operand}"))]
    EmptyFieldOperand {
        /// The invalid field.
        field_name: String,
        /// The field-sequence context.
        context: FieldContext,
        /// The empty operand.
        operand: &'static str,
    },

    /// A target-local field rule rejected otherwise representable intent.
    #[snafu(display(
        "language {language:?} cannot represent field {field_name:?} in {context:?} context: {reason}"
    ))]
    InvalidField {
        /// The language file extension.
        language: String,
        /// The rejected field.
        field_name: String,
        /// The semantic field context.
        context: FieldContext,
        /// Target-local reason for rejection.
        reason: String,
    },

    /// A strict language cannot lower a property in this semantic context.
    #[snafu(display(
        "language {language:?} does not support property {property_name:?} in {context:?} context for owner {owner_name:?}"
    ))]
    UnsupportedPropertyContext {
        /// The language file extension.
        language: String,
        /// The rejected semantic property context.
        context: PropertyContext,
        /// The rejected property.
        property_name: String,
        /// The owning type, when available.
        owner_name: Option<String>,
    },

    /// A property requested capabilities the selected context cannot represent.
    #[snafu(display(
        "language {language:?} does not support {capabilities:?} for property {property_name:?} in {context:?} context"
    ))]
    UnsupportedPropertyCapabilities {
        /// The language file extension.
        language: String,
        /// The rejected property.
        property_name: String,
        /// The semantic property context.
        context: PropertyContext,
        /// Unsupported semantic capabilities.
        capabilities: Vec<PropertyCapability>,
    },

    /// A property omitted capabilities required by the selected context.
    #[snafu(display(
        "language {language:?} requires {capabilities:?} for property {property_name:?} in {context:?} context"
    ))]
    MissingRequiredPropertyCapabilities {
        /// The language file extension.
        language: String,
        /// The rejected property.
        property_name: String,
        /// The semantic property context.
        context: PropertyContext,
        /// Required semantic capabilities omitted by the property.
        capabilities: Vec<PropertyCapability>,
    },

    /// A computed property has neither read nor write behavior.
    #[snafu(display(
        "property {property_name:?} in {context:?} context must define a getter or setter"
    ))]
    MissingPropertyAccessors {
        /// The rejected property.
        property_name: String,
        /// The semantic property context.
        context: PropertyContext,
    },

    /// A property operand was present but contained no semantic value.
    #[snafu(display("property {property_name:?} in {context:?} context has an empty {operand}"))]
    EmptyPropertyOperand {
        /// The rejected property.
        property_name: String,
        /// The semantic property context.
        context: PropertyContext,
        /// The empty operand.
        operand: &'static str,
    },

    /// A setter does not bind the assigned value to a name.
    #[snafu(display(
        "property {property_name:?} in {context:?} context has an empty setter parameter name"
    ))]
    EmptyPropertySetterParameter {
        /// The rejected property.
        property_name: String,
        /// The semantic property context.
        context: PropertyContext,
    },

    /// Deserialized non-property modifiers were attached to a property.
    #[snafu(display(
        "property {property_name:?} in {context:?} context cannot use modifiers {modifiers:?}"
    ))]
    InvalidPropertyModifiers {
        /// The rejected property.
        property_name: String,
        /// The semantic property context.
        context: PropertyContext,
        /// Invalid modifier names.
        modifiers: Vec<&'static str>,
    },

    /// A target-local property rule rejected otherwise representable intent.
    #[snafu(display(
        "language {language:?} cannot represent property {property_name:?} in {context:?} context: {reason}"
    ))]
    InvalidProperty {
        /// The language file extension.
        language: String,
        /// The rejected property.
        property_name: String,
        /// The semantic property context.
        context: PropertyContext,
        /// Target-local reason for rejection.
        reason: String,
    },

    /// One type repeats an exact semantic property name.
    #[snafu(display("duplicate property name {property_name:?} in type {type_name:?}"))]
    DuplicatePropertyName {
        /// The containing type.
        type_name: String,
        /// The duplicated property name.
        property_name: String,
    },

    /// Target lowering maps two semantic members to one target member name.
    #[snafu(display(
        "language {language:?} maps {first_member} and {second_member} to the same member name {member_name:?} in type {type_name:?}"
    ))]
    TypeMemberNameCollision {
        /// The language file extension.
        language: String,
        /// The containing type.
        type_name: String,
        /// The conflicting target-emitted name.
        member_name: String,
        /// Structured origin of the first semantic member.
        first_member: Box<TypeMemberNameOrigin>,
        /// Structured origin of the second semantic member.
        second_member: Box<TypeMemberNameOrigin>,
    },

    /// Import claims contain an alias conflict with no valid assignment.
    #[snafu(display("import alias conflict for {requested_name:?}: {reason}"))]
    ImportAliasConflict {
        /// Requested local binding shared by the conflicting claims.
        requested_name: String,
        /// Why the conflict cannot be resolved.
        reason: String,
    },

    /// A caller-provided import alias resolver rejected the complete conflict set.
    #[snafu(display("import alias resolver rejected the conflict set: {reason}"))]
    ImportAliasResolverRejected {
        /// Resolver-provided diagnostic.
        reason: String,
    },

    /// A resolver returned incomplete, duplicate, unknown, or unsafe assignments.
    #[snafu(display("invalid import alias assignments: {reason}"))]
    InvalidImportAliasAssignments {
        /// Core validation failure.
        reason: String,
    },

    /// A target language rejected an otherwise complete resolved import group.
    #[snafu(display("language {language:?} rejected resolved imports: {reason}"))]
    InvalidResolvedImports {
        /// Language file extension.
        language: String,
        /// Target-local rejection reason.
        reason: String,
    },

    /// A semantic type reference is intrinsically malformed.
    #[snafu(display("invalid type reference at {context}: {reason}"))]
    InvalidTypeName {
        /// Stable source occurrence and structural path.
        context: String,
        /// Target-independent reason for rejection.
        reason: String,
    },

    /// A language adapter cannot represent a semantic type reference.
    #[snafu(display("language {language:?} cannot lower type reference at {context}: {reason}"))]
    UnsupportedTypeName {
        /// Language file extension.
        language: String,
        /// Stable source occurrence and structural path.
        context: String,
        /// Target-local reason for rejection.
        reason: String,
    },

    /// A rewritten source tree is structurally invalid.
    #[snafu(display("invalid rewritten source at {context}: {reason}"))]
    InvalidRewrittenSource {
        /// Stable source occurrence and structural path.
        context: String,
        /// Reason the rewritten source cannot continue through the pipeline.
        reason: String,
    },

    /// A language adapter returned an invalid type-expression block.
    #[snafu(display(
        "language {language:?} returned invalid type-expression output at {context}: {reason}"
    ))]
    InvalidTypeNameLowering {
        /// Language file extension.
        language: String,
        /// Stable source occurrence and structural path.
        context: String,
        /// Reason the adapter output was rejected.
        reason: String,
    },

    /// A non-terminal semantic type reached final rendering.
    #[snafu(display("non-terminal type reference reached final rendering at {context}"))]
    UnexpectedTypeReference {
        /// Final-render occurrence.
        context: String,
    },
}
