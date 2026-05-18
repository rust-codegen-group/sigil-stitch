use super::helpers::*;

#[test]
fn test_dot_chaining() {
    let block = sigil_quote!(TypeScript {
        foo.bar.baz();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("foo.bar.baz();"), "got: {output}");
}

#[test]
fn test_triple_equals() {
    let block = sigil_quote!(TypeScript {
        if(x === 0) {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("x === 0"), "got: {output}");
}

#[test]
fn test_not_equals() {
    let block = sigil_quote!(TypeScript {
        if(x !== null) {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("x !== null"), "got: {output}");
}

#[test]
fn test_arrow_function() {
    let block = sigil_quote!(TypeScript {
        const fn = (x) => x + 1;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("=>"), "got: {output}");
    assert!(output.contains("x + 1"), "got: {output}");
}

#[test]
fn test_rust_path_separator() {
    let block = sigil_quote!(RustLang {
        let x = std::mem::size_of::<u32>();
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("std::mem::size_of"), "got: {output}");
    assert!(!output.contains(":: "), "no space after ::, got: {output}");
    assert!(!output.contains(" ::"), "no space before ::, got: {output}");
}

#[test]
fn test_unary_not() {
    let block = sigil_quote!(TypeScript {
        if(!flag) {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("!flag"), "got: {output}");
}

#[test]
fn test_bracket_access() {
    let block = sigil_quote!(TypeScript {
        const x = arr[0];
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("arr[0]"), "got: {output}");
}

#[test]
fn test_comma_spacing() {
    let block = sigil_quote!(TypeScript {
        foo(a, b, c);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("foo(a, b, c);"), "got: {output}");
}

#[test]
fn test_parenthesized_expression() {
    let block = sigil_quote!(TypeScript {
        const x = (a + b) * c;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("(a + b) * c"), "got: {output}");
}

#[test]
fn test_nested_parens() {
    let block = sigil_quote!(TypeScript {
        foo((a, b));
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("foo((a, b))"), "got: {output}");
}

#[test]
fn test_array_literal() {
    let block = sigil_quote!(TypeScript {
        const arr = [1, 2, 3];
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("[1, 2, 3]"), "got: {output}");
}

// --- Colon spacing: ternary, elvis, type annotations ---

#[test]
fn test_java_ternary_colon_spacing() {
    let block = sigil_quote!(JavaLang {
        String result = x != null ? x.toString() : "default";
    })
    .unwrap();

    let output = render_java(&block);
    assert!(
        output.contains("? x.toString() : \"default\""),
        "ternary colon should have space before it. got: {output}"
    );
}

#[test]
fn test_simple_ternary_colon_spacing() {
    let block = sigil_quote!(TypeScript {
        const y = x > 0 ? x : 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("? x : 0"),
        "ternary colon should have space before it. got: {output}"
    );
}

#[test]
fn test_kotlin_elvis_spacing() {
    let block = sigil_quote!(Kotlin {
        val result = x ?: "default";
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(
        output.contains("x ?: \"default\""),
        "elvis operator should stay glued. got: {output}"
    );
}

#[test]
fn test_type_annotation_colon_no_space() {
    let block = sigil_quote!(TypeScript {
        const name: string = "foo";
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("name: string"),
        "type annotation colon should have no space before it. got: {output}"
    );
    assert!(
        !output.contains("name : string"),
        "should not have space before colon in type annotation. got: {output}"
    );
}

#[test]
fn test_go_walrus_spacing() {
    let block = sigil_quote!(GoLang {
        x := 42;
    })
    .unwrap();

    let output = render_go(&block);
    assert!(
        output.contains("x := 42"),
        "Go := should have space before colon. got: {output}"
    );
}

#[test]
fn test_ternary_resets_after_colon() {
    let block = sigil_quote!(TypeScript {
        const a = x ? 1 : 2;
        const b: number = 3;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("? 1 : 2"),
        "ternary should have space before colon. got: {output}"
    );
    assert!(
        output.contains("b: number"),
        "type annotation after ternary should suppress space. got: {output}"
    );
}

// --- Comprehensive ColonContext coverage ---

#[test]
fn test_colon_type_annotation_in_function_param() {
    let block = sigil_quote!(TypeScript {
        function greet(name: string, age: number) {
            return name;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("name: string"), "got: {output}");
    assert!(output.contains("age: number"), "got: {output}");
}

#[test]
fn test_colon_type_annotation_with_interpolated_type() {
    let t = TypeName::primitive("string");
    let block = sigil_quote!(TypeScript {
        const x: $T(t) = "hello";
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("x: string"), "got: {output}");
    assert!(!output.contains("x : string"), "got: {output}");
}

#[test]
fn test_colon_type_annotation_resets_after_semicolon() {
    let block = sigil_quote!(TypeScript {
        const a = x ? 1 : 2;
        const b: string = "hi";
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("? 1 : 2"), "ternary space. got: {output}");
    assert!(
        output.contains("b: string"),
        "type annotation after semicolon. got: {output}"
    );
}

#[test]
fn test_colon_type_annotation_multiple_per_line() {
    let block = sigil_quote!(TypeScript {
        const fn = (a: number, b: string) => a;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("a: number"), "got: {output}");
    assert!(output.contains("b: string"), "got: {output}");
}

#[test]
fn test_colon_map_entry_in_object_literal() {
    let block = sigil_quote!(TypeScript {
        const config = { timeout: 5000, retries: 3 };
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("timeout: 5000"), "got: {output}");
    assert!(output.contains("retries: 3"), "got: {output}");
    assert!(!output.contains("timeout : 5000"), "got: {output}");
}

#[test]
fn test_colon_map_entry_restores_after_brace_group() {
    let block = sigil_quote!(TypeScript {
        const config = { timeout: 5000 };
        const x: number = 1;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("timeout: 5000"), "got: {output}");
    assert!(
        output.contains("x: number"),
        "type annotation after object literal. got: {output}"
    );
}

#[test]
fn test_colon_ternary_inside_object_literal() {
    let block = sigil_quote!(TypeScript {
        const config = { value: x ? 1 : 0 };
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("value:"), "map entry colon. got: {output}");
    assert!(
        output.contains("? 1 : 0"),
        "ternary inside object should have space. got: {output}"
    );
}

#[test]
fn test_colon_ternary_with_function_calls() {
    let block = sigil_quote!(TypeScript {
        const result = isValid(x) ? getValue(x) : getDefault();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("? getValue(x) : getDefault()"),
        "got: {output}"
    );
}

#[test]
fn test_colon_nested_ternary() {
    let block = sigil_quote!(TypeScript {
        const r = a ? 1 : b ? 2 : 3;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("a ? 1 : b"), "first ternary. got: {output}");
    assert!(
        output.contains("b ? 2 : 3"),
        "nested ternary. got: {output}"
    );
}

#[test]
fn test_colon_ternary_with_interpolation() {
    let default_val = "fallback";
    let block = sigil_quote!(TypeScript {
        const x = cond ? value : $L(default_val);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("? value : fallback"),
        "ternary with interpolation. got: {output}"
    );
}

#[test]
fn test_colon_ternary_resets_at_statement_boundary() {
    let block = sigil_quote!(TypeScript {
        const a = x ? y : z;
        const b: string = "ok";
        const c = p ? q : r;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("? y : z"), "first ternary. got: {output}");
    assert!(
        output.contains("b: string"),
        "type annotation between ternaries. got: {output}"
    );
    assert!(output.contains("? q : r"), "second ternary. got: {output}");
}

#[test]
fn test_colon_ternary_in_java_error_handling() {
    let block = sigil_quote!(JavaLang {
        String msg = error != null ? error.getMessage() : "unknown";
    })
    .unwrap();

    let output = render_java(&block);
    assert!(
        output.contains("? error.getMessage() : \"unknown\""),
        "got: {output}"
    );
}

#[test]
fn test_colon_path_separator_rust() {
    let block = sigil_quote!(RustLang {
        let v = std::collections::HashMap::new();
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("std::"), "got: {output}");
    assert!(output.contains("HashMap::"), "got: {output}");
    assert!(
        !output.contains("std ::"),
        "no space before first colon. got: {output}"
    );
}

#[test]
fn test_colon_path_separator_cpp() {
    let block = sigil_quote!(CppLang {
        auto v = std::make_unique(42);
    })
    .unwrap();

    let file = FileSpec::builder_with("test.cpp", sigil_stitch::lang::cpp_lang::CppLang::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(output.contains("std::"), "got: {output}");
    assert!(
        !output.contains("std ::"),
        "no space before first colon. got: {output}"
    );
}

#[test]
fn test_colon_path_then_type_annotation() {
    let block = sigil_quote!(RustLang {
        let x: std::string::String = String::new();
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("x: std::"), "got: {output}");
    assert!(!output.contains("x : std"), "got: {output}");
}

#[test]
fn test_colon_walrus_with_expression() {
    let block = sigil_quote!(GoLang {
        result := computeValue(42);
    })
    .unwrap();

    let output = render_go(&block);
    assert!(
        output.contains("result := computeValue(42)"),
        "got: {output}"
    );
}

#[test]
fn test_colon_walrus_multiple_assignments() {
    let block = sigil_quote!(GoLang {
        x := 1;
        y := 2;
        z := x + y;
    })
    .unwrap();

    let output = render_go(&block);
    assert!(output.contains("x := 1"), "got: {output}");
    assert!(output.contains("y := 2"), "got: {output}");
    assert!(output.contains("z := x + y"), "got: {output}");
}

#[test]
fn test_colon_walrus_in_control_flow() {
    let block = sigil_quote!(GoLang {
        if err := doSomething() {
            return err;
        }
    })
    .unwrap();

    let output = render_go(&block);
    assert!(output.contains("err := doSomething()"), "got: {output}");
}

#[test]
fn test_colon_context_walrus_then_type_annotation() {
    let block = sigil_quote!(TypeScript {
        const x = { a: 1 };
        const y: number = 2;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("a: 1"), "map entry. got: {output}");
    assert!(
        output.contains("y: number"),
        "type annotation. got: {output}"
    );
}

#[test]
fn test_colon_context_all_in_sequence() {
    let block = sigil_quote!(TypeScript {
        const a: string = x ? "y" : "z";
        const b = { key: a };
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("a: string"),
        "type annotation. got: {output}"
    );
    assert!(
        output.contains("? \"y\" : \"z\"") || output.contains("? 'y' : 'z'"),
        "ternary. got: {output}"
    );
    assert!(output.contains("key: a"), "map entry. got: {output}");
}

#[test]
fn test_colon_context_kotlin_full_scenario() {
    let block = sigil_quote!(Kotlin {
        val name: String = value ?: "default";
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(
        output.contains("name: String"),
        "type annotation. got: {output}"
    );
    assert!(output.contains("?: \"default\""), "elvis. got: {output}");
}

#[test]
fn test_colon_switch_case_label() {
    let block = sigil_quote!(JavaLang {
        switch(x) {
            case(1):
            return "one";
        }
    })
    .unwrap();

    let output = render_java(&block);
    assert!(output.contains("case (1):"), "got: {output}");
}

// --- Safe-call `?.` spacing ---

#[test]
fn test_safe_call_no_space_before_question_dot() {
    let block = sigil_quote!(Kotlin {
        val name = response.body?.string();
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("response.body?.string()"), "got: {output}");
}

#[test]
fn test_safe_call_let_no_space() {
    let block = sigil_quote!(Kotlin {
        value?.let { process(it) }
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("value?.let"), "got: {output}");
}

#[test]
fn test_ternary_still_has_space_before_question() {
    let block = sigil_quote!(JavaLang {
        String result = x != null ? x.toString() : "default";
    })
    .unwrap();

    let output = render_java(&block);
    assert!(output.contains("null ? x"), "got: {output}");
    assert!(output.contains(": \"default\""), "got: {output}");
}

#[test]
fn test_elvis_no_space_before_question() {
    let block = sigil_quote!(Kotlin {
        val result = x ?: "default";
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("x ?: \"default\""), "got: {output}");
}

// --- Postfix pointer (`*`) spacing (Issue #44) ---

#[test]
fn test_postfix_star_pointer_type() {
    let block = sigil_quote!(CppLang {
        Config* cfg = get_config();
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(
        output.contains("Config*"),
        "no space before * in pointer type. got: {output}"
    );
}

#[test]
fn test_postfix_star_const_pointer() {
    let block = sigil_quote!(CppLang {
        const char* host = get_host();
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(
        output.contains("char*"),
        "no space before * after type name. got: {output}"
    );
}

#[test]
fn test_multiplication_still_spaced() {
    // After a group close, `*` should still be multiplication
    let block = sigil_quote!(TypeScript {
        const x = (a + b) * c;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("(a + b) * c"),
        "multiplication should keep spaces. got: {output}"
    );
}

// --- Postfix `++` and `--` spacing ---

#[test]
fn test_postfix_increment() {
    let block = sigil_quote!(JavaScript {
        for (let i = 0; i < 10; i++) {
            count++;
        }
    })
    .unwrap();

    let output = render_js(&block);
    assert!(output.contains("i++"), "got: {output}");
    assert!(output.contains("count++"), "got: {output}");
}

#[test]
fn test_postfix_decrement() {
    let block = sigil_quote!(JavaScript {
        while (n > 0) {
            n--;
        }
    })
    .unwrap();

    let output = render_js(&block);
    assert!(output.contains("n--"), "got: {output}");
}

#[test]
fn test_binary_plus_still_spaced() {
    let block = sigil_quote!(JavaScript {
        const x = a + b;
    })
    .unwrap();

    let output = render_js(&block);
    assert!(
        output.contains("a + b"),
        "binary + should keep spaces. got: {output}"
    );
}

#[test]
fn test_binary_minus_still_spaced() {
    let block = sigil_quote!(JavaScript {
        const x = a - b;
    })
    .unwrap();

    let output = render_js(&block);
    assert!(
        output.contains("a - b"),
        "binary - should keep spaces. got: {output}"
    );
}

#[test]
fn test_binary_star_still_spaced() {
    let block = sigil_quote!(CLang {
        int x = a * b;
    })
    .unwrap();

    let output = render_c(&block);
    assert!(
        output.contains("a * b"),
        "binary * should keep spaces. got: {output}"
    );
}

// ── Dash flag spacing (#93) ──────────────────────────────

#[test]
fn test_dash_flag_no_space() {
    let block = sigil_quote!(TypeScript {
        console.log(-q, -f, -avz);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("-q"), "got: {output}");
    assert!(output.contains("-f"), "got: {output}");
    assert!(output.contains("-avz"), "got: {output}");
}

#[test]
fn test_dash_binary_with_space() {
    let block = sigil_quote!(TypeScript {
        const x = a - b;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("a - b"),
        "binary minus should keep spaces, got: {output}"
    );
}

// ── Slash path spacing (#93) ─────────────────────────────

#[test]
fn test_slash_path_no_space() {
    let block = sigil_quote!(TypeScript {
        const path = linux/amd64;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("linux/amd64"),
        "path separator should be tight, got: {output}"
    );
}

#[test]
fn test_slash_division_with_space() {
    let block = sigil_quote!(TypeScript {
        const x = a / b;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("a / b"),
        "division should keep spaces, got: {output}"
    );
}

#[test]
fn test_hyphenated_flag_intact() {
    let block = sigil_quote!(TypeScript {
        console.log(--from-oci-layout);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("--from-oci-layout"),
        "hyphenated flag should stay intact, got: {output}"
    );
}
