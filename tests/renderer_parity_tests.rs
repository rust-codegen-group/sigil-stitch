use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_node::BlockIntent;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::import::{ImportEntry, ImportGroup};
use sigil_stitch::lang::CodeLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

#[path = "shared/languages.rs"]
mod languages_registry;

fn languages() -> impl Iterator<Item = Box<dyn CodeLang>> {
    languages_registry::BUILT_IN_LANGUAGES
        .into_iter()
        .map(|language| language.adapter())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeNameExampleKind {
    Importable,
    QualifiedImportable,
    Primitive,
    Array,
    ReadonlyArray,
    Generic,
    Union,
    Intersection,
    Pointer,
    Slice,
    Map,
    Optional,
    Tuple,
    UnitTuple,
    Reference,
    MutableReference,
    LifetimeReference,
    AssociatedType,
    QualifiedAssociatedType,
    ImplTrait,
    DynTrait,
    Wildcard,
    UpperWildcard,
    LowerWildcard,
    Function,
    Raw,
    StringLiteral,
}

struct TypeNameExample {
    kind: TypeNameExampleKind,
    label: &'static str,
    value: TypeName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedTypeName {
    Output(&'static str),
    Unsupported,
}

const EXPECTED_TYPE_NAME_LANGUAGES: [&str; 20] = [
    "bash",
    "c",
    "cpp",
    "csharp",
    "dart",
    "go",
    "haskell",
    "java",
    "javascript",
    "kotlin",
    "lua",
    "ocaml",
    "php",
    "python",
    "ruby",
    "rust",
    "scala",
    "swift",
    "typescript",
    "zsh",
];

fn output_for(language: &str, supported: &[(&str, &'static str)]) -> ExpectedTypeName {
    supported
        .iter()
        .find_map(|(id, output)| (*id == language).then_some(ExpectedTypeName::Output(output)))
        .unwrap_or(ExpectedTypeName::Unsupported)
}

fn expected_type_name(language: &str, kind: TypeNameExampleKind) -> ExpectedTypeName {
    use ExpectedTypeName::{Output, Unsupported};
    use TypeNameExampleKind::*;

    assert!(
        EXPECTED_TYPE_NAME_LANGUAGES.contains(&language),
        "add exact TypeName expectations for {language}"
    );

    match kind {
        Importable => match language {
            "go" => Output("example.module.Imported"),
            _ => Output("Imported"),
        },
        QualifiedImportable => match language {
            "cpp" | "ruby" | "rust" => Output("example.module::Qualified"),
            "php" => Output("example.module\\Qualified"),
            "bash" | "c" | "javascript" | "typescript" | "zsh" => Unsupported,
            _ => Output("example.module.Qualified"),
        },
        Primitive => Output("Value"),
        Array => output_for(
            language,
            &[
                ("cpp", "std::vector<Value>"),
                ("csharp", "List<Value>"),
                ("dart", "List<Value>"),
                ("go", "[]Value"),
                ("haskell", "[Value]"),
                ("java", "List<Value>"),
                ("kotlin", "Array<Value>"),
                ("ocaml", "Value array"),
                ("python", "list[Value]"),
                ("rust", "Vec<Value>"),
                ("scala", "Array[Value]"),
                ("swift", "[Value]"),
                ("typescript", "Value[]"),
            ],
        ),
        ReadonlyArray => output_for(
            language,
            &[
                ("cpp", "const std::vector<Value>"),
                ("csharp", "IReadOnlyList<Value>"),
                ("haskell", "[Value]"),
                ("kotlin", "List<Value>"),
                ("ocaml", "Value list"),
                ("scala", "IArray[Value]"),
                ("typescript", "readonly Value[]"),
            ],
        ),
        Generic => output_for(
            language,
            &[
                ("cpp", "Value<Item>"),
                ("csharp", "Value<Item>"),
                ("dart", "Value<Item>"),
                ("go", "Value[Item]"),
                ("haskell", "Value Item"),
                ("java", "Value<Item>"),
                ("kotlin", "Value<Item>"),
                ("ocaml", "Item Value"),
                ("python", "Value[Item]"),
                ("rust", "Value<Item>"),
                ("scala", "Value[Item]"),
                ("swift", "Value<Item>"),
                ("typescript", "Value<Item>"),
            ],
        ),
        Union => output_for(
            language,
            &[
                ("php", "Value | Other"),
                ("python", "Value | Other"),
                ("scala", "Value | Other"),
                ("typescript", "Value | Other"),
            ],
        ),
        Intersection => output_for(
            language,
            &[
                ("php", "Value & Other"),
                ("scala", "Value & Other"),
                ("swift", "Value & Other"),
                ("typescript", "Value & Other"),
            ],
        ),
        Pointer => output_for(
            language,
            &[
                ("c", "Value*"),
                ("cpp", "Value*"),
                ("csharp", "Value*"),
                ("go", "*Value"),
                ("rust", "*const Value"),
            ],
        ),
        Slice => output_for(language, &[("go", "[]Value"), ("rust", "&[Value]")]),
        Map => output_for(
            language,
            &[
                ("cpp", "std::map<Key, Value>"),
                ("dart", "Map<Key, Value>"),
                ("go", "map[Key]Value"),
                ("haskell", "Map Key Value"),
                ("java", "Map<Key, Value>"),
                ("kotlin", "Map<Key, Value>"),
                ("ocaml", "(Key, Value) Hashtbl.t"),
                ("python", "dict[Key, Value]"),
                ("rust", "HashMap<Key, Value>"),
                ("scala", "Map[Key, Value]"),
                ("swift", "[Key: Value]"),
                ("typescript", "Record<Key, Value>"),
            ],
        ),
        Optional => output_for(
            language,
            &[
                ("c", "Value*"),
                ("cpp", "std::optional<Value>"),
                ("csharp", "Value?"),
                ("dart", "Value?"),
                ("go", "*Value"),
                ("haskell", "Maybe Value"),
                ("java", "Optional<Value>"),
                ("kotlin", "Value?"),
                ("ocaml", "Value option"),
                ("php", "?Value"),
                ("python", "Value | None"),
                ("rust", "Option<Value>"),
                ("scala", "Option[Value]"),
                ("swift", "Value?"),
                ("typescript", "Value | null"),
            ],
        ),
        Tuple => output_for(
            language,
            &[
                ("cpp", "std::tuple<Value, Other>"),
                ("csharp", "(Value, Other)"),
                ("dart", "(Value, Other)"),
                ("haskell", "(Value, Other)"),
                ("ocaml", "Value * Other"),
                ("python", "tuple[Value, Other]"),
                ("rust", "(Value, Other)"),
                ("scala", "(Value, Other)"),
                ("swift", "(Value, Other)"),
                ("typescript", "[Value, Other]"),
            ],
        ),
        UnitTuple => output_for(
            language,
            &[
                ("cpp", "std::tuple<>"),
                ("dart", "()"),
                ("haskell", "()"),
                ("ocaml", "unit"),
                ("python", "tuple[()]"),
                ("rust", "()"),
                ("scala", "Unit"),
                ("swift", "()"),
                ("typescript", "[]"),
            ],
        ),
        Reference => output_for(
            language,
            &[
                ("c", "const Value*"),
                ("cpp", "const Value&"),
                ("rust", "&Value"),
            ],
        ),
        MutableReference => output_for(
            language,
            &[
                ("c", "Value*"),
                ("cpp", "Value&"),
                ("ocaml", "Value ref"),
                ("rust", "&mut Value"),
            ],
        ),
        LifetimeReference => output_for(language, &[("rust", "&'a Value")]),
        AssociatedType => output_for(
            language,
            &[
                ("cpp", "Value::Item"),
                ("csharp", "Value.Item"),
                ("java", "Value.Item"),
                ("kotlin", "Value.Item"),
                ("python", "Value.Item"),
                ("rust", "Value::Item"),
                ("scala", "Value.Item"),
                ("swift", "Value.Item"),
                ("typescript", "Value['Item']"),
            ],
        ),
        QualifiedAssociatedType => output_for(language, &[("rust", "<Value as Iterable>::Item")]),
        ImplTrait => output_for(
            language,
            &[
                ("rust", "impl Value + Other"),
                ("swift", "some Value & Other"),
            ],
        ),
        DynTrait => output_for(
            language,
            &[
                ("rust", "dyn Value + Other"),
                ("swift", "any Value & Other"),
            ],
        ),
        Wildcard => output_for(
            language,
            &[
                ("go", "any"),
                ("java", "?"),
                ("kotlin", "*"),
                ("rust", "_"),
                ("scala", "?"),
            ],
        ),
        UpperWildcard => output_for(
            language,
            &[
                ("java", "? extends Value"),
                ("kotlin", "out Value"),
                ("scala", "? <: Value"),
            ],
        ),
        LowerWildcard => output_for(
            language,
            &[
                ("java", "? super Value"),
                ("kotlin", "in Value"),
                ("scala", "? >: Value"),
            ],
        ),
        Function => output_for(
            language,
            &[
                ("cpp", "std::function<Result(Value, Other)>"),
                ("dart", "Result Function(Value, Other)"),
                ("go", "func(Value, Other) Result"),
                ("haskell", "Value -> Other -> Result"),
                ("kotlin", "(Value, Other) -> Result"),
                ("ocaml", "Value -> Other -> Result"),
                ("python", "Callable[[Value, Other], Result]"),
                ("rust", "fn(Value, Other) -> Result"),
                ("scala", "(Value, Other) => Result"),
                ("swift", "(Value, Other) -> Result"),
                ("typescript", "(arg0: Value, arg1: Other) => Result"),
            ],
        ),
        Raw => Output("TargetSpecific"),
        StringLiteral => output_for(
            language,
            &[("python", "Literal['active']"), ("typescript", "'active'")],
        ),
    }
}

fn every_existing_type_name_variant() -> Vec<TypeNameExample> {
    use TypeNameExampleKind::*;

    let value = || TypeName::primitive("Value");
    vec![
        TypeNameExample {
            kind: Importable,
            label: "importable",
            value: TypeName::importable_type("example.module", "Imported"),
        },
        TypeNameExample {
            kind: QualifiedImportable,
            label: "qualified importable",
            value: TypeName::qualified("example.module", "Qualified"),
        },
        TypeNameExample {
            kind: Primitive,
            label: "primitive",
            value: value(),
        },
        TypeNameExample {
            kind: Array,
            label: "array",
            value: TypeName::array(value()),
        },
        TypeNameExample {
            kind: ReadonlyArray,
            label: "readonly array",
            value: TypeName::readonly_array(value()),
        },
        TypeNameExample {
            kind: Generic,
            label: "generic",
            value: TypeName::generic(value(), vec![TypeName::primitive("Item")]),
        },
        TypeNameExample {
            kind: Union,
            label: "union",
            value: TypeName::union(vec![value(), TypeName::primitive("Other")]),
        },
        TypeNameExample {
            kind: Intersection,
            label: "intersection",
            value: TypeName::intersection(vec![value(), TypeName::primitive("Other")]),
        },
        TypeNameExample {
            kind: Pointer,
            label: "pointer",
            value: TypeName::pointer(value()),
        },
        TypeNameExample {
            kind: Slice,
            label: "slice",
            value: TypeName::slice(value()),
        },
        TypeNameExample {
            kind: Map,
            label: "map",
            value: TypeName::map(TypeName::primitive("Key"), value()),
        },
        TypeNameExample {
            kind: Optional,
            label: "optional",
            value: TypeName::optional(value()),
        },
        TypeNameExample {
            kind: Tuple,
            label: "tuple",
            value: TypeName::tuple(vec![value(), TypeName::primitive("Other")]),
        },
        TypeNameExample {
            kind: UnitTuple,
            label: "unit tuple",
            value: TypeName::unit(),
        },
        TypeNameExample {
            kind: Reference,
            label: "reference",
            value: TypeName::reference(value()),
        },
        TypeNameExample {
            kind: MutableReference,
            label: "mutable reference",
            value: TypeName::reference_mut(value()),
        },
        TypeNameExample {
            kind: LifetimeReference,
            label: "lifetime reference",
            value: TypeName::reference_with_lifetime(value(), "'a"),
        },
        TypeNameExample {
            kind: AssociatedType,
            label: "associated type",
            value: TypeName::associated_type(value(), None, "Item"),
        },
        TypeNameExample {
            kind: QualifiedAssociatedType,
            label: "qualified associated type",
            value: TypeName::associated_type(
                value(),
                Some(TypeName::primitive("Iterable")),
                "Item",
            ),
        },
        TypeNameExample {
            kind: ImplTrait,
            label: "impl trait",
            value: TypeName::impl_trait(vec![value(), TypeName::primitive("Other")]),
        },
        TypeNameExample {
            kind: DynTrait,
            label: "dyn trait",
            value: TypeName::dyn_trait(vec![value(), TypeName::primitive("Other")]),
        },
        TypeNameExample {
            kind: Wildcard,
            label: "wildcard",
            value: TypeName::wildcard(),
        },
        TypeNameExample {
            kind: UpperWildcard,
            label: "upper wildcard",
            value: TypeName::wildcard_extends(value()),
        },
        TypeNameExample {
            kind: LowerWildcard,
            label: "lower wildcard",
            value: TypeName::wildcard_super(value()),
        },
        TypeNameExample {
            kind: Function,
            label: "function",
            value: TypeName::function(
                vec![value(), TypeName::primitive("Other")],
                TypeName::primitive("Result"),
            ),
        },
        TypeNameExample {
            kind: Raw,
            label: "raw",
            value: TypeName::raw("TargetSpecific"),
        },
        TypeNameExample {
            kind: StringLiteral,
            label: "string literal",
            value: TypeName::string_literal("active"),
        },
    ]
}

#[test]
fn every_builtin_decides_every_existing_type_name_variant_on_both_adapter_paths() {
    assert_eq!(
        languages_registry::BUILT_IN_LANGUAGES.map(|language| language.id),
        EXPECTED_TYPE_NAME_LANGUAGES,
        "the exact TypeName table must name every built-in language"
    );

    for descriptor in languages_registry::BUILT_IN_LANGUAGES {
        let language = descriptor.adapter();
        for example in every_existing_type_name_variant() {
            let expected = expected_type_name(descriptor.id, example.kind);
            let direct = CodeBlock::of("%T", (example.value.clone(),)).unwrap();
            let pretty = CodeBlock::of("type:%W%T", (example.value,)).unwrap();
            let direct_result = direct.render_standalone(language.as_ref(), 240);
            let wide_pretty_result = pretty.render_standalone(language.as_ref(), 240);
            let narrow_pretty_result = pretty.render_standalone(language.as_ref(), 8);

            match (
                expected,
                direct_result,
                wide_pretty_result,
                narrow_pretty_result,
            ) {
                (
                    ExpectedTypeName::Output(expected),
                    Ok(direct_output),
                    Ok(wide_pretty_output),
                    Ok(narrow_pretty_output),
                ) => {
                    assert_eq!(
                        direct_output, expected,
                        "{} emitted the wrong exact {} grammar",
                        descriptor.id, example.label
                    );
                    assert_eq!(
                        wide_pretty_output,
                        format!("type: {expected}"),
                        "{} changed {} on the wide pretty path",
                        descriptor.id,
                        example.label
                    );
                    assert!(
                        narrow_pretty_output.starts_with("type:"),
                        "{} lost the {} pretty-layout prefix: {narrow_pretty_output:?}",
                        descriptor.id,
                        example.label
                    );
                }
                (
                    ExpectedTypeName::Unsupported,
                    Err(SigilStitchError::UnsupportedTypeName {
                        reason: direct_reason,
                        ..
                    }),
                    Err(SigilStitchError::UnsupportedTypeName {
                        reason: wide_reason,
                        ..
                    }),
                    Err(SigilStitchError::UnsupportedTypeName {
                        reason: narrow_reason,
                        ..
                    }),
                ) => {
                    assert!(!direct_reason.trim().is_empty());
                    assert_eq!(wide_reason, direct_reason);
                    assert_eq!(narrow_reason, direct_reason);
                }
                (expected, direct, wide, narrow) => panic!(
                    "{} disagreed with the exact {} expectation {expected:?}:\ndirect: {direct:?}\nwide pretty: {wide:?}\nnarrow pretty: {narrow:?}",
                    descriptor.id, example.label
                ),
            }
        }
    }
}

#[test]
fn primitive_type_spellings_reject_unicode_line_separators_while_raw_preserves_them() {
    let language = sigil_stitch::lang::typescript::TypeScript::new();

    for separator in ['\u{2028}', '\u{2029}'] {
        let spelling = format!("Before{separator}After");
        let primitive = CodeBlock::of("%T", (TypeName::primitive(&spelling),)).unwrap();
        assert!(matches!(
            primitive.render_standalone(&language, 80),
            Err(SigilStitchError::InvalidTypeName { reason, .. })
                if reason.contains("line break")
        ));

        let raw = CodeBlock::of("%T", (TypeName::raw(&spelling),)).unwrap();
        assert_eq!(raw.render_standalone(&language, 80).unwrap(), spelling);
    }
}

#[test]
fn every_builtin_validates_resolved_aliases_against_its_import_form() {
    const ALIAS_CAPABLE: &[&str] = &[
        "go",
        "haskell",
        "javascript",
        "lua",
        "php",
        "python",
        "rust",
        "typescript",
    ];
    const SIDE_EFFECT_UNSUPPORTED: &[&str] = &[
        "csharp", "haskell", "java", "kotlin", "ocaml", "php", "scala",
    ];
    const WILDCARD_UNSUPPORTED: &[&str] = &["lua", "php"];

    for descriptor in languages_registry::BUILT_IN_LANGUAGES {
        let language = descriptor.adapter();
        let valid_alias = ImportGroup::from(vec![ImportEntry {
            module: "example.module".to_string(),
            name: "Imported".to_string(),
            alias: Some("ValidAlias".to_string()),
            is_type_only: true,
            is_side_effect: false,
            is_wildcard: false,
        }]);
        assert_eq!(
            language.validate_resolved_imports(&valid_alias).is_ok(),
            ALIAS_CAPABLE.contains(&descriptor.id),
            ".{} resolved-import validation disagrees with its import form",
            descriptor.extension
        );

        let invalid_alias = ImportGroup::from(vec![ImportEntry {
            alias: Some("not-valid".to_string()),
            ..valid_alias.entries()[0].clone()
        }]);
        assert!(
            language.validate_resolved_imports(&invalid_alias).is_err(),
            ".{} accepted an invalid resolved alias",
            descriptor.extension
        );

        let dollar_alias = ImportGroup::from(vec![ImportEntry {
            alias: Some("$Alias".to_string()),
            ..valid_alias.entries()[0].clone()
        }]);
        assert_eq!(
            language.validate_resolved_imports(&dollar_alias).is_ok(),
            matches!(descriptor.id, "javascript" | "typescript"),
            ".{} applied another target's dollar-identifier grammar",
            descriptor.extension
        );

        let reserved_alias = ImportGroup::from(vec![ImportEntry {
            alias: Some("if".to_string()),
            ..valid_alias.entries()[0].clone()
        }]);
        assert!(
            language.validate_resolved_imports(&reserved_alias).is_err(),
            ".{} accepted a reserved resolved alias",
            descriptor.extension
        );

        let side_effect = ImportGroup::from(vec![ImportEntry {
            module: "example.module".to_string(),
            name: String::new(),
            alias: None,
            is_type_only: false,
            is_side_effect: true,
            is_wildcard: false,
        }]);
        assert_eq!(
            language.validate_resolved_imports(&side_effect).is_err(),
            SIDE_EFFECT_UNSUPPORTED.contains(&descriptor.id),
            ".{} side-effect import validation disagrees with its import form",
            descriptor.extension
        );

        let wildcard = ImportGroup::from(vec![ImportEntry {
            is_side_effect: false,
            is_wildcard: true,
            ..side_effect.entries()[0].clone()
        }]);
        assert_eq!(
            language.validate_resolved_imports(&wildcard).is_err(),
            WILDCARD_UNSUPPORTED.contains(&descriptor.id),
            ".{} wildcard import validation disagrees with its import form",
            descriptor.extension
        );
    }
}

#[test]
#[expect(deprecated, reason = "0.6.8 renderer compatibility bridge")]
fn direct_and_pretty_adapters_preserve_all_language_indentation() {
    let direct = CodeBlock::of("call(alpha, beta)\n%>body%<", ()).unwrap();
    let pretty = CodeBlock::of("call(%>alpha,%Wbeta%<)\n%>body%<", ()).unwrap();

    for lang in languages() {
        let wide_direct = direct.render_standalone(lang.as_ref(), 120).unwrap();
        let wide_pretty = pretty.render_standalone(lang.as_ref(), 120).unwrap();
        assert_eq!(
            wide_pretty,
            wide_direct,
            "wide adapter parity failed for .{}",
            lang.file_extension()
        );

        let narrow = pretty.render_standalone(lang.as_ref(), 8).unwrap();
        let indent = lang.block_syntax().indent_unit;
        assert!(
            narrow.contains(&format!("\n{indent}beta)")),
            "wrapped argument lost indentation for .{}:\n{narrow}",
            lang.file_extension()
        );
        assert!(
            narrow.contains(&format!("\n{indent}body")),
            "body lost indentation for .{}:\n{narrow}",
            lang.file_extension()
        );
    }
}

#[test]
fn function_parameter_builder_indents_wrapped_continuations() {
    let function = FunSpec::builder("processRequest")
        .add_param(ParameterSpec::of(
            "configuration",
            TypeName::primitive("Configuration"),
        ))
        .add_param(ParameterSpec::of(
            "requestContext",
            TypeName::primitive("RequestContext"),
        ))
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    let output = FileSpec::builder_with(
        "function.ts",
        sigil_stitch::lang::typescript::TypeScript::new(),
    )
    .add_function(function)
    .build()
    .unwrap()
    .render(32)
    .unwrap();

    assert!(output.contains("\n  requestContext: RequestContext"));
    assert!(!output.contains("\nrequestContext"));
}

#[test]
fn primary_constructor_builder_indents_wrapped_continuations() {
    let type_spec = TypeSpec::builder("RequestConfiguration", TypeKind::Class)
        .add_primary_constructor_param(
            ParameterSpec::builder("endpointAddress", TypeName::primitive("String"))
                .is_property()
                .build()
                .unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::builder("requestTimeout", TypeName::primitive("Duration"))
                .is_property()
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let output = FileSpec::builder_with(
        "RequestConfiguration.kt",
        sigil_stitch::lang::kotlin::Kotlin::new(),
    )
    .add_type(type_spec)
    .build()
    .unwrap()
    .render(36)
    .unwrap();

    assert!(
        output.contains("\n    val requestTimeout: Duration"),
        "{output}"
    );
    assert!(!output.contains("\nval requestTimeout"));
}

#[test]
fn language_rewrites_run_before_the_pretty_adapter() {
    let mut cpp = CodeBlock::builder();
    cpp.begin_control_flow("auto fn = [&](int x)", ());
    cpp.add_statement("return x * 2", ());
    cpp.end_control_flow();
    cpp.add("%Wnext", ());
    let cpp = cpp.build().unwrap();
    let cpp_output = cpp
        .render_standalone(&sigil_stitch::lang::cpp::Cpp::new(), 120)
        .unwrap();
    assert!(cpp_output.contains("};"), "{cpp_output}");
    assert!(cpp_output.ends_with(" next"), "{cpp_output}");

    let go = CodeBlock::of("value := <- channel%Wnext", ()).unwrap();
    let go_output = go
        .render_standalone(&sigil_stitch::lang::go::Go::new(), 120)
        .unwrap();
    assert!(go_output.contains("value := <-channel next"), "{go_output}");

    let haskell = CodeBlock::of("apply $value%Wnext", ()).unwrap();
    let haskell_output = haskell
        .render_standalone(&sigil_stitch::lang::haskell::Haskell::new(), 120)
        .unwrap();
    assert!(
        haskell_output.contains("apply $ value next"),
        "{haskell_output}"
    );

    let lua = CodeBlock::of("object: method()%Wnext", ()).unwrap();
    let lua_output = lua
        .render_standalone(&sigil_stitch::lang::lua::Lua::new(), 120)
        .unwrap();
    assert!(lua_output.contains("object:method() next"), "{lua_output}");
}

fn if_intent_block(soft_break: bool) -> CodeBlock {
    let mut block = CodeBlock::builder();
    block.begin_control_flow_with_intent(BlockIntent::If, "if (x > 0)", ());
    if soft_break {
        block.add_statement("call(alpha,%Wbeta)", ());
    } else {
        block.add_statement("call(alpha, beta)", ());
    }
    block.end_control_flow();
    block.build().unwrap()
}

#[test]
#[expect(deprecated, reason = "0.6.8 renderer compatibility bridge")]
fn intent_blocks_match_across_direct_and_pretty_adapters() {
    let direct = if_intent_block(false);
    let pretty = if_intent_block(true);

    for lang in languages() {
        let wide_direct = direct.render_standalone(lang.as_ref(), 240).unwrap();
        let wide_pretty = pretty.render_standalone(lang.as_ref(), 240).unwrap();
        assert_eq!(
            wide_pretty,
            wide_direct,
            "wide adapter parity failed for .{}",
            lang.file_extension()
        );

        let narrow = pretty.render_standalone(lang.as_ref(), 8).unwrap();
        let indent = lang.block_syntax().indent_unit;
        assert!(
            narrow.contains(&format!("\n{indent}beta)")),
            "wrapped intent block body lost indentation for .{}:\n{narrow}",
            lang.file_extension()
        );
    }
}

#[test]
fn block_intent_negative_near_matches_pass_on_the_pretty_adapter() {
    let mut cpp = CodeBlock::builder();
    cpp.begin_control_flow("if (matrix[0] > 0)", ());
    cpp.add_statement("return matrix[0]", ());
    cpp.end_control_flow();
    cpp.add("%Wnext", ());
    let cpp = cpp.build().unwrap();
    let cpp_output = cpp
        .render_standalone(&sigil_stitch::lang::cpp::Cpp::new(), 120)
        .unwrap();
    assert!(!cpp_output.contains("};"), "{cpp_output}");
    assert!(cpp_output.ends_with(" next"), "{cpp_output}");

    let mut go = CodeBlock::builder();
    go.begin_control_flow("if fn.String() != \"func\"", ());
    go.end_control_flow();
    go.add_statement("(\"ordinary\")", ());
    go.add("%Wnext", ());
    let go = go.build().unwrap();
    let go_output = go
        .render_standalone(&sigil_stitch::lang::go::Go::new(), 120)
        .unwrap();
    assert!(go_output.contains("}\n(\"ordinary\")"), "{go_output}");
    assert!(!go_output.contains("}(\"ordinary\")"), "{go_output}");
}
