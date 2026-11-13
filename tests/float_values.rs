//! Exact floating values compose through the existing structured source interface.

use sigil_stitch::lang::rust::Rust;
use sigil_stitch::prelude::*;
use sigil_stitch::{code_renderer::CodeRenderer, import::ImportGroup};

#[path = "shared/languages.rs"]
mod languages_registry;

fn render_value(
    value: FloatValue,
    language: &dyn RendererLang,
) -> Result<String, SigilStitchError> {
    let block = CodeBlock::of("%L", value)?;
    CodeRenderer::new(language, &ImportGroup::new(), 80).render(&block)
}

#[test]
fn rust_constructs_values_from_their_original_format_and_bits() {
    for (value, encoded, expected) in [
        (
            FloatValue::from_f32(-0.0),
            FloatValue::Binary32(0x8000_0000),
            "let gain = ::core::primitive::f32::from_bits(0x80000000u32);",
        ),
        (
            FloatValue::from_f64(1.0),
            FloatValue::Binary64(0x3ff0_0000_0000_0000),
            "let gain = ::core::primitive::f64::from_bits(0x3ff0000000000000u64);",
        ),
    ] {
        assert_eq!(value, encoded);
        let block = CodeBlock::of("let gain = %L;", value).unwrap();
        assert_eq!(block.render_standalone(&Rust::new(), 80).unwrap(), expected);
    }
}

#[derive(Debug)]
struct SuppliedOutput(CodeBlock);

impl RendererLang for SuppliedOutput {
    fn file_extension(&self) -> &str {
        "custom"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn lower_float_value(&self, _: &FloatValue) -> Result<CodeBlock, SigilStitchError> {
        Ok(self.0.clone())
    }

    fn lower_type_name(&self, value: &TypeName) -> Result<CodeBlock, SigilStitchError> {
        Rust::new().lower_type_name(value)
    }
}

impl CodeLang for SuppliedOutput {
    fn render_imports(&self, imports: &sigil_stitch::import::ImportGroup) -> String {
        Rust::new().render_imports(imports)
    }
}

#[test]
fn invalid_adapter_output_fails_instead_of_dropping_or_corrupting_the_expression() {
    let empty = CodeBlock::builder().build().unwrap();
    let nested_empty = CodeBlock::of("%L", empty.clone()).unwrap();
    let mut unbalanced = CodeBlock::of("value", ()).unwrap();
    unbalanced.nodes_mut().push(CodeNode::Dedent);
    let mut unresolved = CodeBlock::of("value", ()).unwrap();
    unresolved
        .nodes_mut()
        .push(CodeNode::InlineLiteral("%>".to_owned()));

    for output in [empty, nested_empty, unbalanced, unresolved] {
        let language = SuppliedOutput(output);
        let error = render_value(FloatValue::Binary64(0), &language).unwrap_err();
        assert!(error.to_string().contains("invalid floating-value output"));
        assert!(matches!(
            error,
            SigilStitchError::InvalidFloatValueLowering { language, reason }
                if language == "custom" && !reason.is_empty()
        ));
    }
}

#[test]
fn layout_only_float_lowering_fails_in_standalone_and_file_rendering() {
    for format in ["%>%<", "%W", "\n", "%[%]", "%> \t\n%W%<"] {
        let layout = CodeBlock::of(format, ()).unwrap();
        let nested = CodeBlock::of("%L", layout.clone()).unwrap();
        let mut sequence = nested.clone();
        let children = std::mem::take(sequence.nodes_mut());
        sequence.nodes_mut().push(CodeNode::Sequence(children));

        for output in [layout, nested, sequence] {
            for source in ["let gain = %L;", "%Wlet gain = %L;"] {
                let block = CodeBlock::of(source, FloatValue::Binary64(0)).unwrap();
                let standalone = block.render_standalone(&SuppliedOutput(output.clone()), 80);
                let file = FileSpec::builder_with("value.custom", SuppliedOutput(output.clone()))
                    .add_code(block)
                    .build()
                    .unwrap()
                    .render(80);

                for result in [standalone, file] {
                    assert!(
                        matches!(
                            &result,
                            Err(SigilStitchError::InvalidFloatValueLowering { language, reason })
                                if language == "custom" && !reason.is_empty()
                        ),
                        "layout {format:?} in {source:?} unexpectedly returned {result:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn identity_and_same_version_round_trips_preserve_format_and_nan_payloads() {
    let values = [
        FloatValue::Binary32(0),
        FloatValue::Binary32(0x8000_0000),
        FloatValue::Binary32(0x7fc0_0000),
        FloatValue::Binary32(0x7fc0_0001),
        FloatValue::Binary32(0x7f80_0001),
        FloatValue::Binary64(0),
        FloatValue::Binary64(0x8000_0000_0000_0000),
        FloatValue::Binary64(0x7ff8_0000_0000_0000),
        FloatValue::Binary64(0x7ff8_0000_0000_0001),
        FloatValue::Binary64(0x7ff0_0000_0000_0001),
    ];
    assert_eq!(
        values
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        values.len()
    );
    for value in values {
        let decoded: FloatValue =
            serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();
        assert_eq!(decoded, value);
        let captured = match value {
            FloatValue::Binary32(bits) => FloatValue::from_f32(f32::from_bits(bits)),
            FloatValue::Binary64(bits) => FloatValue::from_f64(f64::from_bits(bits)),
            _ => unreachable!(),
        };
        assert_eq!(captured, value);
        let block = CodeBlock::of("%L", value).unwrap();
        let decoded: CodeBlock =
            serde_json::from_str(&serde_json::to_string(&block).unwrap()).unwrap();
        assert_eq!(
            decoded.render_standalone(&Rust::new(), 80).unwrap(),
            block.render_standalone(&Rust::new(), 80).unwrap()
        );
    }
}

#[test]
fn every_other_builtin_rejects_exact_float_construction_without_a_fallback() {
    for registered in languages_registry::BUILT_IN_LANGUAGES {
        let language = registered.adapter();
        for value in [
            FloatValue::Binary32(0),
            FloatValue::Binary64(0x7ff8_0000_0000_0042),
        ] {
            for pretty in [false, true] {
                let block = CodeBlock::of(if pretty { "%W%L" } else { "%L" }, value).unwrap();
                let result = block.render_standalone(language.as_ref(), 80);
                if registered.id == "rust" {
                    assert!(result.is_ok());
                } else {
                    assert!(
                        matches!(
                            result,
                            Err(SigilStitchError::UnsupportedFloatValue {
                                language,
                                value: rejected,
                                reason,
                            }) if language == registered.extension && rejected == value && !reason.is_empty()
                        ),
                        "{} unexpectedly accepted {value:?}",
                        registered.id
                    );
                }
            }
        }
    }
}

#[test]
fn unchanged_external_adapters_reject_and_explicit_hook_errors_propagate() {
    #[derive(Debug)]
    struct Unchanged;
    impl RendererLang for Unchanged {
        fn file_extension(&self) -> &str {
            "unchanged"
        }
        fn line_comment_prefix(&self) -> &str {
            "//"
        }
    }
    let error = render_value(FloatValue::from_f32(1.0), &Unchanged).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("unchanged") && message.contains("Binary32"));
    assert!(matches!(error,
        SigilStitchError::UnsupportedFloatValue { language, .. } if language == "unchanged"
    ));

    #[derive(Debug)]
    struct Failing;
    impl RendererLang for Failing {
        fn file_extension(&self) -> &str {
            "failing"
        }
        fn line_comment_prefix(&self) -> &str {
            "//"
        }
        fn lower_float_value(&self, _: &FloatValue) -> Result<CodeBlock, SigilStitchError> {
            Err(SigilStitchError::Render {
                context: "custom float construction".to_string(),
                message: "original failure".to_string(),
            })
        }
    }
    assert!(matches!(
        render_value(FloatValue::from_f64(1.0), &Failing),
        Err(SigilStitchError::Render { context, message })
            if context == "custom float construction" && message == "original failure"
    ));
}

#[test]
fn nested_values_render_through_direct_pretty_and_sequence_groups() {
    let rust = Rust::new();
    let expected = "pair(::core::primitive::f32::from_bits(0x00000001u32),::core::primitive::f64::from_bits(0x7ff8000000000042u64))";
    for pretty in [false, true] {
        let format = if pretty {
            "pair(%>%W%L,%W%L%<%W)"
        } else {
            "pair(%L,%L)"
        };
        let block = CodeBlock::of(
            format,
            (
                FloatValue::Binary32(1),
                FloatValue::Binary64(0x7ff8_0000_0000_0042),
            ),
        )
        .unwrap();
        for sequence in [false, true] {
            let mut composed = block.clone();
            if sequence {
                let nodes = std::mem::take(composed.nodes_mut());
                composed.nodes_mut().push(CodeNode::Sequence(nodes));
            }
            for width in [16, 240] {
                let output = composed.render_standalone(&rust, width).unwrap();
                assert_eq!(output.split_whitespace().collect::<String>(), expected);
            }
        }
    }
}

#[test]
fn custom_float_lowering_keeps_type_references_for_file_alias_resolution() {
    let language = SuppliedOutput(
        CodeBlock::of(
            "%T::from_bits(0x3f800000u32)",
            TypeName::importable("std::primitive", "f32").with_alias("ExactF32"),
        )
        .unwrap(),
    );
    let body = CodeBlock::of("let gain = %L;", FloatValue::from_f32(1.0)).unwrap();
    let file = FileSpec::builder_with("value.rs", language)
        .add_code(body)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(
        output.contains("use std::primitive::f32 as ExactF32;"),
        "{output}"
    );
    assert!(
        output.contains("let gain = ExactF32::from_bits(0x3f800000u32);"),
        "{output}"
    );
}

#[test]
fn quoted_rust_retains_values_until_the_render_language_is_selected() {
    fn build() -> Result<CodeBlock, SigilStitchError> {
        sigil_quote!(Rust {
            const GAIN: $T(TypeName::primitive("f64")) = $L(FloatValue::from_f64(1.0));
        })
    }
    let output = build()
        .unwrap()
        .render_standalone(&Rust::new(), 80)
        .unwrap();
    assert_eq!(
        output,
        "const GAIN: f64 = ::core::primitive::f64::from_bits(0x3ff0000000000000u64);\n"
    );
}

#[test]
fn float_lowering_must_return_an_expression_without_another_deferred_value() {
    #[derive(Debug)]
    struct DeferredOutput(CodeBlock);

    impl RendererLang for DeferredOutput {
        fn file_extension(&self) -> &str {
            "deferred"
        }

        fn line_comment_prefix(&self) -> &str {
            "//"
        }

        fn lower_float_value(&self, value: &FloatValue) -> Result<CodeBlock, SigilStitchError> {
            match value {
                FloatValue::Binary32(_) => Ok(self.0.clone()),
                _ => CodeBlock::of("0.0f64", ()),
            }
        }
    }

    let mut direct = CodeBlock::builder().build().unwrap();
    direct
        .nodes_mut()
        .push(CodeNode::FloatValue(FloatValue::Binary64(0)));
    let nested = CodeBlock::of("%L", direct.clone()).unwrap();
    let mut sequence = CodeBlock::builder().build().unwrap();
    sequence
        .nodes_mut()
        .push(CodeNode::Sequence(direct.nodes_mut().clone()));

    for output in [direct, nested, sequence] {
        assert!(matches!(
            render_value(FloatValue::Binary32(0), &DeferredOutput(output)),
            Err(SigilStitchError::InvalidFloatValueLowering { language, .. })
                if language == "deferred"
        ));
    }
}

#[test]
fn one_source_uses_the_selected_adapter_without_binding_or_mutating_the_value() {
    let block = CodeBlock::of("%L", FloatValue::Binary32(0x8000_0000)).unwrap();
    let original = serde_json::to_string(&block).unwrap();
    let custom = SuppliedOutput(
        CodeBlock::of("::core::primitive::f32::from_bits(2147483648u32)", ()).unwrap(),
    );

    assert_eq!(
        block.render_standalone(&custom, 80).unwrap(),
        "::core::primitive::f32::from_bits(2147483648u32)"
    );
    assert_eq!(
        block.render_standalone(&Rust::new(), 80).unwrap(),
        "::core::primitive::f32::from_bits(0x80000000u32)"
    );
    let unsupported = FileSpec::builder_with("value.go", sigil_stitch::lang::go::Go::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    assert!(matches!(
        unsupported.render(80),
        Err(SigilStitchError::UnsupportedFloatValue { .. })
    ));
    assert_eq!(serde_json::to_string(&block).unwrap(), original);
}

#[test]
fn literal_template_parameters_and_fragments_preserve_deferred_values() {
    let value = FloatValue::from_f64(-0.0);
    let template = CodeTemplate::new("gain(#{value:L})").unwrap();
    let block = template.apply().set("value", value).build().unwrap();
    let fragment = CodeFragment::of("gain(%L)", (value,)).unwrap();
    let expected = "gain(::core::primitive::f64::from_bits(0x8000000000000000u64))";
    for block in [block, CodeBlock::of("%L", fragment).unwrap()] {
        assert_eq!(block.render_standalone(&Rust::new(), 80).unwrap(), expected);
    }
}

#[test]
fn explicit_float_nodes_are_nonempty_and_render_in_file_headers_and_bodies() {
    let mut block = CodeBlock::builder().build().unwrap();
    block
        .nodes_mut()
        .push(CodeNode::FloatValue(FloatValue::Binary64(0)));
    assert!(!block.is_empty());
    let decoded: CodeBlock = serde_json::from_str(&serde_json::to_string(&block).unwrap()).unwrap();
    let file = FileSpec::builder_with("values.rs", Rust::new())
        .header(CodeBlock::of("const HEADER: f64 = %L;", decoded.clone()).unwrap())
        .add_code(CodeBlock::of("const BODY: f64 = %L;", decoded).unwrap())
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    for name in ["HEADER", "BODY"] {
        assert!(
            output.contains(&format!(
                "const {name}: f64 = ::core::primitive::f64::from_bits(0x0000000000000000u64);"
            )),
            "{output}"
        );
    }
}

#[test]
fn source_rewrite_observes_float_intent_before_target_construction() {
    #[derive(Debug)]
    struct RewriteValue;
    impl RendererLang for RewriteValue {
        fn file_extension(&self) -> &str {
            "rs"
        }
        fn line_comment_prefix(&self) -> &str {
            "//"
        }
        fn rewrite_nodes(&self, nodes: &mut Vec<CodeNode>) {
            sigil_stitch::lang::rewrite::walk_nodes_mut(nodes, &|nodes| {
                for node in nodes {
                    if let CodeNode::FloatValue(value) = node {
                        *value = FloatValue::Binary64(0x8000_0000_0000_0000);
                    }
                }
            });
        }
        fn lower_float_value(&self, value: &FloatValue) -> Result<CodeBlock, SigilStitchError> {
            Rust::new().lower_float_value(value)
        }
        fn lower_type_name(&self, value: &TypeName) -> Result<CodeBlock, SigilStitchError> {
            Rust::new().lower_type_name(value)
        }
    }
    assert_eq!(
        render_value(FloatValue::Binary32(0), &RewriteValue).unwrap(),
        "::core::primitive::f64::from_bits(0x8000000000000000u64)"
    );
}

#[test]
fn float_lowering_can_introduce_soft_breaks_before_adapter_selection() {
    let language = SuppliedOutput(
        CodeBlock::of(
            "%T::from_bits(%>%W0x80000000u32%<%W)",
            TypeName::qualified("::core::primitive", "f32"),
        )
        .unwrap(),
    );
    let source = CodeBlock::of("%L", FloatValue::Binary32(0x8000_0000)).unwrap();
    for (width, wrapped) in [(16, true), (240, false)] {
        let output = source.render_standalone(&language, width).unwrap();
        assert_eq!(output.contains('\n'), wrapped, "{output}");
        assert_eq!(
            output.split_whitespace().collect::<String>(),
            "::core::primitive::f32::from_bits(0x80000000u32)"
        );
    }
}

#[test]
fn type_lowering_cannot_smuggle_a_deferred_float_into_a_type_expression() {
    #[derive(Debug)]
    struct ValueAsType;
    impl RendererLang for ValueAsType {
        fn file_extension(&self) -> &str {
            "value-as-type"
        }
        fn line_comment_prefix(&self) -> &str {
            "//"
        }
        fn lower_type_name(&self, _: &TypeName) -> Result<CodeBlock, SigilStitchError> {
            CodeBlock::of("%L", FloatValue::Binary32(0))
        }
    }
    let source = CodeBlock::of("%T", TypeName::primitive("f32")).unwrap();
    assert!(matches!(
        CodeRenderer::new(&ValueAsType, &ImportGroup::new(), 80).render(&source),
        Err(SigilStitchError::InvalidTypeNameLowering { language, .. })
            if language == "value-as-type"
    ));
}
