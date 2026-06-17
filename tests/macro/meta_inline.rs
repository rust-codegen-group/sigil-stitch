use super::helpers::*;

// --- Inline $for inside expressions ---

#[test]
fn test_inline_for_inside_parens_ts() {
    let items = vec!["a", "b"];
    let block = sigil_quote!(TypeScript {
        const x = [$for(item in &items) { $S(*item), }];
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("const x = ["), "got: {output}");
    assert!(output.contains("'a'"), "got: {output}");
    assert!(output.contains("'b'"), "got: {output}");
    // No stray braces from block delimiters
    assert!(!output.contains('{'), "unexpected brace in: {output}");
    assert!(!output.contains('}'), "unexpected brace in: {output}");
}

#[test]
fn test_inline_for_zero_items() {
    let items: Vec<&str> = vec![];
    let block = sigil_quote!(TypeScript {
        const x = [$for(item in &items) { $S(*item), }];
    })
    .unwrap();
    let output = render_ts(&block);
    // Empty iteration produces nothing inside the array
    assert!(
        output.contains("const x = []") || output.contains("const x = [ ]"),
        "got: {output}"
    );
}

#[test]
fn test_inline_for_inside_parens_py() {
    let items = vec!["a", "b"];
    let block = sigil_quote!(Python {
        x = [$for(item in &items) { $S(*item), }]
    })
    .unwrap();
    let output = render_py(&block);
    assert!(output.contains("x = ["), "got: {output}");
    assert!(output.contains("'a'"), "got: {output}");
    assert!(output.contains("'b'"), "got: {output}");
    // No stray colon from block delimiters
    assert!(
        !output.contains(": ["),
        "unexpected colon before bracket in: {output}"
    );
}

#[test]
fn test_inline_for_empty_py() {
    let items: Vec<&str> = vec![];
    let block = sigil_quote!(Python {
        x = [$for(item in &items) { $S(*item), }]
    })
    .unwrap();
    let output = render_py(&block);
    assert!(
        output.contains("x = []") || output.contains("x = [ ]"),
        "got: {output}"
    );
}

// --- Inline $if/$else inside expressions ---

#[test]
fn test_inline_if_true_branch_ts() {
    let flag = true;
    let block = sigil_quote!(TypeScript {
        const msg = $if(flag) { "yes" } $else { "no" };
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("yes"), "got: {output}");
    assert!(!output.contains("no"), "got: {output}");
    // No stray braces
    assert!(!output.contains('{'), "unexpected brace in: {output}");
    assert!(!output.contains('}'), "unexpected brace in: {output}");
}

#[test]
fn test_inline_if_false_branch_ts() {
    let flag = false;
    let block = sigil_quote!(TypeScript {
        const msg = $if(flag) { "yes" } $else { "no" };
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("no"), "got: {output}");
    assert!(!output.contains("yes"), "got: {output}");
}

#[test]
fn test_inline_if_false_without_else_ts() {
    let flag = false;
    let block = sigil_quote!(TypeScript {
        const prefix = "x";
        $if(flag) { const dead = true; }
        const suffix = "y";
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("prefix"), "got: {output}");
    assert!(output.contains("suffix"), "got: {output}");
    assert!(!output.contains("dead"), "got: {output}");
}

#[test]
fn test_inline_if_inside_function_call() {
    let flag = true;
    let block = sigil_quote!(TypeScript {
        foo($if(flag) { "active" } $else { "inactive" })
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("foo("), "got: {output}");
    assert!(output.contains("active"), "got: {output}");
    assert!(!output.contains("inactive"), "got: {output}");
    assert!(!output.contains('{'), "unexpected brace in: {output}");
}

// --- $T import tracking inside inline $for ---

#[test]
fn test_inline_for_with_type_imports() {
    let types = vec![
        TypeName::importable_type("./models", "User"),
        TypeName::importable_type("./models", "Admin"),
    ];
    let block = sigil_quote!(TypeScript {
        const types = [$for(ty in &types) { $T(ty.clone()), }];
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(
        output.contains("import type { Admin, User } from './models'"),
        "imports should be tracked, got:\n{output}"
    );
    assert!(output.contains("User"), "got: {output}");
    assert!(output.contains("Admin"), "got: {output}");
}

// --- Mixed inline and statement-level ---

#[test]
fn test_inline_for_nested_in_stmt_if() {
    let items = vec!["a", "b"];
    let flag = true;
    let block = sigil_quote!(TypeScript {
        $if(flag) {
            const arr = [$for(item in &items) { $S(*item), }];
        }
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("const arr = ["), "got: {output}");
    assert!(output.contains("'a'"), "got: {output}");
    assert!(output.contains("'b'"), "got: {output}");
}

#[test]
fn test_stmt_for_with_inline_if() {
    let pairs: Vec<(&str, bool)> = vec![("x", true), ("y", false)];
    let block = sigil_quote!(TypeScript {
        $for((name, flag) in &pairs) {
            $if(*flag) { required: $S(*name) } $else { optional: $S(*name) }
        }
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("required: 'x'"), "got: {output}");
    assert!(output.contains("optional: 'y'"), "got: {output}");
}

// --- Inline $if with $else_if chain ---

#[test]
fn test_inline_if_else_if_chain_ts() {
    let a = false;
    let b = true;
    let block = sigil_quote!(TypeScript {
        const msg = $if(a) { "a" } $else_if(b) { "b" } $else { "c" };
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("\"b\""), "got: {output}");
    assert!(!output.contains("\"a\""), "got: {output}");
    assert!(!output.contains("\"c\""), "got: {output}");
}

#[test]
fn test_inline_if_else_in_object_literal() {
    let flag = true;
    let block = sigil_quote!(TypeScript {
        const obj = { key: $if(flag) { 1 } $else { 2 } };
    })
    .unwrap();
    let output = render_ts(&block);
    assert!(output.contains("key: 1"), "got: {output}");
    assert!(!output.contains("key: 2"), "got: {output}");
    // Object literal delimiters preserved, content correct
    assert!(output.contains("const obj = {"), "got: {output}");
    assert!(output.contains("key: 1"), "got: {output}");
    assert!(output.contains("};"), "got: {output}");
    // Not block `:` delimiter
    assert!(!output.contains("= :"), "got: {output}");
}

// --- Literal braces must stay literal (not become block delimiters) ---

#[test]
fn test_inline_if_python_dict_preserves_literal_braces() {
    let flag = true;
    let block = sigil_quote!(Python {
        x = { "k": $if(flag) { 1 } $else { 2 } }
    })
    .unwrap();
    let output = render_py(&block);
    assert!(output.contains("x = {"), "got: {output}");
    assert!(output.contains("}"), "got: {output}");
    assert!(output.contains("\"k\": 1"), "got: {output}");
    // Must NOT have stray colon from block_open
    assert!(!output.contains(": {"), "got: {output}");
}

#[test]
fn test_inline_if_python_dict_literal_ruby_hash() {
    // Ruby hash literal with inline $if — braces must stay literal
    // Simulation: Python with inline conditional inside dict
    let flag = false;
    let block = sigil_quote!(Python {
        x = { "a": $if(flag) { true } $else { false } }
    })
    .unwrap();
    let output = render_py(&block);
    assert!(output.contains("x = {"), "got: {output}");
    assert!(output.contains("false"), "got: {output}");
    assert!(!output.contains("true"), "got: {output}");
    // No stray `: {` pattern
    assert!(!output.contains(": {"), "got: {output}");
}

// --- Stricter exact-output tests ---

#[test]
fn test_inline_for_ts_array_exact() {
    let items = vec!["a"];
    let block = sigil_quote!(TypeScript {
        const x = [$for(item in &items) { $S(*item) }];
    })
    .unwrap();
    let output = render_ts(&block);
    assert_eq!(output, "const x = ['a'];\n");
}

#[test]
fn test_inline_if_function_call_exact() {
    let flag = true;
    let block = sigil_quote!(TypeScript {
        foo($if(flag) { "active" } $else { "inactive" });
    })
    .unwrap();
    let output = render_ts(&block);
    assert_eq!(output, "foo(\"active\");\n");
}

#[test]
fn test_inline_for_python_multiline_list_exact() {
    let items = vec!["a", "b"];
    let block = sigil_quote!(Python {
        x = [
            $for(item in &items) { $S(*item), }
        ]
    })
    .unwrap();
    let output = render_py(&block);
    assert_eq!(output, "x = ['a',\n'b',]\n");
}

#[test]
fn test_inline_if_ts_object_literal_exact() {
    let enabled = true;
    let block = sigil_quote!(TypeScript {
        const obj = { enabled: $if(enabled) { true } $else { false } };
    })
    .unwrap();
    let output = render_ts(&block);
    assert_eq!(output, "const obj = {enabled: true};\n");
}

#[test]
fn test_inline_if_ruby_hash_literal_preserves_braces() {
    let enabled = false;
    let block = sigil_quote!(Ruby {
        config = { enabled: $if(enabled) { true } $else { false } }
    })
    .unwrap();
    let output = render_rb(&block);
    assert_eq!(output, "config = {enabled: false}\n");
}

#[test]
fn test_inline_if_php_array_literal_preserves_brackets() {
    let enabled = true;
    let block = sigil_quote!(Php {
        $$config = ["enabled" => $if(enabled) { true } $else { false }];
    })
    .unwrap();
    let output = render_php(&block);
    assert_eq!(output, "$config = [\"enabled\" => true];\n");
}

#[test]
fn test_inline_if_lua_table_literal_preserves_braces() {
    let enabled = true;
    let block = sigil_quote!(Lua {
        config = { enabled = $if(enabled) { true } $else { false } }
    })
    .unwrap();
    let output = render_lua(&block);
    assert_eq!(output, "config = {enabled = true}\n");
}

macro_rules! assert_multiline_paren_close_not_restored_for_plain_specifier {
    ($test_name:ident, $lang:ident, $render:ident) => {
        #[test]
        fn $test_name() {
            let block = sigil_quote!($lang {
                call(
                    $S("x")
                )
            })
            .unwrap();
            let output = $render(&block);
            assert!(
                !output.contains("\n)"),
                "plain specifier should not restore a newline before `)`, got:\n{output}"
            );
        }
    };
}

assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_bash,
    Bash,
    render_bash
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_c,
    C,
    render_c
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_cpp,
    Cpp,
    render_cpp
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_csharp,
    CSharp,
    render_cs
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_dart,
    Dart,
    render_dart
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_go,
    Go,
    render_go
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_haskell,
    Haskell,
    render_hs
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_java,
    Java,
    render_java
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_javascript,
    JavaScript,
    render_js
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_kotlin,
    Kotlin,
    render_kt
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_lua,
    Lua,
    render_lua
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_ocaml,
    OCaml,
    render_ml
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_php,
    Php,
    render_php
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_python,
    Python,
    render_py
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_ruby,
    Ruby,
    render_rb
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_rust,
    Rust,
    render_rs
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_scala,
    Scala,
    render_scala
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_swift,
    Swift,
    render_swift
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_typescript,
    TypeScript,
    render_ts
);
assert_multiline_paren_close_not_restored_for_plain_specifier!(
    test_plain_specifier_multiline_paren_zsh,
    Zsh,
    render_zsh
);
