#![expect(deprecated, reason = "0.6.8 renderer compatibility assertions")]

use super::*;
use super::{direct::DirectAdapter, pretty::PrettyAdapter};
use std::cell::RefCell;
use std::rc::Rc;

use crate::code_block::{CodeBlock, StringLitArg, VerbatimStrArg};
use crate::import::ImportGroup;
use crate::lang::CodeLang;
use crate::lang::config::BlockSyntaxConfig;
use crate::lang::typescript::TypeScript;
use crate::type_name::TypeName;
#[derive(Debug)]
struct TestLang {
    indent: &'static str,
    multiline_strings: bool,
    reject_bad_after_rewrite: bool,
}

impl RendererLang for TestLang {
    fn file_extension(&self) -> &str {
        "test"
    }

    fn line_comment_prefix(&self) -> &str {
        "//"
    }

    fn block_syntax(&self) -> BlockSyntaxConfig<'_> {
        BlockSyntaxConfig {
            indent_unit: self.indent,
            ..BlockSyntaxConfig::default()
        }
    }

    fn render_string_literal(&self, value: &str) -> String {
        if self.multiline_strings {
            value.to_string()
        } else {
            format!("\"{value}\"")
        }
    }

    fn render_verbatim_string(&self, value: &str) -> String {
        self.render_string_literal(value)
    }

    fn rewrite_nodes(&self, nodes: &mut Vec<CodeNode>) {
        if self.reject_bad_after_rewrite
            && matches!(nodes.first(), Some(CodeNode::Literal(text)) if text == "bad")
        {
            nodes.push(CodeNode::Dedent);
        }
    }
}

fn test_lang(indent: &'static str) -> TestLang {
    TestLang {
        indent,
        multiline_strings: false,
        reject_bad_after_rewrite: false,
    }
}

fn render_block(block: &CodeBlock, width: usize) -> String {
    let ts = TypeScript::new();
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&ts, &imports, width);
    renderer.render(block).unwrap()
}

#[test]
fn test_simple_statement() {
    let mut builder = CodeBlock::builder();
    builder.add_statement("const x = 42", ());
    let output = render_block(&builder.build().unwrap(), 80);
    assert_eq!(output.trim(), "const x = 42;");
}

#[test]
fn test_control_flow() {
    let mut builder = CodeBlock::builder();
    builder.begin_control_flow("if (x > 0)", ());
    builder.add_statement("return x", ());
    builder.end_control_flow();
    let output = render_block(&builder.build().unwrap(), 80);
    assert!(output.contains("if (x > 0) {"));
    assert!(output.contains("  return x;"));
    assert!(output.contains('}'));
}

#[test]
fn test_if_else() {
    let mut builder = CodeBlock::builder();
    builder.begin_control_flow("if (x > 0)", ());
    builder.add_statement("return x", ());
    builder.next_control_flow("else", ());
    builder.add_statement("return 0", ());
    builder.end_control_flow();
    let output = render_block(&builder.build().unwrap(), 80);
    assert!(output.contains("} else {"));
    assert!(output.contains("  return 0;"));
}

#[test]
fn test_type_rendering() {
    let user = TypeName::importable("./models", "User");
    let imports = ImportGroup {
        entries: vec![crate::import::ImportEntry {
            module: "./models".to_string(),
            name: "User".to_string(),
            alias: None,
            is_type_only: true,
            is_side_effect: false,
            is_wildcard: false,
        }],
    };
    let block = CodeBlock::of("const u: %T = getUser()", (user,)).unwrap();
    let ts = TypeScript::new();
    let mut renderer = CodeRenderer::new(&ts, &imports, 80);
    assert_eq!(
        renderer.render(&block).unwrap(),
        "const u: User = getUser()"
    );
}

#[test]
fn test_string_literal() {
    let block = CodeBlock::of(
        "const x = %S",
        (crate::code_block::StringLitArg("hello".to_string()),),
    )
    .unwrap();
    assert_eq!(render_block(&block, 80), "const x = 'hello'");
}

#[test]
fn compatibility_type_lowerer_rejects_string_singletons() {
    let block = CodeBlock::of("%T", (TypeName::string_literal("active"),)).unwrap();
    let lang = test_lang("  ");
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);

    assert!(matches!(
        renderer.render(&block),
        Err(SigilStitchError::UnsupportedTypeName { reason, .. })
            if reason == "the 0.6.8 compatibility lowerer does not support string singleton types"
    ));
}

#[test]
fn test_nested_indent() {
    let mut builder = CodeBlock::builder();
    builder.begin_control_flow("if (a)", ());
    builder.begin_control_flow("if (b)", ());
    builder.add_statement("return c", ());
    builder.end_control_flow();
    builder.end_control_flow();
    let output = render_block(&builder.build().unwrap(), 80);
    assert!(output.contains("    return c;"));
}

#[test]
fn test_comment() {
    let mut builder = CodeBlock::builder();
    builder.add_comment("This is a comment");
    let output = render_block(&builder.build().unwrap(), 80);
    assert!(output.contains("// This is a comment"));
}

#[test]
fn empty_comment_uses_the_language_delimiters_without_padding() {
    let mut builder = CodeBlock::builder();
    builder.add_comment("");
    assert_eq!(render_block(&builder.build().unwrap(), 80), "//\n");
}

#[test]
fn test_multiline_literal_via_percent_l_reindents_each_line() {
    let mut builder = CodeBlock::builder();
    builder.begin_control_flow("interface User", ());
    builder.add("%L", "/**\n * The user's name.\n */".to_string());
    builder.add_line();
    builder.end_control_flow();
    let output = render_block(&builder.build().unwrap(), 80);
    assert!(output.contains("  /**"));
    assert!(output.contains("   * The user's name."));
    assert!(output.contains("   */"));
    assert!(!output.contains("\n * The user's name."));
    assert!(!output.contains("\n */"));
}

#[test]
fn test_multiline_literal_direct_reindents_each_line() {
    let mut builder = CodeBlock::builder();
    builder.begin_control_flow("function f()", ());
    builder.add("line1\nline2\nline3", ());
    builder.add_line();
    builder.end_control_flow();
    let output = render_block(&builder.build().unwrap(), 80);
    assert!(output.contains("  line1"));
    assert!(output.contains("  line2"));
    assert!(output.contains("  line3"));
    assert!(!output.contains("\nline2"));
}

#[test]
fn test_block_open_for_override() {
    use crate::lang::haskell::Haskell;

    let haskell = Haskell::new();
    let imports = ImportGroup::new();
    let mut builder = CodeBlock::builder();
    builder.begin_control_flow("class Functor f", ());
    builder.add_statement("fmap :: a -> b", ());
    builder.end_control_flow();
    let block = builder.build().unwrap();
    let mut renderer = CodeRenderer::new(&haskell, &imports, 80);
    let output = renderer.render(&block).unwrap();
    assert!(output.contains("class Functor f where"));
}

#[test]
fn pretty_path_preserves_exact_tab_indentation() {
    let block = CodeBlock::of("list(%>%Walpha,%Wbeta%<%W)", ()).unwrap();
    let lang = test_lang("\t");
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 8);

    let output = renderer.render(&block).unwrap();
    assert_eq!(output, "list(\n\talpha,\n\tbeta )");
    assert!(!output.contains("\n "));
}

#[test]
fn pretty_path_preserves_exact_multi_character_indentation() {
    let block = CodeBlock::of("list(%>%Walpha,%Wbeta%<%W)", ()).unwrap();
    let lang = test_lang(".-");
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 8);

    let output = renderer.render(&block).unwrap();
    assert_eq!(output, "list(\n.-alpha,\n.-beta )");
    assert!(!output.contains("\n  "));
}

#[test]
fn pretty_path_reindents_multiline_structured_text() {
    let mut builder = CodeBlock::builder();
    builder.add("x(%>%W", ());
    builder.add_comment("first\nsecond");
    builder.add("%<)", ());

    let output = render_block(&builder.build().unwrap(), 8);
    assert_eq!(output, "x(\n  // first\n  // second\n)");
    assert!(!output.contains("\n//"));
}

#[test]
fn opaque_multiline_hook_output_is_not_reindented() {
    let lang = TestLang {
        indent: "  ",
        multiline_strings: true,
        reject_bad_after_rewrite: false,
    };
    let imports = ImportGroup::new();

    let direct = CodeBlock::of(
        "%>value: %S%<",
        (StringLitArg("first\nsecond".to_string()),),
    )
    .unwrap();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);
    assert_eq!(renderer.render(&direct).unwrap(), "  value: first\nsecond");

    let pretty =
        CodeBlock::of("x(%>%W%S%<)", (StringLitArg("first\nsecond".to_string()),)).unwrap();
    assert_eq!(renderer.render(&pretty).unwrap(), "x( first\nsecond)");

    let direct = CodeBlock::of(
        "%>value: %V%<",
        (VerbatimStrArg("first\nsecond".to_string()),),
    )
    .unwrap();
    assert_eq!(renderer.render(&direct).unwrap(), "  value: first\nsecond");

    let pretty = CodeBlock::of(
        "x(%>%W%V%<)",
        (VerbatimStrArg("first\nsecond".to_string()),),
    )
    .unwrap();
    assert_eq!(renderer.render(&pretty).unwrap(), "x( first\nsecond)");

    let whitespace_lines = "first\n   \n\t\nsecond";
    let pretty = CodeBlock::of("%S%Wtail", (StringLitArg(whitespace_lines.to_string()),)).unwrap();
    assert_eq!(
        renderer.render(&pretty).unwrap(),
        "first\n   \n\t\nsecond tail"
    );

    let pretty =
        CodeBlock::of("%V%Wtail", (VerbatimStrArg(whitespace_lines.to_string()),)).unwrap();
    assert_eq!(
        renderer.render(&pretty).unwrap(),
        "first\n   \n\t\nsecond tail"
    );
}

#[test]
fn soft_break_uses_display_width_at_zero_and_unicode_boundaries() {
    let combining = CodeBlock::of("é%Wz", ()).unwrap();
    assert_eq!(render_block(&combining, 3), "é z");
    assert_eq!(render_block(&combining, 2), "é\nz");
    assert_eq!(render_block(&combining, 0), "é\nz");

    let cjk = CodeBlock::of("界%Wz", ()).unwrap();
    assert_eq!(render_block(&cjk, 4), "界 z");
    assert_eq!(render_block(&cjk, 3), "界\nz");

    let control = CodeBlock::of("\u{7}a%Wz", ()).unwrap();
    assert_eq!(render_block(&control, 4), "\u{7}a z");
    assert_eq!(render_block(&control, 3), "\u{7}a\nz");
}

#[test]
fn broken_soft_break_does_not_indent_an_empty_line() {
    let block = CodeBlock::of("x(%>%W\nvalue%<)", ()).unwrap();

    let output = render_block(&block, 2);
    assert_eq!(output, "x(\n\n  value)");
    assert!(
        !output
            .lines()
            .any(|line| !line.is_empty() && line.trim().is_empty())
    );
}

#[test]
fn nested_and_sequence_soft_breaks_select_the_pretty_adapter() {
    let nested = CodeBlock::of("nested(%>%Wa,%Wb%<)", ()).unwrap();
    let mut block = CodeBlock::of("outer[%L]", nested).unwrap();
    block.nodes_mut().push(CodeNode::Sequence(vec![
        CodeNode::Literal(" tail(".to_string()),
        CodeNode::Indent,
        CodeNode::SoftBreak,
        CodeNode::Literal("c".to_string()),
        CodeNode::Dedent,
        CodeNode::Literal(")".to_string()),
    ]));

    let output = render_block(&block, 8);
    assert!(output.contains("nested(\n  a,\n  b)"));
    assert!(output.contains("tail(\n  c)"));
}

#[test]
fn sequence_soft_break_alone_selects_the_pretty_adapter() {
    let mut block = CodeBlock::of("outer", ()).unwrap();
    block.nodes_mut().push(CodeNode::Sequence(vec![
        CodeNode::SoftBreak,
        CodeNode::Literal("child".to_string()),
    ]));

    assert_eq!(render_block(&block, 6), "outer\nchild");
}

#[test]
fn direct_adapter_reports_invalid_internal_operations() {
    let mut adapter = DirectAdapter::new("  ", 80);
    assert!(matches!(
        adapter.dedent(),
        Err(SigilStitchError::Render { context, .. })
            if context == "CodeRenderer direct indentation"
    ));

    adapter.soft_break().unwrap();
    assert_eq!(adapter.finish(), " ");
}

#[test]
fn pretty_adapter_reports_invalid_internal_operations() {
    let mut adapter = PrettyAdapter::new("  ", 80);
    assert!(matches!(
        adapter.dedent(),
        Err(SigilStitchError::Render { context, .. })
            if context == "CodeRenderer pretty indentation"
    ));
    assert!(matches!(
        adapter.end_group(),
        Err(SigilStitchError::Render { context, .. })
            if context == "CodeRenderer pretty groups"
    ));

    let mut adapter = PrettyAdapter::new("  ", 80);
    adapter.begin_group(LayoutGroup::IndependentBreaks).unwrap();
    assert!(matches!(
        adapter.finish(),
        Err(SigilStitchError::Render { context, .. })
            if context == "CodeRenderer pretty groups"
    ));
}

#[test]
fn terminal_render_failure_stops_before_later_renderer_events() {
    #[derive(Debug)]
    struct FailingRenderLang(Rc<RefCell<Vec<&'static str>>>);

    impl RendererLang for FailingRenderLang {
        fn file_extension(&self) -> &str {
            "test"
        }

        fn line_comment_prefix(&self) -> &str {
            "//"
        }

        fn block_open_for_intent(&self, _intent: BlockIntent, _condition: &str) -> Option<&str> {
            self.0.borrow_mut().push("open");
            Some("{")
        }

        fn block_close_for_intent(&self, _intent: BlockIntent, _condition: &str) -> Option<&str> {
            self.0.borrow_mut().push("close");
            Some("}")
        }
    }

    impl CodeLang for FailingRenderLang {}

    let events = Rc::new(RefCell::new(Vec::new()));
    let lang = FailingRenderLang(events.clone());
    let imports = ImportGroup::new();
    let renderer = CodeRenderer::new(&lang, &imports, 80);
    let prepared = CodeBlock {
        nodes: vec![
            CodeNode::BlockOpenIntent {
                condition: "if ready".to_string(),
                intent: BlockIntent::If,
            },
            CodeNode::Dedent,
            CodeNode::BlockCloseIntent {
                condition: "if ready".to_string(),
                intent: BlockIntent::If,
            },
        ],
    };

    assert!(matches!(
        renderer.render_prepared(&prepared),
        Err(SigilStitchError::Render { context, .. })
            if context == "CodeRenderer direct indentation"
    ));
    assert_eq!(events.borrow().as_slice(), &["open"]);
}

#[test]
fn multiline_type_continuations_use_block_indent_once() {
    let compound_types = [
        TypeName::generic(
            TypeName::primitive("Result"),
            vec![
                TypeName::primitive("VeryLongSuccess"),
                TypeName::primitive("VeryLongFailure"),
            ],
        ),
        TypeName::union(vec![
            TypeName::primitive("VeryLongSuccess"),
            TypeName::primitive("VeryLongFailure"),
        ]),
        TypeName::tuple(vec![
            TypeName::primitive("VeryLongSuccess"),
            TypeName::primitive("VeryLongFailure"),
        ]),
        TypeName::function(
            vec![
                TypeName::primitive("VeryLongSuccess"),
                TypeName::primitive("VeryLongFailure"),
            ],
            TypeName::primitive("VeryLongReturn"),
        ),
    ];

    for compound_type in compound_types {
        let direct = CodeBlock::of("%>value: %T%<", (compound_type.clone(),)).unwrap();
        let pretty = CodeBlock::of("marker%W\n%>value: %T%<", (compound_type,)).unwrap();

        for block in [&direct, &pretty] {
            let output = render_block(block, 20);
            let continuation = output
                .lines()
                .find(|line| line.contains("VeryLongFailure"))
                .unwrap();
            assert!(
                continuation.starts_with("  "),
                "type continuation should preserve its block indentation:\n{output}"
            );
            assert!(
                !continuation.starts_with("      "),
                "type continuation should not duplicate type indentation:\n{output}"
            );
        }
    }
}

#[test]
fn lowered_infix_type_sequences_break_consistently() {
    let union = TypeName::union(vec![
        TypeName::primitive("Alpha"),
        TypeName::primitive("Beta"),
        TypeName::primitive("Gamma"),
    ]);
    let block = CodeBlock::of("type Value = %T", (union,)).unwrap();

    assert_eq!(
        render_block(&block, 20),
        "type Value = Alpha\n| Beta\n| Gamma"
    );
}

#[test]
fn renderer_state_is_local_after_post_rewrite_validation_error() {
    let lang = TestLang {
        indent: "  ",
        multiline_strings: false,
        reject_bad_after_rewrite: true,
    };
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 80);

    let bad = CodeBlock::of("bad", ()).unwrap();
    assert!(matches!(
        renderer.render(&bad),
        Err(SigilStitchError::InvalidRewrittenSource { context, reason })
            if context == "standalone" && reason.contains("depth is -1")
    ));

    let good = CodeBlock::of("good", ()).unwrap();
    assert_eq!(renderer.render(&good).unwrap(), "good");
}

#[test]
fn renderer_state_is_local_between_successful_calls() {
    let lang = test_lang("  ");
    let imports = ImportGroup::new();
    let mut renderer = CodeRenderer::new(&lang, &imports, 8);

    let first = CodeBlock::of("first", ()).unwrap();
    assert_eq!(renderer.render(&first).unwrap(), "first");

    let second = CodeBlock::of("list(%>%Walpha,%Wbeta%<)", ()).unwrap();
    assert_eq!(
        renderer.render(&second).unwrap(),
        "list(\n  alpha,\n  beta)"
    );
}
