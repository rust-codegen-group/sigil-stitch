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
fn python_async_comprehension_breaks_before_the_complete_clause() {
    let block = sigil_quote!(Python {
        async def collect(values): {
            return (item async for item in values)
        }
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 32),
        "async def collect(values):\n    return (\n        item\n        async for item in values\n    )\n"
    );
}

#[test]
fn python_async_comprehension_preserves_the_clause_after_a_broken_element() {
    let block = sigil_quote!(Python {
        async def collect(values): {
            return [item.is_eligible and item.has_matching_metadata async for item in values if item.enabled]
        }
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 40),
        concat!(
            "async def collect(values):\n",
            "    return [\n",
            "        item.is_eligible\n",
            "        and item.has_matching_metadata\n",
            "        async for item in values\n",
            "        if item.enabled\n",
            "    ]\n",
        ),
    );
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

#[test]
fn test_python_static_call_parameter_layout_no_spurious_trailing_comma() {
    let block = sigil_quote!(Python {
        result = function(first_argument, second_argument)
    })
    .unwrap();
    let output = render_width(&block, 40);
    assert_eq!(
        output,
        "result = function(\n    first_argument, second_argument\n)\n"
    );
}

#[test]
fn test_python_dynamic_call_parameter_layout_breaks_items() {
    let names = ["first_argument", "second_argument", "third_argument"];
    let block = sigil_quote!(Python {
        result = function($for(name in &names; separator = ", ") { $N(name) })
    })
    .unwrap();
    let output = render_width(&block, 30);
    assert_eq!(
        output,
        "result = function(\n    first_argument,\n    second_argument,\n    third_argument,\n)\n"
    );
}

#[test]
fn python_call_loop_keeps_non_comma_separators_verbatim() {
    let parts = ["first", "second"];
    let block = sigil_quote!(Python {
        run($for(part in parts; separator = "") { $N(part) })
    })
    .unwrap();
    assert_eq!(render(&block), "run(firstsecond)\n");
}

#[test]
fn python_conditional_argument_shares_the_call_indentation() {
    let block = sigil_quote!(Python {
        def run() {
            return Response(data=None if "optional_choice" not in data else decode_optional_choice(data.get("optional_choice")), raw=response)
        }
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 100),
        concat!(
            "def run():\n",
            "    return Response(\n",
            "        data=None\n",
            "        if \"optional_choice\" not in data\n",
            "        else decode_optional_choice(data.get(\"optional_choice\")),\n",
            "        raw=response,\n",
            "    )\n",
        ),
    );
}

#[test]
fn python_boolean_chain_wraps_inside_the_control_header() {
    let block = sigil_quote!(Python {
        if (checks.first_condition(value) and checks.second_condition(value) and checks.third_condition(value)) {
            return value
        }
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 100),
        concat!(
            "if (\n",
            "    checks.first_condition(value)\n",
            "    and checks.second_condition(value)\n",
            "    and checks.third_condition(value)\n",
            "):\n",
            "    return value\n",
        ),
    );
}

#[test]
fn broken_dynamic_arguments_do_not_duplicate_explicit_trailing_commas() {
    let args = ["first_long_parameter", "second_long_parameter"];
    let block = sigil_quote!(Python {
        result = f($for(name in args; separator = ", ", trailing = true) { $N(name) })
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 32),
        "result = f(\n    first_long_parameter,\n    second_long_parameter,\n)\n",
        "{block:?}",
    );
}

#[test]
fn python_collection_edges_explode_items_together() {
    let block = sigil_quote!(Python {
        really_long_dictionary_variable["key"] = (first_argument, second_argument, third_argument)
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 60),
        concat!(
            "really_long_dictionary_variable[\"key\"] = (\n",
            "    first_argument,\n",
            "    second_argument,\n",
            "    third_argument,\n",
            ")\n",
        )
    );
}

#[test]
fn python_inline_pragmas_stay_on_the_expression_line() {
    let block = sigil_quote!(Python {
        value = decode(payload, encoding) $comment("type: ignore[assignment]  # démо")
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 36),
        "value = decode(payload, encoding)  # type: ignore[assignment]  # démо\n"
    );
}

#[test]
fn python_pragma_layout_uses_python_policy_for_pyi_files() {
    let block = sigil_quote!(Python {
        value = decode(payload, encoding) $comment("type: ignore[assignment]  # démо")
    })
    .unwrap();
    let output = FileSpec::builder_with("test.pyi", Python::new().with_extension("pyi"))
        .add_code(block)
        .build()
        .unwrap()
        .render(36)
        .unwrap();
    assert_eq!(
        output,
        "value = decode(payload, encoding)  # type: ignore[assignment]  # démо\n"
    );
}

#[test]
fn python_conditional_expression_preserves_statement_prefixes_and_nesting() {
    let assert_block = sigil_quote!(Python {
        assert first_choice if condition else second_choice
    })
    .unwrap();
    assert_eq!(
        render_width(&assert_block, 20),
        "assert (\n    first_choice\n    if condition\n    else second_choice\n)\n"
    );

    let augmented_block = sigil_quote!(Python {
        value += first_choice if condition else second_choice
    })
    .unwrap();
    assert_eq!(
        render_width(&augmented_block, 20),
        "value += (\n    first_choice\n    if condition\n    else second_choice\n)\n"
    );

    let chained_block = sigil_quote!(Python {
        first = second = first_choice if condition else second_choice
    })
    .unwrap();
    assert_eq!(
        render_width(&chained_block, 20),
        "first = second = (\n    first_choice\n    if condition\n    else second_choice\n)\n"
    );

    let nested_block = sigil_quote!(Python {
        value = first if second else third if fourth else fifth
    })
    .unwrap();
    assert_eq!(
        render_width(&nested_block, 20),
        "value = (\n    first\n    if second\n    else third\n    if fourth\n    else fifth\n)\n"
    );
}

#[test]
fn python_explicit_trailing_comma_forces_item_breaks() {
    let block = sigil_quote!(Python {
        result = function(first_argument, second_argument,)
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 88),
        "result = function(\n    first_argument,\n    second_argument,\n)\n"
    );
}

#[test]
fn python_empty_dynamic_collection_does_not_emit_a_comma() {
    let items: [&str; 0] = [];
    let block = sigil_quote!(Python {
        values = [$for(item in &items; separator = ", ") { $N(item) }]
    })
    .unwrap();
    assert_eq!(render_width(&block, 20), "values = []\n");
}

#[test]
fn python_dynamic_trailing_loop_preserves_requested_tuple_separator() {
    let names = ["single"];
    let block = sigil_quote!(Python {
        value = ($for(name in &names; separator = ", ", trailing = true) { $N(name) })
    })
    .unwrap();
    assert_eq!(render_width(&block, 88), "value = (single,)\n");
}

#[test]
fn python_dynamic_trailing_loop_keeps_call_magic_comma() {
    let names = ["first_argument", "second_argument"];
    let block = sigil_quote!(Python {
        result = function($for(name in &names; separator = ", ", trailing = true) { $N(name) })
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 88),
        "result = function(\n    first_argument,\n    second_argument,\n)\n"
    );
}

#[test]
fn python_empty_dynamic_list_stays_empty_when_the_assignment_breaks() {
    let items: [&str; 0] = [];
    let block = sigil_quote!(Python {
        a_very_long_variable_name_that_exceeds_width = [$for(item in &items; separator = ", ") { $N(item) }]
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 40),
        "a_very_long_variable_name_that_exceeds_width = []\n"
    );
}

#[test]
fn python_dynamic_singleton_subscript_keeps_scalar_index_semantics() {
    let items = ["an_extremely_long_index_name_that_breaks"];
    let block = sigil_quote!(Python {
        values[$for(item in &items; separator = ", ") { $N(item) }]
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 40),
        "values[\n    an_extremely_long_index_name_that_breaks\n]\n"
    );
}

#[test]
fn python_literal_loop_evaluates_iterator_and_trailing_option_once() {
    let mut iterator_evaluations = 0;
    let mut trailing_evaluations = 0;
    let block = sigil_quote!(Python {
        result = f($for(item in { iterator_evaluations += 1; ["first", "second"] }; separator = ", ", trailing = { trailing_evaluations += 1; false }) { $N(item) })
    })
    .unwrap();
    assert_eq!(iterator_evaluations, 1);
    assert_eq!(trailing_evaluations, 1);
    assert_eq!(render_width(&block, 88), "result = f(first, second)\n");

    let empty: [&str; 0] = [];
    let block = sigil_quote!(Python {
        result = f($for(item in empty; separator = ", ", trailing = { trailing_evaluations += 1; true }) { $N(item) })
    })
    .unwrap();
    assert_eq!(trailing_evaluations, 1);
    assert_eq!(render_width(&block, 88), "result = f()\n");
}

#[test]
fn python_mixed_and_runtime_loop_separators_are_verbatim() {
    let separator = ", ";
    let names = ["second", "third"];
    let block = sigil_quote!(Python {
        mixed = f(first, $for(item in names; separator = ", ", trailing = true) { $N(item) })
        runtime = f($for(item in names; separator = separator, trailing = true) { $N(item) })
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 88),
        "mixed = f(first, second, third, )\nruntime = f(second, third, )\n",
    );
}

#[test]
fn python_explicit_trailing_comma_forces_tuple_item_breaks() {
    let block = sigil_quote!(Python {
        value = (first_argument, second_argument,)
    })
    .unwrap();
    assert_eq!(
        render_width(&block, 88),
        "value = (\n    first_argument,\n    second_argument,\n)\n"
    );
}
