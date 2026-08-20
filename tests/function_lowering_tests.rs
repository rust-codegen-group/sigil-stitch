use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::capability::{
    FunctionCapability, FunctionCapabilityProfile, FunctionContext, FunctionForm,
    LanguageCapabilities,
};
use sigil_stitch::lang::{CodeLang, FunctionIntent, RendererLang, ValidatedFunction};
use sigil_stitch::spec::annotation_spec::{AnnotationNameRef, AnnotationSpec};
use sigil_stitch::spec::emittable::Emittable;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::{TypeParamKind, TypeParamSpec};
use sigil_stitch::type_name::TypeName;

#[derive(Debug, Clone)]
struct NovelLang {
    calls: Arc<AtomicUsize>,
    validation_calls: Arc<AtomicUsize>,
    strict: bool,
}

impl NovelLang {
    fn permissive() -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
            validation_calls: Arc::new(AtomicUsize::new(0)),
            strict: false,
        }
    }

    fn strict() -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
            validation_calls: Arc::new(AtomicUsize::new(0)),
            strict: true,
        }
    }
}

impl RendererLang for NovelLang {
    fn file_extension(&self) -> &str {
        "novel"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

const NOVEL_FUNCTIONS: &[FunctionCapabilityProfile<'_>] = &[FunctionCapabilityProfile::new(
    FunctionContext::TopLevel,
    FunctionForm::Function,
    &[
        FunctionCapability::ParametricPolymorphism,
        FunctionCapability::BoundedPolymorphism,
    ],
)];

impl CodeLang for NovelLang {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        if self.strict {
            LanguageCapabilities::strict().with_functions(NOVEL_FUNCTIONS)
        } else {
            LanguageCapabilities::permissive()
        }
    }

    fn validate_function(&self, function: FunctionIntent<'_>) -> Result<(), SigilStitchError> {
        self.validation_calls.fetch_add(1, Ordering::SeqCst);
        assert!(!function.name().is_empty());
        Ok(())
    }

    fn lower_function(
        &self,
        function: ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut block = CodeBlock::builder();
        block.add("declare %L(", function.name().to_string());
        for (index, parameter) in function.parameters().iter().enumerate() {
            if index > 0 {
                block.add(",%W", ());
            }
            block.add("%T", parameter.param_type().clone());
        }
        block.add(")", ());
        block.build()
    }
}

#[derive(Debug)]
struct LegacyLang;

impl RendererLang for LegacyLang {
    fn file_extension(&self) -> &str {
        "legacy-lowering"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for LegacyLang {
    fn function_keyword(&self, _context: DeclarationContext) -> &str {
        "legacy_fn"
    }
}

#[derive(Debug)]
struct SemanticViewLang;

impl RendererLang for SemanticViewLang {
    fn file_extension(&self) -> &str {
        "semantic-view"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

impl CodeLang for SemanticViewLang {
    fn lower_function(
        &self,
        function: ValidatedFunction<'_>,
    ) -> Result<CodeBlock, SigilStitchError> {
        assert_eq!(function.name(), "inspect");
        assert_eq!(function.declaration_context(), DeclarationContext::TopLevel);
        assert_eq!(function.function_context(), FunctionContext::ReceiverMethod);
        assert_eq!(function.form(), FunctionForm::Function);

        let receiver = function.receiver().unwrap();
        assert_eq!(receiver.name(), "self");
        assert_eq!(receiver.param_type(), &TypeName::primitive("Receiver"));
        assert!(receiver.default_value().is_none());
        assert!(!receiver.is_variadic());
        assert!(!receiver.is_property());
        assert!(!receiver.is_mutable_property());

        let parameters = function.parameters();
        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0].name(), "readonly_items");
        assert_eq!(parameters[0].param_type(), &TypeName::primitive("Item"));
        assert!(parameters[0].default_value().is_some());
        assert!(parameters[0].is_variadic());
        assert!(parameters[0].is_property());
        assert!(!parameters[0].is_mutable_property());
        assert_eq!(parameters[1].name(), "mutable_item");
        assert!(!parameters[1].is_variadic());
        assert!(!parameters[1].is_property());
        assert!(parameters[1].is_mutable_property());

        assert_eq!(function.return_type(), Some(&TypeName::primitive("Output")));
        assert!(function.body().is_some());
        assert_eq!(function.modifiers().visibility, Visibility::Public);
        assert!(function.modifiers().is_async);
        assert!(function.modifiers().is_static);
        assert!(function.modifiers().is_abstract);
        assert!(function.modifiers().is_override);
        assert_eq!(function.doc(), &["Semantic view".to_string()]);

        let type_params = function.type_params();
        assert_eq!(type_params.len(), 2);
        assert_eq!(type_params[0].name(), "'a");
        assert!(type_params[0].is_lifetime());
        assert_eq!(type_params[1].name(), "T");
        assert_eq!(type_params[1].bounds(), &[TypeName::primitive("Cloneable")]);
        assert!(matches!(
            type_params[1].kind(),
            Some(TypeParamKind::Constructor1)
        ));
        assert!(!type_params[1].is_lifetime());
        assert_eq!(
            type_params[1].context_bounds(),
            &[TypeName::primitive("Ordered")]
        );

        assert_eq!(function.annotations().len(), 1);
        let annotations = function.annotation_specs();
        assert_eq!(annotations.len(), 2);
        assert!(matches!(
            annotations[0].name(),
            AnnotationNameRef::Simple("simple")
        ));
        assert_eq!(annotations[0].arguments(), &["first", "second"]);
        assert!(matches!(
            annotations[1].name(),
            AnnotationNameRef::Importable(name)
                if name == &TypeName::importable("./annotations", "Tracked")
        ));
        assert!(annotations[1].arguments().is_empty());

        assert_eq!(function.suffixes(), &["const"]);
        assert!(function.delegation().is_some());
        let constraints = function.where_constraints();
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].subject(), &TypeName::primitive("T"));
        assert_eq!(
            constraints[0].bounds(),
            &[TypeName::primitive("Serializable")]
        );

        CodeBlock::of("semantic-view", ())
    }
}

#[test]
fn legacy_adapter_uses_the_default_compatibility_lowerer() {
    let function = FunSpec::builder("work")
        .add_param(ParameterSpec::of("value", TypeName::primitive("Text")))
        .returns(TypeName::primitive("Result"))
        .build()
        .unwrap();

    let rendered = render(&function, &LegacyLang, 80);
    assert_eq!(rendered, "legacy_fn work(value: Text): Result;\n");
}

#[test]
fn language_override_dispatches_through_every_function_facade() {
    let lang = NovelLang::permissive();
    let function = FunSpec::builder("work").build().unwrap();

    function.emit(&lang, DeclarationContext::TopLevel).unwrap();
    function.emit_members(&lang).unwrap();
    TypeSpec::builder("Container", TypeKind::Class)
        .add_method(function.clone())
        .build()
        .unwrap()
        .emit(&lang)
        .unwrap();
    FileSpec::builder_with("work.novel", lang.clone())
        .add_function(function)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert_eq!(lang.calls.load(Ordering::SeqCst), 4);
    assert_eq!(lang.validation_calls.load(Ordering::SeqCst), 6);
}

#[test]
fn validation_prevents_invalid_intent_from_reaching_the_lowerer() {
    let lang = NovelLang::strict();
    let function = FunSpec::builder("work").is_async().build().unwrap();

    assert!(function.emit(&lang, DeclarationContext::TopLevel).is_err());
    assert_eq!(lang.validation_calls.load(Ordering::SeqCst), 0);
    assert_eq!(lang.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn default_constraint_validation_is_syntax_independent() {
    let lang = NovelLang::strict();
    let function = FunSpec::builder("copy")
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::primitive("U"),
            vec![TypeName::primitive("Cloneable")],
        )
        .build()
        .unwrap();

    function.emit(&lang, DeclarationContext::TopLevel).unwrap();
    assert_eq!(lang.validation_calls.load(Ordering::SeqCst), 1);
    assert_eq!(lang.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn validated_function_exposes_complete_read_only_semantic_intent() {
    let function = FunSpec::builder("inspect")
        .receiver(ParameterSpec::of("self", TypeName::primitive("Receiver")))
        .add_param(
            ParameterSpec::builder("readonly_items", TypeName::primitive("Item"))
                .default_value(CodeBlock::of("default_item", ()).unwrap())
                .variadic()
                .is_property()
                .build()
                .unwrap(),
        )
        .add_param(
            ParameterSpec::builder("mutable_item", TypeName::primitive("Item"))
                .is_mutable_property()
                .build()
                .unwrap(),
        )
        .returns(TypeName::primitive("Output"))
        .body(CodeBlock::of("work", ()).unwrap())
        .visibility(Visibility::Public)
        .is_async()
        .is_static()
        .is_abstract()
        .is_override()
        .doc("Semantic view")
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_type_param(
            TypeParamSpec::new("T")
                .with_bound(TypeName::primitive("Cloneable"))
                .with_kind(TypeParamKind::Constructor1)
                .with_context_bound(TypeName::primitive("Ordered")),
        )
        .annotation(CodeBlock::of("@raw", ()).unwrap())
        .annotate(AnnotationSpec::new("simple").args(["first", "second"]))
        .annotate(AnnotationSpec::importable(TypeName::importable(
            "./annotations",
            "Tracked",
        )))
        .suffix("const")
        .delegation(CodeBlock::of("super()", ()).unwrap())
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Serializable")],
        )
        .build()
        .unwrap();

    let lowered = function
        .emit(&SemanticViewLang, DeclarationContext::TopLevel)
        .unwrap();
    assert!(!lowered.is_empty());
}

#[test]
fn boxed_adapter_preserves_type_refs_and_pretty_layout() {
    let lang = NovelLang::permissive();
    let boxed: Box<dyn CodeLang> = Box::new(lang.clone());
    let function = FunSpec::builder("merge")
        .add_param(ParameterSpec::of(
            "current",
            TypeName::importable("./models", "User"),
        ))
        .add_param(ParameterSpec::of(
            "legacy",
            TypeName::importable("./legacy", "User"),
        ))
        .build()
        .unwrap();

    let boxed_block = function
        .emit(boxed.as_ref(), DeclarationContext::TopLevel)
        .unwrap();
    assert!(!boxed_block.is_empty());

    let direct = FileSpec::builder_with("merge.novel", lang.clone())
        .add_function(function.clone())
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    let pretty = FileSpec::builder_with("merge.novel", lang)
        .add_function(function)
        .build()
        .unwrap()
        .render(12)
        .unwrap();

    assert!(
        direct.contains("declare merge(User, LegacyUser)"),
        "{direct}"
    );
    assert!(pretty.contains("User,"), "{pretty}");
    assert!(pretty.contains("LegacyUser"), "{pretty}");
    assert!(pretty.contains('\n'), "{pretty}");
}

fn render(function: &FunSpec, lang: &dyn CodeLang, width: usize) -> String {
    let block = function.emit(lang, DeclarationContext::TopLevel).unwrap();
    let imports = sigil_stitch::import::ImportGroup::new();
    let mut renderer = sigil_stitch::code_renderer::CodeRenderer::new(lang, &imports, width);
    renderer.render(&block).unwrap()
}
