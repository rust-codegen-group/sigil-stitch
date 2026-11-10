//! Integer singleton values use the ordinary type preparation and rendering path.

use sigil_stitch::lang::{
    CodeLang, RendererLang, go::Go, python::Python, scala::Scala, typescript::TypeScript,
};
use sigil_stitch::prelude::*;

fn render_type(
    type_name: TypeName,
    language: &dyn CodeLang,
    pretty: bool,
    width: usize,
) -> Result<String, SigilStitchError> {
    CodeBlock::of(if pretty { "value:%W%T" } else { "%T" }, (type_name,))?
        .render_standalone(language, width)
}

#[test]
fn canonical_integer_values_preserve_their_text_on_both_render_paths() {
    for value in ["0", "1", "-1", "200", "-200", "9007199254740992"] {
        let type_name = TypeName::integer_literal(value);
        assert_eq!(type_name, TypeName::IntegerLiteral(value.to_owned()));
        assert_eq!(type_name.simple_name(), None);
        for (language, expected) in [
            (&TypeScript::new() as &dyn CodeLang, value.to_owned()),
            (&Python::new(), format!("Literal[{value}]")),
        ] {
            assert_eq!(
                render_type(type_name.clone(), language, false, 240).unwrap(),
                expected
            );
            for width in [8, 240] {
                let pretty = render_type(type_name.clone(), language, true, width).unwrap();
                assert_eq!(
                    pretty.split_whitespace().collect::<String>(),
                    format!("value:{expected}")
                );
            }
        }
    }
}

#[test]
fn constructors_and_json_store_text_without_normalizing_or_validating_it() {
    for value in [
        "200",
        "-1",
        "9007199254740993",
        "-0",
        "+00200",
        "not an integer",
    ] {
        let json = serde_json::json!({ "IntegerLiteral": value });
        let decoded: TypeName = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(decoded, TypeName::IntegerLiteral(value.to_owned()));
        assert_eq!(decoded, TypeName::integer_literal(value.to_owned()));
        assert_eq!(serde_json::to_value(decoded).unwrap(), json);
    }
    for payload in [
        serde_json::json!(200),
        serde_json::json!(true),
        serde_json::Value::Null,
    ] {
        assert!(
            serde_json::from_value::<TypeName>(serde_json::json!({ "IntegerLiteral": payload }))
                .is_err()
        );
    }
}

#[test]
#[expect(
    deprecated,
    reason = "released facades reject new semantic singleton forms"
)]
fn external_compatibility_lowerers_and_legacy_document_facades_reject_integer_types() {
    #[derive(Debug)]
    struct LegacyAdapter;

    impl RendererLang for LegacyAdapter {
        fn file_extension(&self) -> &str {
            "legacy"
        }

        fn line_comment_prefix(&self) -> &str {
            "//"
        }
    }
    impl CodeLang for LegacyAdapter {}

    let value = TypeName::integer_literal("200");
    let resolve = |_: &str, name: &str| name.to_owned();
    assert!(value.render(80, &resolve).is_err());
    assert!(
        value
            .to_doc_with_lang(&resolve, &TypeScript::new())
            .render(80, &mut Vec::new())
            .is_err()
    );
    assert!(matches!(
        render_type(value, &LegacyAdapter, false, 80),
        Err(SigilStitchError::UnsupportedTypeName { language, reason, .. })
            if language == "legacy" && reason.contains("does not support integer singleton types")
    ));
}

#[test]
fn malformed_integer_payloads_fail_intrinsic_validation_on_every_construction_path() {
    let ts = TypeScript::new();
    let py = Python::new();
    let scala = Scala::new();
    let go = Go::new();
    let languages: [&dyn CodeLang; 4] = [&ts, &py, &scala, &go];
    for value in [
        "",
        "-",
        "+",
        "-0",
        "+0",
        "00",
        "01",
        "-01",
        "+200",
        " 200",
        "200 ",
        "2\n00",
        "2\0",
        "0xC8",
        "2e2",
        "2_00",
        "200.0",
        "200n",
        "200L",
        "true",
        "NaN",
        "Infinity",
        "２００",
        "−1",
        "--1",
        "1; injected()",
    ] {
        let decoded =
            serde_json::from_value::<TypeName>(serde_json::json!({ "IntegerLiteral": value }))
                .unwrap();
        for type_name in [
            TypeName::integer_literal(value),
            TypeName::IntegerLiteral(value.to_owned()),
            decoded,
        ] {
            // Nested types and even unsupported targets must validate the value
            // before asking the selected adapter for a representation.
            let nested = TypeName::map(TypeName::primitive("Key"), type_name);
            for language in languages {
                for pretty in [false, true] {
                    assert!(
                        matches!(
                            render_type(nested.clone(), language, pretty, 8),
                            Err(SigilStitchError::InvalidTypeName { context, reason })
                                if context.contains("map.value")
                                    && reason.contains("canonical ASCII decimal")
                        ),
                        "{value:?} in {}",
                        language.file_extension()
                    );
                }
            }
        }
    }
}

#[test]
fn typescript_exactness_is_not_the_safe_integer_range_or_a_machine_word_bound() {
    let ts = TypeScript::new();
    for power in [53, 63, 64, 100, 127] {
        let exact = 1_u128 << power;
        for sign in ["", "-"] {
            let value = format!("{sign}{exact}");
            for pretty in [false, true] {
                let output =
                    render_type(TypeName::integer_literal(&value), &ts, pretty, 8).unwrap();
                assert_eq!(
                    output.split_whitespace().collect::<String>(),
                    format!("{}{value}", if pretty { "value:" } else { "" })
                );
                let rounded = format!("{sign}{}", exact + 1);
                assert!(matches!(
                    render_type(TypeName::integer_literal(rounded), &ts, pretty, 8),
                    Err(SigilStitchError::UnsupportedTypeName { language, context, reason })
                        if language == "ts" && context != "root" && reason.contains("exactly representable")
                ));
            }
        }
    }
}

#[test]
fn typescript_compares_the_full_integer_not_the_shortest_float_spelling() {
    let ts = TypeScript::new();
    // 10^20 is exact; binary64 rounds 10^23 to 99999999999999991611392.
    for value in ["100000000000000000000", "99999999999999991611392"] {
        assert_eq!(
            render_type(TypeName::integer_literal(value), &ts, false, 240).unwrap(),
            value
        );
    }
    for value in ["100000000000000000000000", "-100000000000000000000000"] {
        assert!(matches!(
            render_type(TypeName::integer_literal(value), &ts, false, 240),
            Err(SigilStitchError::UnsupportedTypeName { .. })
        ));
    }
}

#[test]
fn typescript_accepts_the_largest_finite_integer_but_rejects_rounding_and_overflow() {
    // Exact integer value of (2^53 - 1) * 2^971, the largest finite binary64.
    let largest = concat!(
        "179769313486231570814527423731704356798070567525844996598917476803157260780028538760589558632766878171",
        "540458953514382464234321326889464182768467546703537516986049910576551282076245490090389328944075868508",
        "455133942304583236903222948165808559332123348274797826204144723168738177180919299881250404026184124858368",
    );
    let ts = TypeScript::new();
    for sign in ["", "-"] {
        let value = format!("{sign}{largest}");
        assert_eq!(
            render_type(TypeName::integer_literal(&value), &ts, false, 240).unwrap(),
            value
        );
        let rounded = format!("{sign}{}9", &largest[..largest.len() - 1]);
        let overflow = format!("{sign}1{}", "0".repeat(400));
        for value in [rounded, overflow] {
            assert!(matches!(
                render_type(TypeName::integer_literal(value), &ts, false, 240),
                Err(SigilStitchError::UnsupportedTypeName { .. })
            ));
        }
    }
}

#[test]
fn typescript_integer_literals_compose_with_postfix_union_and_application_types() {
    let value = TypeName::tuple(vec![
        TypeName::array(TypeName::integer_literal("-1")),
        TypeName::optional(TypeName::integer_literal("0")),
        TypeName::map(
            TypeName::string_literal("status"),
            TypeName::union(vec![
                TypeName::integer_literal("200"),
                TypeName::integer_literal("201"),
            ]),
        ),
    ]);
    for width in [8, 240] {
        let output = render_type(value.clone(), &TypeScript::new(), false, width).unwrap();
        assert_eq!(
            output.split_whitespace().collect::<String>(),
            "[(-1)[],0|null,Record<'status',200|201>]"
        );
    }
}

#[test]
fn scala_integer_literals_preserve_native_int_and_long_boundaries() {
    for (value, expected) in [
        ("0", "0"),
        ("1", "1"),
        ("-1", "-1"),
        ("200", "200"),
        ("-200", "-200"),
        ("2147483647", "2147483647"),
        ("-2147483648", "-2147483648"),
        ("2147483648", "2147483648L"),
        ("-2147483649", "-2147483649L"),
        ("9007199254740993", "9007199254740993L"),
        ("9223372036854775807", "9223372036854775807L"),
        ("-9223372036854775808", "-9223372036854775808L"),
    ] {
        for pretty in [false, true] {
            for width in [8, 240] {
                let output = render_type(
                    TypeName::integer_literal(value),
                    &Scala::new(),
                    pretty,
                    width,
                )
                .unwrap();
                assert_eq!(
                    output.split_whitespace().collect::<String>(),
                    format!("{}{expected}", if pretty { "value:" } else { "" })
                );
            }
        }
    }
}

#[test]
fn scala_rejects_values_outside_its_native_integer_literal_range() {
    for value in [
        "9223372036854775808".to_owned(),
        "-9223372036854775809".to_owned(),
        u128::MAX.to_string(),
        format!("-1{}", "0".repeat(400)),
    ] {
        for type_name in [
            TypeName::integer_literal(&value),
            TypeName::array(TypeName::integer_literal(&value)),
        ] {
            for pretty in [false, true] {
                assert!(matches!(
                    render_type(type_name.clone(), &Scala::new(), pretty, 8),
                    Err(SigilStitchError::UnsupportedTypeName { language, context, reason })
                        if language == "scala" && context != "root"
                            && reason.contains("outside Scala's Int and Long literal range")
                ));
            }
        }
    }
}

#[test]
fn scala_integer_literals_compose_without_flattening_types_or_imports() {
    let value = TypeName::tuple(vec![
        TypeName::array(TypeName::union(vec![
            TypeName::integer_literal("-1"),
            TypeName::integer_literal("200"),
        ])),
        TypeName::optional(TypeName::integer_literal("-9223372036854775808")),
        TypeName::application(
            TypeName::importable("example", "Box"),
            vec![TypeArgument::Single(TypeName::integer_literal(
                "2147483648",
            ))],
        ),
        TypeName::callable(
            vec![CallableParam::Single {
                name: None,
                type_name: TypeName::integer_literal("200"),
                presence: CallableParamPresence::Required,
            }],
            vec![TypeName::integer_literal("2147483648")],
        ),
    ]);
    let file = FileSpec::builder_with("IntegerTypes.scala", Scala::new())
        .add_type(
            TypeSpec::builder("IntegerTypes", TypeKind::TypeAlias)
                .extends(value.clone())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let expected =
        "(Array[-1|200],Option[-9223372036854775808L],Box[2147483648L],(200)=>2147483648L)";
    for width in [8, 240] {
        for pretty in [false, true] {
            let output = render_type(value.clone(), &Scala::new(), pretty, width).unwrap();
            assert_eq!(
                output.split_whitespace().collect::<String>(),
                format!("{}{expected}", if pretty { "value:" } else { "" })
            );
        }
        let output = file.render(width).unwrap();
        assert_eq!(output.matches("import example.Box").count(), 1, "{output}");
        assert!(
            output
                .split_whitespace()
                .collect::<String>()
                .contains(&format!("typeIntegerTypes={expected}")),
            "{output}"
        );
    }
}

#[test]
fn python_preserves_integers_beyond_machine_words_and_binary64_range() {
    for value in [
        "9007199254740993".to_owned(),
        u128::MAX.to_string(),
        format!("-1{}", "0".repeat(400)),
    ] {
        for pretty in [false, true] {
            let output =
                render_type(TypeName::integer_literal(&value), &Python::new(), pretty, 8).unwrap();
            assert_eq!(
                output.split_whitespace().collect::<String>(),
                format!("{}Literal[{value}]", if pretty { "value:" } else { "" })
            );
        }
    }
}

#[test]
fn python_integer_and_mixed_unions_keep_order_duplicates_and_recursive_composition() {
    let value = TypeName::union(vec![
        TypeName::integer_literal("200"),
        TypeName::union(vec![
            TypeName::integer_literal("201"),
            TypeName::string_literal("error"),
        ]),
        TypeName::integer_literal("200"),
    ]);
    for width in [8, 240] {
        let output = render_type(value.clone(), &Python::new(), false, width).unwrap();
        assert_eq!(
            output.split_whitespace().collect::<String>(),
            "Literal[200]|Literal[201]|Literal['error']|Literal[200]"
        );
    }
}

#[test]
fn python_integer_target_imports_are_deduplicated_and_alias_resolved_after_lowering() {
    let block = CodeBlock::of(
        "external: %T\nstatus: %T\nitems: %T",
        (
            TypeName::importable_type("example", "Literal"),
            TypeName::integer_literal("200"),
            TypeName::array(TypeName::integer_literal("9007199254740993")),
        ),
    )
    .unwrap();
    let file = FileSpec::builder_with("literal.py", Python::new())
        .add_code(block)
        .build()
        .unwrap();
    for width in [8, 240] {
        let output = file.render(width).unwrap();
        assert!(output.contains("from example import Literal"), "{output}");
        assert_eq!(
            output
                .matches("from typing import Literal as TypingLiteral")
                .count(),
            1,
            "{output}"
        );
        let compact = output.split_whitespace().collect::<String>();
        assert!(compact.contains("status:TypingLiteral[200]"), "{output}");
        assert!(
            compact.contains("items:list[TypingLiteral[9007199254740993]]"),
            "{output}"
        );
    }
}
