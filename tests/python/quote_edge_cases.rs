use sigil_stitch::code_block::{CodeBlock, CodeFragment};
use sigil_stitch::lang::python::Python;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

fn render(block: &CodeBlock) -> String {
    render_width(block, 80)
}

fn render_width(block: &CodeBlock, width: usize) -> String {
    FileSpec::builder("test.py")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(width)
        .unwrap()
}

#[test]
fn test_decorator() {
    let block = sigil_quote!(Python {
        @app.route("/users")
        def get_users(): {
            return jsonify(users)
        }
    })
    .unwrap();
    golden::assert_golden("python/quote_decorator.py", &render(&block));
}

#[test]
fn test_python_keyword_argument_spacing_is_contextual() {
    let callable = TypeName::primitive("field");
    let keyword = "kw_only";
    let block = sigil_quote!(Python {
        $T(callable)($N(keyword)=True, other == value)
        value = True
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("field(kw_only=True, other == value)"),
        "{output}"
    );
    assert!(output.contains("value = True"), "{output}");
}

#[test]
fn test_python_control_header_parentheses_preserve_generators_and_normalize_elif() {
    let block = sigil_quote!(Python {
        if (item for item in values) {
            return item;
        } elif (enabled) {
            return fallback;
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("if (item for item in values):"), "{output}");
    assert!(output.contains("elif enabled:"), "{output}");
}

#[test]
fn test_python_slice_colon_stays_tight() {
    let block = sigil_quote!(Python {
        result = items[start:stop]
        stepped = items[start::step]
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("result = items[start:stop]"), "{output}");
    assert!(output.contains("stepped = items[start::step]"), "{output}");
}

#[test]
fn test_python_call_group_breaks_with_explicit_continuation_scope() {
    let block = sigil_quote!(Python {
        result = function(first_argument, second_argument, third_argument)
    })
    .unwrap();
    let output = render_width(&block, 30);
    assert!(
        output.contains(
            "function(\n    first_argument,\n    second_argument,\n    third_argument,\n)"
        ),
        "{output}"
    );
}

#[test]
fn test_python_walrus_and_compound_assignment_spacing() {
    let block = sigil_quote!(Python {
        result = (value := source)
        total += value
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("result = (value := source)"), "{output}");
    assert!(output.contains("total += value"), "{output}");
}

#[test]
fn test_python_default_parameter_spacing_is_contextual() {
    let block = sigil_quote!(Python {
        def defaults(value=1, typed: int = 2): {
            return value + typed
        }
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("def defaults(value=1, typed: int = 2):"),
        "{output}"
    );
}

#[test]
fn test_python_lambda_default_spacing_is_tight() {
    let block = sigil_quote!(Python { value = lambda item=1: item }).unwrap();
    let output = render(&block);
    assert!(output.contains("value = lambda item=1: item"), "{output}");
}

#[test]
fn test_python_relative_import_spacing() {
    let block = sigil_quote!(Python { from . import sibling }).unwrap();
    let output = render(&block);
    assert!(output.contains("from . import sibling"), "{output}");
}

#[test]
fn test_python_inline_for_inherits_call_argument_context() {
    let names = ["first", "second"];
    let block = sigil_quote!(Python {
        field($for(name in &names; separator = ", ") { $N(name)=True })
    })
    .unwrap();
    let output = render(&block);
    assert!(
        output.contains("field(first=True, second=True)"),
        "{output}"
    );
}

#[test]
fn test_python_inline_if_inherits_call_argument_context() {
    let include_keyword = true;
    let block = sigil_quote!(Python {
        field($if(include_keyword) { kw_only=True } $else { other=False })
    })
    .unwrap();
    let output = render(&block);
    assert!(output.contains("field(kw_only=True)"), "{output}");
}

#[test]
fn test_python_comparison_is_not_angle_generic_spacing() {
    let block = sigil_quote!(Python { result = left < right }).unwrap();
    let output = render(&block);
    assert!(output.contains("result = left < right"), "{output}");
}

#[test]
fn test_type_hints() {
    let block = sigil_quote!(Python {
        def process(items: list[str], count: int = 10) -> dict[str, int]: {
            result: dict[str, int] = {}
            return result
        }
    })
    .unwrap();
    golden::assert_golden("python/quote_type_hints.py", &render(&block));
}

#[test]
fn test_comprehension() {
    let block = sigil_quote!(Python {
        squares = [x * x for x in range(10)]
        evens = [x for x in items if x % 2 == 0]
    })
    .unwrap();
    golden::assert_golden("python/quote_comprehension.py", &render(&block));
}

#[test]
fn test_async_def() {
    let block = sigil_quote!(Python {
        async def fetch_data(url: str) -> bytes: {
            async with aiohttp.ClientSession() as session: {
                response = await session.get(url)
                return await response.read()
            }
        }
    })
    .unwrap();
    golden::assert_golden("python/quote_async.py", &render(&block));
}

#[test]
fn test_with_statement() {
    let block = sigil_quote!(Python {
        with open($S("file.txt"), $S("r")) as f: {
            content = f.read()
        }
    })
    .unwrap();
    golden::assert_golden("python/quote_with.py", &render(&block));
}

#[test]
fn test_name_keyword_escape_in_macro() {
    let name = "class";
    let block = sigil_quote!(Python {
        $N(name) = 1
    })
    .unwrap();

    let output = render(&block);
    assert!(output.contains("class_ = 1"), "got: {output}");
}

#[test]
fn test_multiline_paren_union_type() {
    // Newline between `(` and `$for` in a multiline parenthesized group
    // should be preserved in the output.
    let members = ["Member1", "Member2"];
    let block = sigil_quote!(Python {
        type MyType = (
        $for((i, m) in members.iter().enumerate()) {
            $if(i == 0) { $S(*m) }
            $if(i > 0) { | $S(*m) }
        }
        )
    })
    .unwrap();
    let output = render(&block);
    // Content should appear on a new line after `(`
    assert!(
        output.contains("= (\n"),
        "newline after ( missing, got:\n{output}"
    );
    assert!(
        output.contains("\n)"),
        "newline before ) missing, got:\n{output}"
    );
}

#[test]
fn test_code_fragment_composes_python_indentation() {
    let fragment = CodeFragment::of("if enabled:\n%>return value%<", ()).unwrap();
    let block = sigil_quote!(Python {
        def choose(enabled: bool, value: str) -> str: {
            $L(fragment)
            return "fallback"
        }
    })
    .unwrap();

    let output = render(&block);
    assert!(
        output.contains("def choose(enabled: bool, value: str) -> str:\n    if enabled:\n        return value\n    return \"fallback\""),
        "fragment indentation should compose structurally, got:\n{output}"
    );
    assert!(!output.contains("%>"), "indent marker leaked:\n{output}");
    assert!(!output.contains("%<"), "dedent marker leaked:\n{output}");
}

#[test]
fn test_python_raw_member_preserves_trailing_newlines() {
    let raw = "print('opaque')\n\n";
    let output = FileSpec::builder_with("raw.py", Python::new())
        .add_raw(raw)
        .build()
        .unwrap()
        .render(88)
        .unwrap();
    assert_eq!(output, raw);
}

#[test]
fn test_raw_python_literal_rejects_unresolved_indent_marker() {
    let result = CodeBlock::of("%L", "if enabled:\n%>return value\n%<");

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("unresolved indentation marker '%>'"),
        "got: {err_msg}"
    );
}

#[test]
fn test_ruff_singleton_call_edge_layout() {
    let block = sigil_quote!(Python {
        value = function(argument)
    })
    .unwrap();
    let output = render_width(&block, 20);
    assert_eq!(output, "value = function(\n    argument\n)\n");
}

#[test]
fn test_ruff_local_import_spacing() {
    let block = sigil_quote!(Python {
        def load(): {
            from . import sibling
            return sibling
        }
    })
    .unwrap();
    let output = render_width(&block, 88);
    assert_eq!(
        output,
        "def load():\n    from . import sibling\n\n    return sibling\n"
    );
}

#[test]
fn test_ruff_top_level_declaration_spacing() {
    let first = sigil_quote!(Python {
        def first(): {
            pass
        }
    })
    .unwrap();
    let second = sigil_quote!(Python {
        def second(): {
            pass
        }
    })
    .unwrap();
    let output = FileSpec::builder("test.py")
        .add_code(first)
        .add_code(second)
        .build()
        .unwrap()
        .render(88)
        .unwrap();
    assert_eq!(
        output,
        "def first():\n    pass\n\n\ndef second():\n    pass\n"
    );
}

#[test]
fn test_ruff_nested_generator_condition_shape() {
    let block = sigil_quote!(Python {
        if (_support.is_dict(value) and all((_support.number(_item, True)
            or isinstance(_item, str)) for _item in _support.as_dict(value).values())) {
            return value;
        }
    })
    .unwrap();
    let output = render_width(&block, 88);
    assert_eq!(
        output,
        "if _support.is_dict(value) and all(\n    (_support.number(_item, True) or isinstance(_item, str))\n    for _item in _support.as_dict(value).values()\n):\n    return value\n"
    );
}

#[test]
fn test_ruff_dict_comprehension_shape() {
    let block = sigil_quote!(Python {
        return {key: convert(item) for key, item in values.items()}
    })
    .unwrap();
    let output = render_width(&block, 40);
    assert_eq!(
        output,
        "return {\n    key: convert(item)\n    for key, item in values.items()\n}\n"
    );
}

#[test]
fn test_ruff_nested_generic_call_shape() {
    let block = sigil_quote!(Python {
        additional_properties: _dict[_str, dict[str, Choice] | str] = _dataclass_field(default_factory=lambda: _dict[_str, dict[str, Choice] | str](), kw_only=True, metadata={"openapi_additional_properties": True, "openapi_dictionary": True})
    })
    .unwrap();
    let output = render_width(&block, 88);
    assert_eq!(
        output,
        "additional_properties: _dict[_str, dict[str, Choice] | str] = _dataclass_field(\n    default_factory=lambda: _dict[_str, dict[str, Choice] | str](),\n    kw_only=True,\n    metadata={\"openapi_additional_properties\": True, \"openapi_dictionary\": True},\n)\n"
    );
}
