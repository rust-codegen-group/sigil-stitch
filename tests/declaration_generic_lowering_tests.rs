use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_renderer::CodeRenderer;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::ImportGroup;
use sigil_stitch::lang::capability::{
    FunctionBodyPolicy, FunctionCapabilityProfile, FunctionContext, FunctionForm,
    LanguageCapabilities,
};
use sigil_stitch::lang::{CodeLang, RendererLang};
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{DeclarationContext, TypeKind};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::{TypeParamKind, TypeParamSpec};
use sigil_stitch::type_name::TypeName;
use std::fmt::Write;

#[path = "shared/golden.rs"]
mod golden;
#[path = "shared/languages.rs"]
mod languages_registry;

const TYPE_GENERIC_LANGUAGES: [&str; 12] = [
    "cpp",
    "csharp",
    "dart",
    "go",
    "haskell",
    "java",
    "kotlin",
    "ocaml",
    "rust",
    "scala",
    "swift",
    "typescript",
];

const TYPE_CLASS_LANGUAGES: [&str; 17] = [
    "c",
    "cpp",
    "csharp",
    "dart",
    "go",
    "haskell",
    "java",
    "javascript",
    "kotlin",
    "ocaml",
    "php",
    "python",
    "ruby",
    "rust",
    "scala",
    "swift",
    "typescript",
];

const FUNCTION_GENERIC_LANGUAGES: [&str; 10] = [
    "csharp",
    "dart",
    "go",
    "haskell",
    "java",
    "kotlin",
    "rust",
    "scala",
    "swift",
    "typescript",
];

const BOUNDED_TYPE_LANGUAGES: [&str; 10] = [
    "csharp",
    "dart",
    "go",
    "haskell",
    "java",
    "kotlin",
    "rust",
    "scala",
    "swift",
    "typescript",
];

fn parameter_name(language: &str, index: usize) -> &'static str {
    match (language, index) {
        ("haskell" | "ocaml", 0) => "a",
        ("haskell" | "ocaml", 1) => "b",
        (_, 0) => "T",
        (_, 1) => "U",
        _ => unreachable!(),
    }
}

fn type_name(language: &str) -> &'static str {
    if language == "ocaml" { "box" } else { "Box" }
}

fn generic_type(language: &str, parameter_count: usize, bound: bool) -> TypeSpec {
    let field_type = if parameter_count == 0 {
        TypeName::primitive("Value")
    } else {
        TypeName::primitive(parameter_name(language, 0))
    };
    let mut builder = TypeSpec::builder(type_name(language), TypeKind::Class);
    if language == "c" || TYPE_GENERIC_LANGUAGES.contains(&language) {
        builder = builder.add_field(FieldSpec::builder("value", field_type).build().unwrap());
    }
    for index in 0..parameter_count {
        let mut parameter = TypeParamSpec::new(parameter_name(language, index));
        if bound && index == 0 {
            parameter = parameter.with_bound(TypeName::primitive("Bound"));
        }
        builder = builder.add_type_param(parameter);
    }
    builder.build().unwrap()
}

fn type_with_where_constraint(language: &str) -> TypeSpec {
    let name = parameter_name(language, 0);
    TypeSpec::builder(type_name(language), TypeKind::Class)
        .add_type_param(TypeParamSpec::new(name))
        .add_where_constraint(
            TypeName::primitive(name),
            vec![TypeName::primitive("Bound")],
        )
        .add_field(
            FieldSpec::builder("value", TypeName::primitive(name))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn type_with_type_parameter(language: &str, parameter: TypeParamSpec) -> TypeSpec {
    let field_type = if parameter.is_lifetime() {
        TypeName::primitive("Value")
    } else {
        TypeName::primitive(parameter.name())
    };
    TypeSpec::builder(type_name(language), TypeKind::Class)
        .add_type_param(parameter)
        .add_field(FieldSpec::builder("value", field_type).build().unwrap())
        .build()
        .unwrap()
}

fn imported_constraint_modules(language: &str) -> (&'static str, &'static str) {
    match language {
        "haskell" | "swift" => ("Alpha", "Beta"),
        _ => ("alpha", "beta"),
    }
}

fn imported_constraint_bounds(language: &str) -> (TypeName, TypeName) {
    let (first_module, second_module) = imported_constraint_modules(language);
    (
        TypeName::importable(first_module, "FirstConstraint"),
        TypeName::importable(second_module, "SecondConstraint"),
    )
}

fn type_with_imported_constraints(language: &str) -> TypeSpec {
    let first_name = parameter_name(language, 0);
    let second_name = parameter_name(language, 1);
    let (first_bound, second_bound) = imported_constraint_bounds(language);
    TypeSpec::builder(type_name(language), TypeKind::Class)
        .add_type_param(TypeParamSpec::new(first_name))
        .add_type_param(TypeParamSpec::new(second_name))
        .add_where_constraint(TypeName::primitive(first_name), vec![first_bound])
        .add_where_constraint(TypeName::primitive(second_name), vec![second_bound])
        .add_field(
            FieldSpec::builder("value", TypeName::primitive(first_name))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
}

fn function_with_imported_constraints(language: &str) -> FunSpec {
    let first_name = parameter_name(language, 0);
    let second_name = parameter_name(language, 1);
    let (first_bound, second_bound) = imported_constraint_bounds(language);
    FunSpec::builder("work")
        .add_type_param(TypeParamSpec::new(first_name))
        .add_type_param(TypeParamSpec::new(second_name))
        .add_where_constraint(TypeName::primitive(first_name), vec![first_bound])
        .add_where_constraint(TypeName::primitive(second_name), vec![second_bound])
        .add_param(ParameterSpec::of("value", TypeName::primitive(first_name)))
        .returns(TypeName::primitive(second_name))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap()
}

fn render_type(language: &str, spec: &TypeSpec, width: usize) -> Result<String, SigilStitchError> {
    let adapter = languages_registry::adapter_for(language);
    let imports = ImportGroup::new();
    spec.emit(adapter.as_ref())?
        .iter()
        .map(|block| CodeRenderer::new(adapter.as_ref(), &imports, width).render(block))
        .collect::<Result<Vec<_>, _>>()
        .map(|blocks| blocks.join("\n"))
}

fn function_context(language: &str) -> DeclarationContext {
    match language {
        "csharp" | "java" => DeclarationContext::Member,
        _ => DeclarationContext::TopLevel,
    }
}

fn generic_function(language: &str, parameter_count: usize, bound: bool) -> FunSpec {
    let mut builder = FunSpec::builder("work").body(CodeBlock::of("body", ()).unwrap());
    if parameter_count == 0 {
        if matches!(language, "c" | "cpp" | "csharp" | "java") {
            builder = builder.returns(TypeName::primitive("Value"));
        }
        return builder.build().unwrap();
    }

    for index in 0..parameter_count {
        let name = parameter_name(language, index);
        let mut parameter = TypeParamSpec::new(name);
        if bound && index == 0 {
            parameter = parameter.with_bound(TypeName::primitive("Bound"));
        }
        builder = builder
            .add_type_param(parameter)
            .add_param(ParameterSpec::of(
                if index == 0 { "first" } else { "second" },
                TypeName::primitive(name),
            ));
    }
    builder
        .returns(TypeName::primitive(parameter_name(language, 0)))
        .build()
        .unwrap()
}

fn function_with_type_parameter(parameter: TypeParamSpec) -> FunSpec {
    FunSpec::builder("work")
        .add_type_param(parameter)
        .add_param(ParameterSpec::of("value", TypeName::primitive("Value")))
        .returns(TypeName::primitive("Value"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap()
}

fn function_with_where_constraint(language: &str) -> FunSpec {
    let name = parameter_name(language, 0);
    FunSpec::builder("work")
        .add_type_param(TypeParamSpec::new(name))
        .add_param(ParameterSpec::of("value", TypeName::primitive(name)))
        .returns(TypeName::primitive(name))
        .add_where_constraint(
            TypeName::primitive(name),
            vec![TypeName::primitive("Bound")],
        )
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap()
}

fn render_function(
    language: &str,
    spec: &FunSpec,
    width: usize,
) -> Result<String, SigilStitchError> {
    let adapter = languages_registry::adapter_for(language);
    let imports = ImportGroup::new();
    let block = spec.emit(adapter.as_ref(), function_context(language))?;
    CodeRenderer::new(adapter.as_ref(), &imports, width).render(&block)
}

fn render_imported_type_constraints(
    language: languages_registry::BuiltInLanguage,
    width: usize,
) -> Result<String, SigilStitchError> {
    FileSpec::builder(&format!("bounds.{}", language.extension))
        .add_type(type_with_imported_constraints(language.id))
        .build()
        .unwrap()
        .render(width)
}

fn render_imported_function_constraints(
    language: languages_registry::BuiltInLanguage,
    width: usize,
) -> Result<String, SigilStitchError> {
    let builder = FileSpec::builder(&format!("bounds.{}", language.extension));
    let builder = if matches!(language.id, "csharp" | "java") {
        builder.add_type(
            TypeSpec::builder("Bounds", TypeKind::Class)
                .add_method(function_with_imported_constraints(language.id))
                .build()
                .unwrap(),
        )
    } else {
        builder.add_function(function_with_imported_constraints(language.id))
    };
    builder.build().unwrap().render(width)
}

fn type_fragment(language: &str, count: usize) -> &'static str {
    match (language, count) {
        ("cpp", 1) => "template <typename T>\nclass Box",
        ("cpp", 2) => "template <typename T, typename U>\nclass Box",
        ("csharp" | "dart" | "java" | "kotlin" | "swift" | "typescript", 1) => "class Box<T>",
        ("csharp" | "dart" | "java" | "kotlin" | "swift" | "typescript", 2) => "class Box<T, U>",
        ("go", 1) => "type Box[T any] struct",
        ("go", 2) => "type Box[T any, U any] struct",
        ("haskell", 1) => "data Box a",
        ("haskell", 2) => "data Box a b",
        ("ocaml", 1) => "type 'a box =",
        ("ocaml", 2) => "type ('a, 'b) box =",
        ("rust", 1) => "struct Box<T>",
        ("rust", 2) => "struct Box<T, U>",
        ("scala", 1) => "class Box[T]",
        ("scala", 2) => "class Box[T, U]",
        _ => unreachable!("missing type-generic expectation for {language} with {count}"),
    }
}

fn function_fragment(language: &str, count: usize) -> &'static str {
    match (language, count) {
        ("csharp", 1) => "T work<T>",
        ("csharp", 2) => "T work<T, U>",
        ("dart", 1) => "T work<T>",
        ("dart", 2) => "T work<T, U>",
        ("go", 1) => "func work[T any]",
        ("go", 2) => "func work[T any, U any]",
        ("haskell", 1) => "work :: a -> a",
        ("haskell", 2) => "work :: a -> b -> a",
        ("java", 1) => "<T> T work",
        ("java", 2) => "<T, U> T work",
        ("kotlin", 1) => "fun <T> work",
        ("kotlin", 2) => "fun <T, U> work",
        ("rust", 1) => "fn work<T>",
        ("rust", 2) => "fn work<T, U>",
        ("scala", 1) => "def work[T]",
        ("scala", 2) => "def work[T, U]",
        ("swift", 1) => "func work<T>",
        ("swift", 2) => "func work<T, U>",
        ("typescript", 1) => "function work<T>",
        ("typescript", 2) => "function work<T, U>",
        _ => unreachable!("missing function-generic expectation for {language} with {count}"),
    }
}

#[test]
fn type_declaration_generic_matrix_covers_zero_one_and_many_parameters() {
    for language in languages_registry::BUILT_IN_LANGUAGES {
        let zero = render_type(language.id, &generic_type(language.id, 0, false), 100);
        if TYPE_CLASS_LANGUAGES.contains(&language.id) {
            assert!(
                zero.is_ok(),
                "{} zero-parameter type: {zero:?}",
                language.id
            );
        } else {
            assert!(
                matches!(zero, Err(SigilStitchError::UnsupportedTypeKind { .. })),
                "{} zero-parameter type: {zero:?}",
                language.id
            );
        }

        for count in [1, 2] {
            let result = render_type(language.id, &generic_type(language.id, count, false), 100);
            if TYPE_GENERIC_LANGUAGES.contains(&language.id) {
                let output = result.unwrap_or_else(|error| panic!("{}: {error}", language.id));
                assert!(
                    output.contains(type_fragment(language.id, count)),
                    "{}: {output}",
                    language.id
                );
            } else {
                if TYPE_CLASS_LANGUAGES.contains(&language.id) {
                    assert!(
                        matches!(
                            result,
                            Err(SigilStitchError::UnsupportedTypeCapabilities { .. })
                        ),
                        "{}: {result:?}",
                        language.id
                    );
                } else {
                    assert!(
                        matches!(result, Err(SigilStitchError::UnsupportedTypeKind { .. })),
                        "{}: {result:?}",
                        language.id
                    );
                }
            }
        }
    }
}

#[test]
fn function_declaration_generic_matrix_covers_zero_one_and_many_parameters() {
    for language in languages_registry::BUILT_IN_LANGUAGES {
        let zero = render_function(language.id, &generic_function(language.id, 0, false), 100);
        assert!(
            zero.is_ok(),
            "{} zero-parameter function: {zero:?}",
            language.id
        );

        for count in [1, 2] {
            let function = generic_function(language.id, count, false);
            let wide = render_function(language.id, &function, 100);
            if FUNCTION_GENERIC_LANGUAGES.contains(&language.id) {
                let output = wide.unwrap_or_else(|error| panic!("{}: {error}", language.id));
                assert!(
                    output.contains(function_fragment(language.id, count)),
                    "{}: {output}",
                    language.id
                );
                let narrow = render_function(language.id, &function, 12).unwrap();
                assert!(
                    narrow.contains(function_fragment(language.id, count)),
                    "{} narrow: {narrow}",
                    language.id
                );
            } else {
                match language.id {
                    "bash" | "zsh" => assert!(
                        matches!(
                            wide,
                            Err(SigilStitchError::TooManyFunctionParameters { .. })
                        ),
                        "{}: {wide:?}",
                        language.id
                    ),
                    _ => assert!(
                        matches!(
                            wide,
                            Err(SigilStitchError::UnsupportedFunctionCapabilities { .. })
                        ),
                        "{}: {wide:?}",
                        language.id
                    ),
                }
            }
        }
    }
}

fn append_matrix_result(
    report: &mut String,
    language: &str,
    declaration: &str,
    parameter_shape: &str,
    width: usize,
    result: Result<String, SigilStitchError>,
) {
    writeln!(
        report,
        "===== {language} {declaration} {parameter_shape} width={width} ====="
    )
    .unwrap();
    match result {
        Ok(output) => report.push_str(&output),
        Err(error) => writeln!(report, "ERROR: {error:?}").unwrap(),
    }
    if !report.ends_with('\n') {
        report.push('\n');
    }
}

#[test]
fn declaration_generic_matrix_matches_exact_wide_and_narrow_results() {
    let mut report = String::new();
    for language in languages_registry::BUILT_IN_LANGUAGES {
        for (count, shape) in [(0, "zero"), (1, "one"), (2, "many")] {
            let type_ = generic_type(language.id, count, false);
            let function = generic_function(language.id, count, false);
            for width in [100, 12] {
                append_matrix_result(
                    &mut report,
                    language.id,
                    "type",
                    shape,
                    width,
                    render_type(language.id, &type_, width),
                );
                append_matrix_result(
                    &mut report,
                    language.id,
                    "function",
                    shape,
                    width,
                    render_function(language.id, &function, width),
                );
            }
        }

        let bounded_type = generic_type(language.id, 1, true);
        let bounded_function = generic_function(language.id, 1, true);
        for width in [100, 12] {
            append_matrix_result(
                &mut report,
                language.id,
                "type",
                "bounded",
                width,
                render_type(language.id, &bounded_type, width),
            );
            append_matrix_result(
                &mut report,
                language.id,
                "function",
                "bounded",
                width,
                render_function(language.id, &bounded_function, width),
            );
        }

        let constrained_type = type_with_where_constraint(language.id);
        for width in [100, 12] {
            append_matrix_result(
                &mut report,
                language.id,
                "type",
                "explicit-constraint",
                width,
                render_type(language.id, &constrained_type, width),
            );
        }

        let parameter = parameter_name(language.id, 0);
        let semantic_types = [
            (
                "context-bound",
                type_with_type_parameter(
                    language.id,
                    TypeParamSpec::new(parameter)
                        .with_context_bound(TypeName::primitive("Context")),
                ),
            ),
            (
                "lifetime",
                type_with_type_parameter(language.id, TypeParamSpec::lifetime("'a")),
            ),
            (
                "higher-kinded",
                type_with_type_parameter(
                    language.id,
                    TypeParamSpec::new(if language.id == "scala" {
                        "F"
                    } else {
                        parameter
                    })
                    .with_kind(TypeParamKind::Constructor1),
                ),
            ),
        ];
        for (shape, type_) in semantic_types {
            for width in [100, 12] {
                append_matrix_result(
                    &mut report,
                    language.id,
                    "type",
                    shape,
                    width,
                    render_type(language.id, &type_, width),
                );
            }
        }

        let semantic_functions = [
            (
                "context-bound",
                function_with_type_parameter(
                    TypeParamSpec::new(parameter)
                        .with_context_bound(TypeName::primitive("Context")),
                ),
            ),
            (
                "lifetime",
                function_with_type_parameter(TypeParamSpec::lifetime("'a")),
            ),
            (
                "higher-kinded",
                function_with_type_parameter(
                    TypeParamSpec::new(if language.id == "scala" {
                        "F"
                    } else {
                        parameter
                    })
                    .with_kind(TypeParamKind::Constructor1),
                ),
            ),
            (
                "explicit-constraint",
                function_with_where_constraint(language.id),
            ),
        ];
        for (shape, function) in semantic_functions {
            for width in [100, 12] {
                append_matrix_result(
                    &mut report,
                    language.id,
                    "function",
                    shape,
                    width,
                    render_function(language.id, &function, width),
                );
            }
        }

        for width in [100, 12] {
            append_matrix_result(
                &mut report,
                language.id,
                "type",
                "imported-constraints",
                width,
                render_imported_type_constraints(language, width),
            );
            append_matrix_result(
                &mut report,
                language.id,
                "function",
                "imported-constraints",
                width,
                render_imported_function_constraints(language, width),
            );
        }
    }

    golden::assert_golden("declaration_generics/matrix.txt", &report);
}

fn assert_invalid_function_type_parameter(
    language: &str,
    result: Result<String, SigilStitchError>,
) {
    assert!(
        matches!(
            result,
            Err(SigilStitchError::InvalidFunctionTypeParameter { .. })
        ),
        "{language}: {result:?}"
    );
}

#[test]
fn function_type_parameter_semantics_are_lowered_or_rejected() {
    for language in FUNCTION_GENERIC_LANGUAGES {
        let parameter = parameter_name(language, 0);
        let context_bound = function_with_type_parameter(
            TypeParamSpec::new(parameter).with_context_bound(TypeName::primitive("Context")),
        );
        let context_result = render_function(language, &context_bound, 100);
        match language {
            "csharp" => assert_eq!(
                context_result.unwrap(),
                "Value work<T>(Value @value)\n    where T : Context\n{\n    body\n}\n"
            ),
            "haskell" => {
                assert_invalid_function_type_parameter(language, context_result);
            }
            "scala" => assert_eq!(
                context_result.unwrap(),
                "def work[T : Context](value: Value): Value = {\n  body\n}\n"
            ),
            _ => assert_invalid_function_type_parameter(language, context_result),
        }

        let lifetime = function_with_type_parameter(TypeParamSpec::lifetime("'a"));
        let lifetime_result = render_function(language, &lifetime, 100);
        if language == "rust" {
            assert_eq!(
                lifetime_result.unwrap(),
                "fn work<'a>(value: Value) -> Value {\n    body\n}\n"
            );
        } else {
            assert_invalid_function_type_parameter(language, lifetime_result);
        }

        let invalid_name = function_with_type_parameter(TypeParamSpec::new("not-valid"));
        assert_invalid_function_type_parameter(
            language,
            render_function(language, &invalid_name, 100),
        );
    }

    let dart_with_two_bounds = function_with_type_parameter(
        TypeParamSpec::new("T")
            .with_bound(TypeName::primitive("First"))
            .with_bound(TypeName::primitive("Second")),
    );
    assert_invalid_function_type_parameter(
        "dart",
        render_function("dart", &dart_with_two_bounds, 100),
    );

    let dart_with_direct_and_explicit_bounds = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("First")))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Second")],
        )
        .add_param(ParameterSpec::of("value", TypeName::primitive("T")))
        .returns(TypeName::primitive("T"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert_invalid_function_type_parameter(
        "dart",
        render_function("dart", &dart_with_direct_and_explicit_bounds, 100),
    );

    let dart_with_duplicate_direct_and_explicit_bound = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("Bound")))
        .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("Bound")])
        .add_param(ParameterSpec::of("value", TypeName::primitive("T")))
        .returns(TypeName::primitive("T"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render_function("dart", &dart_with_duplicate_direct_and_explicit_bound, 100).unwrap(),
        "T work<T extends Bound>(T value) {\n  body\n}\n"
    );

    let haskell_with_direct_and_context_bounds = function_with_type_parameter(
        TypeParamSpec::new("a")
            .with_bound(TypeName::primitive("Eq"))
            .with_context_bound(TypeName::primitive("Ord")),
    );
    assert_invalid_function_type_parameter(
        "haskell",
        render_function("haskell", &haskell_with_direct_and_context_bounds, 100),
    );

    for invalid_rust_parameter in [
        TypeParamSpec::lifetime("a"),
        TypeParamSpec::lifetime("'static"),
        TypeParamSpec::lifetime("'_"),
        TypeParamSpec::lifetime("'self"),
        TypeParamSpec::lifetime("'a").with_bound(TypeName::primitive("Clone")),
        TypeParamSpec::lifetime("'a").with_bound(TypeName::array(TypeName::primitive("Clone"))),
        TypeParamSpec::new("'a"),
    ] {
        let function = function_with_type_parameter(invalid_rust_parameter);
        assert_invalid_function_type_parameter("rust", render_function("rust", &function, 100));
    }

    let static_lifetime_bound = function_with_type_parameter(
        TypeParamSpec::lifetime("'a").with_bound(TypeName::primitive("'static")),
    );
    assert_eq!(
        render_function("rust", &static_lifetime_bound, 100).unwrap(),
        "fn work<'a: 'static>(value: Value) -> Value {\n    body\n}\n"
    );

    let declared_lifetime_bound = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::lifetime("'b"))
        .add_type_param(TypeParamSpec::lifetime("'a").with_bound(TypeName::primitive("'b")))
        .add_param(ParameterSpec::of("value", TypeName::primitive("Value")))
        .returns(TypeName::primitive("Value"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render_function("rust", &declared_lifetime_bound, 100).unwrap(),
        "fn work<'b, 'a: 'b>(value: Value) -> Value {\n    body\n}\n"
    );

    let explicit_lifetime_bound = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_where_constraint(
            TypeName::primitive("'a"),
            vec![TypeName::primitive("'static")],
        )
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        render_function("rust", &explicit_lifetime_bound, 100)
            .unwrap()
            .contains("where\n    'a: 'static,")
    );

    let invalid_lifetime_constraint = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_where_constraint(
            TypeName::primitive("'a"),
            vec![TypeName::primitive("Clone")],
        )
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert_invalid_function_type_parameter(
        "rust",
        render_function("rust", &invalid_lifetime_constraint, 100),
    );

    let undeclared_lifetime_constraint = FunSpec::builder("work")
        .add_where_constraint(
            TypeName::primitive("'missing"),
            vec![TypeName::primitive("'static")],
        )
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert!(matches!(
        render_function("rust", &undeclared_lifetime_constraint, 100),
        Err(SigilStitchError::InvalidFunctionConstraintSubject { .. })
    ));

    let compound_type_constraint = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(
            TypeName::generic(TypeName::primitive("Vec"), vec![TypeName::primitive("T")]),
            vec![TypeName::primitive("Clone")],
        )
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert!(
        render_function("rust", &compound_type_constraint, 100)
            .unwrap()
            .contains("where\n    Vec<T>: Clone,")
    );
}

#[test]
fn haskell_function_type_parameters_must_occur_in_the_signature() {
    let unused = function_with_type_parameter(TypeParamSpec::new("a"));
    assert_invalid_function_type_parameter("haskell", render_function("haskell", &unused, 100));

    let nested_use = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::new("a"))
        .add_param(ParameterSpec::of(
            "values",
            TypeName::array(TypeName::primitive("a")),
        ))
        .returns(TypeName::primitive("Value"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    assert_eq!(
        render_function("haskell", &nested_use, 100).unwrap(),
        "work :: [a] -> Value\nwork values =\n  body\n"
    );
}

#[test]
fn java_declaration_constraints_deduplicate_exact_direct_bounds() {
    let parameter = || {
        TypeParamSpec::new("T")
            .with_bound(TypeName::primitive("Bound"))
            .with_bound(TypeName::primitive("Bound"))
    };
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(parameter())
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("T"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let function = FunSpec::builder("work")
        .add_type_param(parameter())
        .add_param(ParameterSpec::of("value", TypeName::primitive("T")))
        .returns(TypeName::primitive("T"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();

    assert_eq!(
        render_type("java", &type_, 100).unwrap(),
        "class Box<T extends Bound> {\n    T value;\n}\n"
    );
    assert_eq!(
        render_function("java", &function, 100).unwrap(),
        "<T extends Bound> T work(T value) {\n    body\n}\n"
    );
}

#[test]
fn java_declaration_constraints_reject_duplicate_bound_erasures() {
    let container = |element| {
        TypeName::generic(
            TypeName::importable("constraints", "Container"),
            vec![TypeName::primitive(element)],
        )
    };
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T").with_bound(container("String")))
        .add_where_constraint(TypeName::primitive("T"), vec![container("Integer")])
        .build()
        .unwrap();
    let function = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::new("T").with_bound(container("String")))
        .add_where_constraint(TypeName::primitive("T"), vec![container("Integer")])
        .add_param(ParameterSpec::of("value", TypeName::primitive("T")))
        .returns(TypeName::primitive("T"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();

    assert!(matches!(
        render_type("java", &type_, 100),
        Err(SigilStitchError::InvalidTypeParameter {
            parameter_name,
            reason,
            ..
        }) if parameter_name == "T" && reason.contains("same erased type")
    ));
    assert!(matches!(
        render_function("java", &function, 100),
        Err(SigilStitchError::InvalidFunctionTypeParameter {
            parameter_name,
            reason,
            ..
        }) if parameter_name == "T" && reason.contains("same erased type")
    ));
}

#[test]
fn csharp_type_constraints_deduplicate_exact_bounds() {
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("Bound")))
        .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("Bound")])
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("T"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    assert_eq!(
        render_type("csharp", &type_, 100).unwrap(),
        "internal class Box<T>\n    where T : Bound\n{\n    T @value;\n}\n"
    );
}

#[test]
fn csharp_type_constraints_deduplicate_semantic_imports() {
    let bound = TypeName::importable("constraints", "Bound");
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T").with_bound(bound.clone()))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![bound.with_alias("PreferredBound")],
        )
        .build()
        .unwrap();

    assert_eq!(
        FileSpec::builder("bounds.cs")
            .add_type(type_)
            .build()
            .unwrap()
            .render(100)
            .unwrap(),
        "using constraints;\n\ninternal class Box<T>\n    where T : Bound\n{\n}\n"
    );
}

#[test]
fn csharp_function_constraints_deduplicate_semantic_imports() {
    let bound = TypeName::importable("constraints", "Bound");
    let method = FunSpec::builder("Work")
        .add_type_param(TypeParamSpec::new("T").with_bound(bound.clone()))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![bound.with_alias("PreferredBound")],
        )
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    let type_ = TypeSpec::builder("Owner", TypeKind::Class)
        .add_method(method)
        .build()
        .unwrap();

    for width in [100, 12] {
        assert_eq!(
            FileSpec::builder("bounds.cs")
                .add_type(type_.clone())
                .build()
                .unwrap()
                .render(width)
                .unwrap(),
            "using constraints;\n\ninternal class Owner {\n    void Work<T>()\n        where T : Bound\n    {\n        body\n    }\n}\n"
        );
    }
}

#[test]
fn csharp_constraint_deduplication_preserves_qualified_references() {
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::generic(
            TypeName::importable("constraints", "Container"),
            vec![TypeName::qualified("models", "Item")],
        )))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::generic(
                TypeName::qualified("constraints", "Container"),
                vec![TypeName::importable("models", "Item")],
            )],
        )
        .build()
        .unwrap();

    for width in [100, 12] {
        assert_eq!(
            FileSpec::builder("bounds.cs")
                .add_type(type_.clone())
                .build()
                .unwrap()
                .render(width)
                .unwrap(),
            "internal class Box<T>\n    where T : constraints.Container<models.Item>\n{\n}\n"
        );
    }
}

#[test]
fn csharp_constraints_deduplicate_equivalent_terminal_spellings() {
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::raw("IDisposable")))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("IDisposable")],
        )
        .build()
        .unwrap();

    assert_eq!(
        render_type("csharp", &type_, 100).unwrap(),
        "internal class Box<T>\n    where T : IDisposable\n{\n}\n"
    );
}

#[test]
fn csharp_constraint_sources_share_target_order() {
    let parameter = || {
        TypeParamSpec::new("T")
            .with_bound(TypeName::primitive("new()"))
            .with_context_bound(TypeName::primitive("IDisposable"))
    };
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(parameter())
        .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("class")])
        .build()
        .unwrap();
    let function = FunSpec::builder("Work")
        .add_type_param(parameter())
        .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("class")])
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();

    assert_eq!(
        render_type("csharp", &type_, 100).unwrap(),
        "internal class Box<T>\n    where T : class, IDisposable, new()\n{\n}\n"
    );
    assert_eq!(
        render_function("csharp", &function, 100).unwrap(),
        "void Work<T>()\n    where T : class, IDisposable, new()\n{\n    body\n}\n"
    );
}

#[test]
fn csharp_conflicting_special_constraints_fail_closed() {
    let conflicting_bounds = [
        vec![TypeName::primitive("class"), TypeName::primitive("struct")],
        vec![TypeName::primitive("struct"), TypeName::primitive("new()")],
        vec![
            TypeName::primitive("struct"),
            TypeName::primitive("unmanaged"),
        ],
        vec![
            TypeName::primitive("class"),
            TypeName::primitive("allows ref struct"),
        ],
        vec![
            TypeName::primitive("class?"),
            TypeName::primitive("allows ref struct"),
        ],
    ];

    for bounds in conflicting_bounds {
        let mut type_parameter = TypeParamSpec::new("T");
        let mut function_parameter = TypeParamSpec::new("T");
        for bound in bounds {
            type_parameter = type_parameter.with_bound(bound.clone());
            function_parameter = function_parameter.with_bound(bound);
        }
        let type_ = TypeSpec::builder("Box", TypeKind::Class)
            .add_type_param(type_parameter)
            .build()
            .unwrap();
        let function = FunSpec::builder("Work")
            .add_type_param(function_parameter)
            .returns(TypeName::primitive("void"))
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap();

        assert!(matches!(
            render_type("csharp", &type_, 100),
            Err(SigilStitchError::InvalidTypeParameter { .. })
        ));
        assert!(matches!(
            render_function("csharp", &function, 100),
            Err(SigilStitchError::InvalidFunctionTypeParameter { .. })
        ));
    }
}

#[test]
fn csharp_default_constraint_requires_an_override_and_stands_alone() {
    let type_ = TypeSpec::builder("Box", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("default")))
        .build()
        .unwrap();
    let ordinary_function = FunSpec::builder("Work")
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("default")))
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    let override_function = FunSpec::builder("Work")
        .is_override()
        .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive("default")))
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();
    let constrained_override = FunSpec::builder("Work")
        .is_override()
        .add_type_param(
            TypeParamSpec::new("T")
                .with_bound(TypeName::primitive("default"))
                .with_bound(TypeName::primitive("IDisposable")),
        )
        .returns(TypeName::primitive("void"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();

    assert!(matches!(
        render_type("csharp", &type_, 100),
        Err(SigilStitchError::InvalidTypeParameter { .. })
    ));
    assert!(matches!(
        render_function("csharp", &ordinary_function, 100),
        Err(SigilStitchError::InvalidFunctionTypeParameter { .. })
    ));
    assert_eq!(
        render_function("csharp", &override_function, 100).unwrap(),
        "override void Work<T>()\n    where T : default\n{\n    body\n}\n"
    );
    assert!(matches!(
        render_function("csharp", &constrained_override, 100),
        Err(SigilStitchError::InvalidFunctionTypeParameter { .. })
    ));
}

#[test]
fn csharp_override_constraints_follow_target_rules() {
    for bound in [
        TypeName::primitive("class?"),
        TypeName::primitive("notnull"),
        TypeName::primitive("unmanaged"),
        TypeName::primitive("IDisposable"),
        TypeName::primitive("new()"),
        TypeName::primitive("allows ref struct"),
    ] {
        let function = FunSpec::builder("Work")
            .is_override()
            .add_type_param(TypeParamSpec::new("T").with_bound(bound))
            .returns(TypeName::primitive("void"))
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap();

        assert!(matches!(
            render_function("csharp", &function, 100),
            Err(SigilStitchError::InvalidFunctionTypeParameter { .. })
        ));
    }

    for bound in ["class", "struct"] {
        let function = FunSpec::builder("Work")
            .is_override()
            .add_type_param(TypeParamSpec::new("T").with_bound(TypeName::primitive(bound)))
            .returns(TypeName::primitive("void"))
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap();

        assert_eq!(
            render_function("csharp", &function, 100).unwrap(),
            format!("override void Work<T>()\n    where T : {bound}\n{{\n    body\n}}\n")
        );
    }
}

#[test]
fn csharp_constraints_reject_invalid_type_shapes() {
    for bound in [
        TypeName::primitive("int"),
        TypeName::pointer(TypeName::primitive("int")),
        TypeName::tuple(vec![TypeName::primitive("int"), TypeName::primitive("int")]),
        TypeName::optional(TypeName::primitive("struct")),
        TypeName::optional(TypeName::optional(TypeName::primitive("IDisposable"))),
    ] {
        let type_ = TypeSpec::builder("Box", TypeKind::Class)
            .add_type_param(TypeParamSpec::new("T").with_bound(bound.clone()))
            .build()
            .unwrap();
        let function = FunSpec::builder("Work")
            .add_type_param(TypeParamSpec::new("T").with_bound(bound))
            .returns(TypeName::primitive("void"))
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap();

        assert!(matches!(
            render_type("csharp", &type_, 100),
            Err(SigilStitchError::InvalidTypeParameter { .. })
        ));
        assert!(matches!(
            render_function("csharp", &function, 100),
            Err(SigilStitchError::InvalidFunctionTypeParameter { .. })
        ));
    }
}

#[test]
fn csharp_nullable_and_non_nullable_constraints_cannot_repeat() {
    for nullable in [
        TypeName::optional(TypeName::primitive("IDisposable")),
        TypeName::raw("IDisposable?"),
    ] {
        let parameter = || {
            TypeParamSpec::new("T")
                .with_bound(TypeName::primitive("IDisposable"))
                .with_bound(nullable.clone())
        };
        let type_ = TypeSpec::builder("Box", TypeKind::Class)
            .add_type_param(parameter())
            .build()
            .unwrap();
        let function = FunSpec::builder("Work")
            .add_type_param(parameter())
            .returns(TypeName::primitive("void"))
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap();

        assert!(matches!(
            render_type("csharp", &type_, 100),
            Err(SigilStitchError::InvalidTypeParameter { .. })
        ));
        assert!(matches!(
            render_function("csharp", &function, 100),
            Err(SigilStitchError::InvalidFunctionTypeParameter { .. })
        ));
    }
}

#[test]
fn rust_lifetime_constraints_reject_compound_subjects() {
    let compound_lifetime =
        || TypeName::generic(TypeName::primitive("'a"), vec![TypeName::primitive("T")]);
    let type_ = TypeSpec::builder("Borrowed", TypeKind::Class)
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(compound_lifetime(), vec![TypeName::primitive("'static")])
        .build()
        .unwrap();
    let function = FunSpec::builder("borrow")
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_type_param(TypeParamSpec::new("T"))
        .add_where_constraint(compound_lifetime(), vec![TypeName::primitive("'static")])
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();

    assert!(matches!(
        render_type("rust", &type_, 100),
        Err(SigilStitchError::InvalidTypeParameter { .. })
    ));
    assert!(matches!(
        render_function("rust", &function, 100),
        Err(SigilStitchError::InvalidFunctionConstraintSubject { .. })
    ));
}

fn structured_constraint_subjects(parameter: &str) -> [TypeName; 3] {
    [
        TypeName::generic(
            TypeName::primitive(parameter),
            vec![TypeName::primitive("U")],
        ),
        TypeName::importable("alpha", parameter),
        TypeName::raw(parameter),
    ]
}

#[test]
fn inline_type_constraints_require_primitive_declared_parameter_subjects() {
    for language in BOUNDED_TYPE_LANGUAGES
        .into_iter()
        .filter(|language| *language != "rust")
    {
        let parameter = parameter_name(language, 0);
        for subject in structured_constraint_subjects(parameter) {
            let type_ = TypeSpec::builder(type_name(language), TypeKind::Class)
                .add_type_param(TypeParamSpec::new(parameter))
                .add_where_constraint(subject, vec![TypeName::primitive("Bound")])
                .add_field(
                    FieldSpec::builder("value", TypeName::primitive(parameter))
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap();
            let result = render_type(language, &type_, 100);
            assert!(
                matches!(result, Err(SigilStitchError::InvalidTypeParameter { .. })),
                "{language}: {result:?}"
            );
        }
    }
}

#[test]
fn inline_function_constraints_require_primitive_declared_parameter_subjects() {
    for language in FUNCTION_GENERIC_LANGUAGES
        .into_iter()
        .filter(|language| *language != "rust")
    {
        let parameter = parameter_name(language, 0);
        for subject in structured_constraint_subjects(parameter) {
            let function = FunSpec::builder("work")
                .add_type_param(TypeParamSpec::new(parameter))
                .add_where_constraint(subject, vec![TypeName::primitive("Bound")])
                .add_param(ParameterSpec::of("value", TypeName::primitive(parameter)))
                .returns(TypeName::primitive(parameter))
                .body(CodeBlock::of("body", ()).unwrap())
                .build()
                .unwrap();
            let result = render_function(language, &function, 100);
            assert!(
                matches!(
                    result,
                    Err(SigilStitchError::InvalidFunctionConstraintSubject { .. })
                ),
                "{language}: {result:?}"
            );
        }
    }
}

#[test]
fn duplicate_function_type_parameters_are_intrinsically_invalid() {
    let function = FunSpec::builder("work")
        .add_type_param(TypeParamSpec::new("T"))
        .add_type_param(TypeParamSpec::new("T"))
        .add_param(ParameterSpec::of("value", TypeName::primitive("T")))
        .returns(TypeName::primitive("T"))
        .body(CodeBlock::of("body", ()).unwrap())
        .build()
        .unwrap();

    for language in ["java", "typescript"] {
        let result = render_function(language, &function, 100);
        assert!(
            matches!(
                result,
                Err(SigilStitchError::DuplicateFunctionTypeParameterName {
                    ref function_name,
                    ref parameter_name,
                }) if function_name == "work" && parameter_name == "T"
            ),
            "{language}: {result:?}"
        );
    }
}

#[test]
fn strict_function_constraints_reject_empty_subjects_and_bounds() {
    for function in [
        FunSpec::builder("work")
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(TypeName::primitive(""), vec![TypeName::primitive("Bound")])
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap(),
        FunSpec::builder("work")
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(TypeName::primitive("T"), Vec::new())
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap(),
        FunSpec::builder("work")
            .add_type_param(TypeParamSpec::new("T"))
            .add_where_constraint(TypeName::primitive("T"), vec![TypeName::primitive("")])
            .body(CodeBlock::of("body", ()).unwrap())
            .build()
            .unwrap(),
    ] {
        assert_invalid_function_type_parameter("rust", render_function("rust", &function, 100));
    }
}

#[test]
fn bounded_type_parameter_matrix_uses_exact_local_grammar_or_rejects() {
    let fragment = |language| match language {
        "csharp" => "class Box<T>\n    where T : Bound",
        "dart" => "class Box<T extends Bound>",
        "go" => "type Box[T Bound] struct",
        "haskell" => "data Bound a => Box a",
        "java" => "class Box<T extends Bound>",
        "kotlin" => "class Box<T : Bound>",
        "rust" => "struct Box<T: Bound>",
        "scala" => "class Box[T <: Bound]",
        "swift" => "class Box<T: Bound>",
        "typescript" => "class Box<T extends Bound>",
        _ => unreachable!("missing bounded type expectation for {language}"),
    };

    for language in languages_registry::BUILT_IN_LANGUAGES {
        let result = render_type(language.id, &generic_type(language.id, 1, true), 100);
        if BOUNDED_TYPE_LANGUAGES.contains(&language.id) {
            let output = result.unwrap_or_else(|error| panic!("{}: {error}", language.id));
            assert!(
                output.contains(fragment(language.id)),
                "{}: {output}",
                language.id
            );
        } else if TYPE_CLASS_LANGUAGES.contains(&language.id) {
            assert!(
                matches!(
                    result,
                    Err(SigilStitchError::UnsupportedTypeCapabilities { .. })
                ),
                "{}: {result:?}",
                language.id
            );
        } else {
            assert!(
                matches!(result, Err(SigilStitchError::UnsupportedTypeKind { .. })),
                "{}: {result:?}",
                language.id
            );
        }
    }
}

#[test]
fn bounded_function_parameters_use_language_owned_grammar() {
    let expected = [
        ("csharp", "work<T>", "where T : Bound"),
        ("dart", "work<T extends Bound>", "work<T extends Bound>"),
        ("go", "work[T Bound]", "work[T Bound]"),
        ("haskell", "Bound a =>", "Bound a =>"),
        ("java", "<T extends Bound>", "<T extends Bound>"),
        ("kotlin", "<T : Bound>", "<T : Bound>"),
        ("rust", "<T: Bound>", "<T: Bound>"),
        ("scala", "[T <: Bound]", "[T <: Bound]"),
        ("swift", "<T: Bound>", "<T: Bound>"),
        ("typescript", "<T extends Bound>", "<T extends Bound>"),
    ];
    for (language, declaration, constraint) in expected {
        let output = render_function(language, &generic_function(language, 1, true), 100).unwrap();
        assert!(output.contains(declaration), "{language}: {output}");
        assert!(output.contains(constraint), "{language}: {output}");
    }
}

#[test]
fn lifetimes_kinds_context_bounds_and_where_constraints_remain_target_local() {
    let rust = TypeSpec::builder("Borrowed", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("T"))
        .add_type_param(TypeParamSpec::lifetime("'a"))
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("T"))
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("get")
                .returns(TypeName::primitive("T"))
                .body(CodeBlock::of("self.value", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = render_type("rust", &rust, 100).unwrap();
    assert!(output.contains("struct Borrowed<'a, T>"), "{output}");
    assert!(output.contains("impl<'a, T> Borrowed<'a, T>"), "{output}");

    let scala = TypeSpec::builder("Context", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("F").with_kind(TypeParamKind::Constructor1))
        .add_type_param(TypeParamSpec::new("T").with_context_bound(TypeName::primitive("Ordering")))
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("T"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = render_type("scala", &scala, 100).unwrap();
    assert!(
        output.contains("class Context[F[_], T : Ordering]"),
        "{output}"
    );

    let haskell_context = TypeSpec::builder("Context", TypeKind::Class)
        .add_type_param(TypeParamSpec::new("a").with_context_bound(TypeName::primitive("Ordering")))
        .add_field(
            FieldSpec::builder("value", TypeName::primitive("a"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = render_type("haskell", &haskell_context, 100).unwrap();
    assert!(output.contains("data Ordering a => Context a"), "{output}");

    for language in languages_registry::BUILT_IN_LANGUAGES {
        let name = type_name(language.id);
        let parameter = if language.id == "scala" {
            "F"
        } else {
            parameter_name(language.id, 0)
        };
        let hkt = TypeSpec::builder(name, TypeKind::Class)
            .add_type_param(TypeParamSpec::new(parameter).with_kind(TypeParamKind::Constructor1))
            .add_field(
                FieldSpec::builder("value", TypeName::primitive(parameter))
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let result = render_type(language.id, &hkt, 100);
        if language.id == "scala" {
            let output = result.unwrap();
            assert!(output.contains("class Box[F[_]]"), "{output}");
        } else {
            assert!(
                result.is_err(),
                "{} unexpectedly accepted a declared kind",
                language.id
            );
        }
    }

    let rust_where = FunSpec::builder("copy")
        .add_type_param(TypeParamSpec::new("T"))
        .add_param(ParameterSpec::of("value", TypeName::primitive("T")))
        .returns(TypeName::primitive("T"))
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("Clone"), TypeName::primitive("Send")],
        )
        .body(CodeBlock::of("value", ()).unwrap())
        .build()
        .unwrap();
    let output = render_function("rust", &rust_where, 100).unwrap();
    assert!(output.contains("where\n    T: Clone + Send,"), "{output}");
}

#[test]
fn imported_bound_aliases_survive_type_and_function_lowering() {
    let first = TypeName::importable("./alpha", "Constraint");
    let second = TypeName::importable("./beta", "Constraint").with_alias("SecondConstraint");
    let type_ = TypeSpec::builder("Pair", TypeKind::TypeAlias)
        .add_type_param(TypeParamSpec::new("T"))
        .add_type_param(TypeParamSpec::new("U"))
        .add_where_constraint(TypeName::primitive("T"), vec![first.clone()])
        .add_where_constraint(TypeName::primitive("U"), vec![second.clone()])
        .extends(TypeName::primitive("T"))
        .build()
        .unwrap();
    let function = FunSpec::builder("convert")
        .add_type_param(TypeParamSpec::new("T").with_bound(first))
        .add_type_param(TypeParamSpec::new("U").with_bound(second))
        .add_param(ParameterSpec::of("value", TypeName::primitive("T")))
        .returns(TypeName::primitive("U"))
        .body(CodeBlock::of("value as U", ()).unwrap())
        .build()
        .unwrap();
    let output = FileSpec::builder("bounds.ts")
        .add_type(type_)
        .add_function(function)
        .build()
        .unwrap()
        .render(100)
        .unwrap();
    assert!(
        output.contains("Constraint as SecondConstraint"),
        "{output}"
    );
    assert!(
        output.contains("Pair<T extends Constraint, U extends SecondConstraint>"),
        "{output}"
    );
    assert!(
        output.contains("convert<T extends Constraint, U extends SecondConstraint>"),
        "{output}"
    );
}

#[derive(Debug)]
struct StrictFunctionWithoutLowerer;

impl RendererLang for StrictFunctionWithoutLowerer {
    fn file_extension(&self) -> &str {
        "strict-function"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }
}

const STRICT_FUNCTIONS: &[FunctionCapabilityProfile<'_>] =
    &[
        FunctionCapabilityProfile::new(FunctionContext::TopLevel, FunctionForm::Function, &[])
            .with_body_policy(FunctionBodyPolicy::Optional),
    ];

impl CodeLang for StrictFunctionWithoutLowerer {
    fn capabilities(&self) -> LanguageCapabilities<'_> {
        LanguageCapabilities::strict().with_functions(STRICT_FUNCTIONS)
    }
}

#[test]
fn strict_adapter_without_complete_function_lowering_fails_closed() {
    let function = FunSpec::builder("work").build().unwrap();
    let error = function
        .emit(&StrictFunctionWithoutLowerer, DeclarationContext::TopLevel)
        .unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::MissingFunctionLowerer { .. }
    ));
}
