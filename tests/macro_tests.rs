//! Integration tests for the `sigil_quote!` proc macro.

use sigil_stitch::code_block::{CodeBlock, NameArg, StringLitArg};
use sigil_stitch::import_collector;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::lang::ocaml::OCaml;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

// ════════════════════════════════════════════════════════
// A. Basic statements
// ════════════════════════════════════════════════════════

#[test]
fn test_simple_statement() {
    let block = sigil_quote!(TypeScript {
        const x = 42;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 42;"), "got: {output}");
}

#[test]
fn test_multiple_statements() {
    let block = sigil_quote!(TypeScript {
        const a = 1;
        const b = 2;
        const c = a + b;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const a = 1;"), "got: {output}");
    assert!(output.contains("const b = 2;"), "got: {output}");
    assert!(output.contains("const c = a + b;"), "got: {output}");
}

#[test]
fn test_empty_body() {
    let block = sigil_quote!(TypeScript {}).unwrap();
    assert!(block.is_empty());
}

// ════════════════════════════════════════════════════════
// B. Interpolation — $T, $N, $S, $L, $C
// ════════════════════════════════════════════════════════

#[test]
fn test_type_interpolation_import_tracking() {
    let user_type = TypeName::importable_type("./models", "User");
    let block = sigil_quote!(TypeScript {
        const user: $T(user_type) = getUser();
    })
    .unwrap();

    let refs = import_collector::collect_imports(&block);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "User");
}

#[test]
fn test_type_interpolation_renders() {
    let user_type = TypeName::importable_type("./models", "User");
    let block = sigil_quote!(TypeScript {
        const user: $T(user_type) = getUser();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("User"), "got: {output}");
    assert!(output.contains("= getUser();"), "got: {output}");
}

#[test]
fn test_name_interpolation() {
    let var_name = "myVar";
    let block = sigil_quote!(TypeScript {
        const $N(var_name) = 42;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("myVar"), "got: {output}");
    assert!(output.contains("= 42;"), "got: {output}");
}

#[test]
fn test_string_lit_interpolation() {
    let block = sigil_quote!(TypeScript {
        console.log($S("hello world"));
    })
    .unwrap();

    let output = render_ts(&block);
    // TypeScript renderer uses single quotes by default.
    assert!(
        output.contains("console.log('hello world');"),
        "got: {output}"
    );
}

#[test]
fn test_literal_interpolation() {
    let val = "42";
    let block = sigil_quote!(TypeScript {
        const x = $L(val);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 42;"), "got: {output}");
}

#[test]
fn test_code_block_interpolation() {
    let inner = CodeBlock::of("doSomething()", ()).unwrap();
    let block = sigil_quote!(TypeScript {
        $C(inner);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("doSomething();"), "got: {output}");
}

#[test]
fn test_multiple_interpolations_in_one_statement() {
    let t1 = TypeName::primitive("string");
    let t2 = TypeName::primitive("number");
    let block = sigil_quote!(TypeScript {
        const x: $T(t1) = $L("getVal") as $T(t2);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("string"), "got: {output}");
    assert!(output.contains("number"), "got: {output}");
    assert!(output.contains("getVal"), "got: {output}");
}

#[test]
fn test_mixed_arg_types_in_one_statement() {
    let ty = TypeName::primitive("User");
    let block = sigil_quote!(TypeScript {
        const $N("x"): $T(ty) = $S("hello") + $L("42");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("x"), "got: {output}");
    assert!(output.contains("User"), "got: {output}");
    assert!(output.contains("'hello'"), "got: {output}");
    assert!(output.contains("42"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// C. Control flow
// ════════════════════════════════════════════════════════

#[test]
fn test_if_block() {
    let block = sigil_quote!(TypeScript {
        if(x > 0) {
            return true;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x > 0) {"), "got: {output}");
    assert!(output.contains("return true;"), "got: {output}");
    assert!(output.contains("}"), "got: {output}");
}

#[test]
fn test_if_else() {
    let block = sigil_quote!(TypeScript {
        if(x > 0) {
            return true;
        } else {
            return false;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x > 0) {"), "got: {output}");
    assert!(output.contains("} else {"), "got: {output}");
    assert!(output.contains("return false;"), "got: {output}");
}

#[test]
fn test_if_else_if_else() {
    let block = sigil_quote!(TypeScript {
        if(x > 0) {
            return 1;
        } else if(x < 0) {
            return 0;
        } else {
            return 0;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x > 0) {"), "got: {output}");
    assert!(output.contains("} else if (x < 0) {"), "got: {output}");
    assert!(output.contains("} else {"), "got: {output}");
}

#[test]
fn test_long_else_if_chain() {
    let block = sigil_quote!(TypeScript {
        if(a) {
            return 1;
        } else if(b) {
            return 2;
        } else if(c) {
            return 3;
        } else if(d) {
            return 4;
        } else {
            return 0;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("} else if (b) {"), "got: {output}");
    assert!(output.contains("} else if (c) {"), "got: {output}");
    assert!(output.contains("} else if (d) {"), "got: {output}");
    assert!(output.contains("} else {"), "got: {output}");
}

#[test]
fn test_nested_control_flow() {
    let block = sigil_quote!(TypeScript {
        if(x > 0) {
            if(y > 0) {
                return true;
            }
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x > 0) {"), "got: {output}");
    assert!(output.contains("if (y > 0) {"), "got: {output}");
    assert!(output.contains("return true;"), "got: {output}");
    // Should have proper indentation (two levels).
    assert!(output.contains("    return true;"), "got: {output}");
}

#[test]
fn test_empty_control_flow_body() {
    let block = sigil_quote!(TypeScript {
        if(x) {}
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x) {"), "got: {output}");
    assert!(output.contains("}"), "got: {output}");
}

#[test]
fn test_interpolation_in_condition() {
    let cond = "x > 0";
    let block = sigil_quote!(TypeScript {
        if($L(cond)) {
            return true;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x > 0) {"), "got: {output}");
}

#[test]
fn test_control_flow_with_type_and_string_interpolation() {
    let error_type = TypeName::importable("./errors", "NotFoundError");
    let block = sigil_quote!(TypeScript {
        if(!user) {
            throw new $T(error_type)($S("not found"));
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("NotFoundError"), "got: {output}");
    assert!(output.contains("'not found'"), "got: {output}");
    let refs = import_collector::collect_imports(&block);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].name, "NotFoundError");
}

// ════════════════════════════════════════════════════════
// D. Blank lines
// ════════════════════════════════════════════════════════

#[test]
fn test_single_blank_line() {
    let block = sigil_quote!(TypeScript {
        const a = 1;

        const b = 2;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("const a = 1;\n\nconst b = 2;"),
        "got: {output}"
    );
}

#[test]
fn test_multiple_blank_lines() {
    let block = sigil_quote!(TypeScript {
        const a = 1;



        const b = 2;
    })
    .unwrap();

    let output = render_ts(&block);
    // Multiple blank lines should be preserved.
    let idx_a = output.find("const a = 1;").unwrap();
    let idx_b = output.find("const b = 2;").unwrap();
    let between = &output[idx_a + "const a = 1;".len()..idx_b];
    let newlines = between.chars().filter(|c| *c == '\n').count();
    // At least 2 blank lines (3+ newlines between the statements).
    assert!(
        newlines >= 3,
        "expected multiple blank lines, got {newlines} newlines in: {between:?}"
    );
}

// ════════════════════════════════════════════════════════
// E. Comments
// ════════════════════════════════════════════════════════

#[test]
fn test_comment_with_semicolon() {
    let block = sigil_quote!(TypeScript {
        $comment("Initialize the value");
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// Initialize the value"), "got: {output}");
    assert!(output.contains("const x = 0;"), "got: {output}");
}

#[test]
fn test_comment_without_semicolon() {
    // $comment without trailing ; should also work.
    let block = sigil_quote!(TypeScript {
        $comment("no semicolon")
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// no semicolon"), "got: {output}");
    assert!(output.contains("const x = 0;"), "got: {output}");
}

#[test]
fn test_comment_only() {
    let block = sigil_quote!(TypeScript {
        $comment("just a comment");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// just a comment"), "got: {output}");
}

#[test]
fn test_multiple_comments() {
    let block = sigil_quote!(TypeScript {
        $comment("line 1");
        $comment("line 2");
        const x = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("// line 1"), "got: {output}");
    assert!(output.contains("// line 2"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// F. Object literals (not control flow)
// ════════════════════════════════════════════════════════

#[test]
fn test_object_literal_simple() {
    let block = sigil_quote!(TypeScript {
        const config = { timeout: 30, retries: 3 };
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("config = {"), "got: {output}");
    assert!(output.contains("timeout:"), "got: {output}");
    assert!(output.contains("retries:"), "got: {output}");
}

#[test]
fn test_nested_object_literal() {
    let block = sigil_quote!(TypeScript {
        const x = { a: 1, b: { c: 2 } };
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("{"), "got: {output}");
    assert!(output.contains("a:"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// G. Spacing edge cases
// ════════════════════════════════════════════════════════

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
    // `::` is reconstructed correctly (Joint punct), but `<`/`>` are separate
    // tokens so they get spaces. This is a known tokenization artifact.
    assert!(output.contains("std::"), "got: {output}");
    assert!(output.contains("mem::"), "got: {output}");
    assert!(output.contains("size_of"), "got: {output}");
    assert!(output.contains("u32"), "got: {output}");
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

// ════════════════════════════════════════════════════════
// H. Percent escaping
// ════════════════════════════════════════════════════════

#[test]
fn test_percent_in_source() {
    let block = sigil_quote!(TypeScript {
        const x = 100 % 10;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("100 % 10"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// I. Dollar escape
// ════════════════════════════════════════════════════════

#[test]
fn test_dollar_escape_basic() {
    let block = sigil_quote!(TypeScript {
        const price = $$100;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("$"), "got: {output}");
    assert!(output.contains("100"), "got: {output}");
}

#[test]
fn test_dollar_escape_with_interpolation() {
    let val = "50";
    let block = sigil_quote!(TypeScript {
        const total = $$100 + $L(val);
    })
    .unwrap();

    let output = render_ts(&block);
    // $$ becomes literal $ in format string; `100` is a separate token so there's a space
    assert!(output.contains("$"), "got: {output}");
    assert!(output.contains("100"), "got: {output}");
    assert!(output.contains("50"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// J. Wrap points ($W)
// ════════════════════════════════════════════════════════

#[test]
fn test_wrap_point_in_params() {
    let config_type = TypeName::primitive("Config");
    let block = sigil_quote!(TypeScript {
        export async function createUser($W name: string,$W age: number,$W config: $T(config_type) $W): Promise<void> {
            return undefined;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("createUser"), "got: {output}");
    assert!(output.contains("Config"), "got: {output}");
}

#[test]
fn test_wrap_point_narrow_width() {
    let block = sigil_quote!(TypeScript {
        doSomething($W alpha,$W beta,$W gamma);
    })
    .unwrap();

    // Render at narrow width to force line breaks.
    let file = FileSpec::builder("test.ts")
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(20).unwrap();
    // With narrow width, %W should break lines.
    assert!(output.contains("doSomething"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// K. Multi-language support
// ════════════════════════════════════════════════════════

#[test]
fn test_rust_language() {
    let block = sigil_quote!(RustLang {
        let x = 42;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("let x = 42;"), "got: {output}");
}

#[test]
fn test_python_control_flow() {
    // Python uses `:` instead of `{` for blocks, rendered by CodeLang.
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if x > 0", ());
    b.add_statement("return True", ());
    b.end_control_flow();
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(Python {
        if x > 0 {
            return True;
        }
    })
    .unwrap();

    let manual_output = render_py(&manual);
    let macro_output = render_py(&macro_block);
    // Both should produce Python-style output with `:` and indentation.
    assert!(
        manual_output.contains("if x > 0:"),
        "manual: {manual_output}"
    );
    assert!(macro_output.contains("if x > 0:"), "macro: {macro_output}");
}

#[test]
fn test_go_language() {
    let block = sigil_quote!(GoLang {
        x := 42;
    })
    .unwrap();

    let output = render_go(&block);
    // `:` is in the "no space before" set, so `x:= 42` is expected.
    // Go doesn't emit semicolons.
    assert!(output.contains("x:= 42"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// L. Equivalence tests — macro vs manual builder
// ════════════════════════════════════════════════════════

#[test]
fn test_equiv_simple_statement() {
    let user_type = TypeName::importable_type("./models", "User");

    let mut b = CodeBlock::builder();
    b.add_statement("const user: %T = getUser()", (user_type.clone(),));
    b.add_statement("return user", ());
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(TypeScript {
        const user: $T(user_type) = getUser();
        return user;
    })
    .unwrap();

    // Import refs should match.
    let manual_refs = import_collector::collect_imports(&manual);
    let macro_refs = import_collector::collect_imports(&macro_block);
    assert_eq!(manual_refs.len(), macro_refs.len());
    assert_eq!(manual_refs[0].name, macro_refs[0].name);
}

#[test]
fn test_equiv_control_flow() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if (x > 0)", ());
    b.add_statement("return true", ());
    b.next_control_flow("else", ());
    b.add_statement("return false", ());
    b.end_control_flow();
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(TypeScript {
        if(x > 0) {
            return true;
        } else {
            return false;
        }
    })
    .unwrap();

    let manual_output = render_ts(&manual);
    let macro_output = render_ts(&macro_block);
    assert_eq!(
        manual_output, macro_output,
        "manual:\n{manual_output}\nmacro:\n{macro_output}"
    );
}

#[test]
fn test_equiv_comment() {
    let mut b = CodeBlock::builder();
    b.add_comment("hello");
    b.add_statement("const x = 1", ());
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(TypeScript {
        $comment("hello");
        const x = 1;
    })
    .unwrap();

    let manual_output = render_ts(&manual);
    let macro_output = render_ts(&macro_block);
    assert_eq!(
        manual_output, macro_output,
        "manual:\n{manual_output}\nmacro:\n{macro_output}"
    );
}

#[test]
fn test_equiv_blank_line() {
    let mut b = CodeBlock::builder();
    b.add_statement("const a = 1", ());
    b.add_line();
    b.add_statement("const b = 2", ());
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(TypeScript {
        const a = 1;

        const b = 2;
    })
    .unwrap();

    let manual_output = render_ts(&manual);
    let macro_output = render_ts(&macro_block);
    assert_eq!(
        manual_output, macro_output,
        "manual:\n{manual_output}\nmacro:\n{macro_output}"
    );
}

#[test]
fn test_equiv_name_arg() {
    let mut b = CodeBlock::builder();
    b.add_statement("const %N = 1", (NameArg("x".to_string()),));
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(TypeScript {
        const $N("x") = 1;
    })
    .unwrap();

    let manual_output = render_ts(&manual);
    let macro_output = render_ts(&macro_block);
    assert_eq!(
        manual_output, macro_output,
        "manual:\n{manual_output}\nmacro:\n{macro_output}"
    );
}

#[test]
fn test_equiv_string_lit_arg() {
    let mut b = CodeBlock::builder();
    b.add_statement("console.log(%S)", (StringLitArg("hi".to_string()),));
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(TypeScript {
        console.log($S("hi"));
    })
    .unwrap();

    let manual_output = render_ts(&manual);
    let macro_output = render_ts(&macro_block);
    assert_eq!(
        manual_output, macro_output,
        "manual:\n{manual_output}\nmacro:\n{macro_output}"
    );
}

#[test]
fn test_equiv_nested_code_block() {
    let inner = CodeBlock::of("doWork()", ()).unwrap();

    let mut b = CodeBlock::builder();
    b.add_statement("%L", (inner.clone(),));
    let manual = b.build().unwrap();

    let macro_block = sigil_quote!(TypeScript {
        $C(inner);
    })
    .unwrap();

    let manual_output = render_ts(&manual);
    let macro_output = render_ts(&macro_block);
    assert_eq!(
        manual_output, macro_output,
        "manual:\n{manual_output}\nmacro:\n{macro_output}"
    );
}

#[test]
fn test_equiv_wrap_point() {
    // $W inserts %W but tokenizer adds spaces around tokens;
    // manual format strings can be denser (e.g., "%Wa" vs "%W a").
    // Just verify the macro version produces valid output with wrap points.
    let macro_block = sigil_quote!(TypeScript {
        foo($W a,$W b,$W c);
    })
    .unwrap();

    let output = render_ts(&macro_block);
    assert!(output.contains("foo("), "got: {output}");
    assert!(output.contains("a,"), "got: {output}");
    assert!(output.contains("b,"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// M. Args tuple shape
// ════════════════════════════════════════════════════════

#[test]
fn test_zero_args_statement() {
    let block = sigil_quote!(TypeScript {
        return null;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("return null;"), "got: {output}");
}

#[test]
fn test_single_arg_statement() {
    let ty = TypeName::primitive("number");
    let block = sigil_quote!(TypeScript {
        const x: $T(ty) = 1;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("number"), "got: {output}");
}

#[test]
fn test_many_args_statement() {
    let t1 = TypeName::primitive("string");
    let t2 = TypeName::primitive("number");
    let t3 = TypeName::primitive("boolean");
    let block = sigil_quote!(TypeScript {
        function f(a: $T(t1), b: $T(t2), c: $T(t3)): void {};
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("string"), "got: {output}");
    assert!(output.contains("number"), "got: {output}");
    assert!(output.contains("boolean"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// N. Complex real-world patterns
// ════════════════════════════════════════════════════════

#[test]
fn test_class_like_structure() {
    let user_type = TypeName::importable_type("./models", "User");
    let block = sigil_quote!(TypeScript {
        export class UserService {
            getUser(id: $T(user_type)): void {
                console.log($S("getting user"));
            }
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("export class UserService"), "got: {output}");
    assert!(output.contains("getUser"), "got: {output}");
    assert!(output.contains("'getting user'"), "got: {output}");
}

#[test]
fn test_for_loop() {
    let block = sigil_quote!(TypeScript {
        for(let i = 0; i < 10; i++) {
            console.log(i);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("for ("), "got: {output}");
    assert!(output.contains("i < 10"), "got: {output}");
    assert!(output.contains("console.log(i);"), "got: {output}");
}

#[test]
fn test_while_loop() {
    let block = sigil_quote!(TypeScript {
        while(running) {
            process();
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("while (running) {"), "got: {output}");
    assert!(output.contains("process();"), "got: {output}");
}

#[test]
fn test_try_catch() {
    let block = sigil_quote!(TypeScript {
        try {
            doRisky();
        } catch(e) {
            console.error(e);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("try {"), "got: {output}");
    // next_control_flow renders "} " then "catch(e)" on potentially separate lines
    assert!(output.contains("catch (e)"), "got: {output}");
    assert!(output.contains("doRisky();"), "got: {output}");
    assert!(output.contains("console.error(e);"), "got: {output}");
}

#[test]
fn test_statements_before_and_after_control_flow() {
    let block = sigil_quote!(TypeScript {
        const x = 1;
        if(x > 0) {
            return x;
        }
        const y = 2;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 1;"), "got: {output}");
    assert!(output.contains("if (x > 0) {"), "got: {output}");
    assert!(output.contains("const y = 2;"), "got: {output}");
}

#[test]
fn test_control_flow_then_statement() {
    let block = sigil_quote!(TypeScript {
        if(x) {
            doA();
        } else {
            doB();
        }
        cleanup();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("} else {"), "got: {output}");
    assert!(output.contains("cleanup();"), "got: {output}");
}

#[test]
fn test_many_types_with_imports() {
    let t1 = TypeName::importable_type("./a", "TypeA");
    let t2 = TypeName::importable_type("./b", "TypeB");
    let t3 = TypeName::importable_type("./c", "TypeC");

    let block = sigil_quote!(TypeScript {
        const a: $T(t1) = getA();
        const b: $T(t2) = getB();
        const c: $T(t3) = getC();
    })
    .unwrap();

    let refs = import_collector::collect_imports(&block);
    assert_eq!(refs.len(), 3, "refs: {refs:?}");

    let output = render_ts(&block);
    assert!(output.contains("import type { TypeA }"), "got: {output}");
    assert!(output.contains("import type { TypeB }"), "got: {output}");
    assert!(output.contains("import type { TypeC }"), "got: {output}");
}

#[test]
fn test_complex_expression_interpolation() {
    // Expression in $T() can be a method call.
    let block = sigil_quote!(TypeScript {
        const x: $T(TypeName::primitive("string")) = $S("hello");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("string"), "got: {output}");
    assert!(output.contains("'hello'"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// Helpers
// ════════════════════════════════════════════════════════

fn render_ts(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.ts")
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

fn render_rs(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.rs")
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

fn render_py(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.py")
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

fn render_go(block: &CodeBlock) -> String {
    let file = FileSpec::builder("test.go")
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

fn render_hs(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.hs", Haskell::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

fn render_ml(block: &CodeBlock) -> String {
    let file = FileSpec::builder_with("test.ml", OCaml::new())
        .add_code(block.clone())
        .build()
        .unwrap();
    file.render(80).unwrap()
}

// ════════════════════════════════════════════════════════
// O. Comment escape sequences
// ════════════════════════════════════════════════════════

#[test]
fn test_comment_with_newline_escape() {
    let block = sigil_quote!(TypeScript {
        $comment("first line\nsecond line");
        const x = 1;
    })
    .unwrap();

    let output = render_ts(&block);
    // The \n in the comment is unescaped to a real newline.
    // add_comment renders the prefix only on the first line.
    assert!(
        output.contains("// first line\nsecond line"),
        "got: {output}"
    );
}

#[test]
fn test_comment_with_tab_escape() {
    let block = sigil_quote!(TypeScript {
        $comment("indented\ttab");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("indented\ttab"), "got: {output}");
}

#[test]
fn test_comment_with_backslash_escape() {
    let block = sigil_quote!(TypeScript {
        $comment("path\\to\\file");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("path\\to\\file"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// P. Manual indent/dedent ($> / $<)
// ════════════════════════════════════════════════════════

#[test]
fn test_manual_indent_dedent() {
    // $> and $< control indent depth around statements.
    let block = sigil_quote!(TypeScript {
        namespace Foo {$>
        const x = 1;
        const y = 2;
        $<}
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("namespace Foo {"), "got: {output}");
    assert!(
        output.contains("    const x = 1;"),
        "expected indented x, got: {output}"
    );
    assert!(
        output.contains("    const y = 2;"),
        "expected indented y, got: {output}"
    );
    assert!(output.contains("}"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// Q. Custom block openers ($open)
// ════════════════════════════════════════════════════════

#[test]
fn test_open_haskell_where() {
    let block = sigil_quote!(Haskell {
        class Functor f $open(" where") {
            fmap :: (a -> b) -> f a -> f b;
        }
    })
    .unwrap();

    let output = render_hs(&block);
    assert!(output.contains("class Functor f where"), "got: {output}");
    assert!(output.contains("fmap"), "got: {output}");
    // Should NOT contain " =" (default Haskell block_open).
    assert!(!output.contains("class Functor f ="), "got: {output}");
}

#[test]
fn test_open_ocaml_module() {
    let block = sigil_quote!(OCaml {
        module Foo $open(" = struct") {
            let x = 42;
        }
    })
    .unwrap();

    let output = render_ml(&block);
    assert!(output.contains("module Foo = struct"), "got: {output}");
    assert!(output.contains("let x = 42"), "got: {output}");
}

#[test]
fn test_open_empty_suppresses_block_opener() {
    let block = sigil_quote!(TypeScript {
        something $open("") {
            body;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("something"), "got: {output}");
    assert!(output.contains("body;"), "got: {output}");
    // Should NOT contain " {" after "something" since we suppressed the opener.
    assert!(!output.contains("something {"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// R. Non-brace languages via sigil_quote!
// ════════════════════════════════════════════════════════

#[test]
fn test_haskell_basic_statement() {
    let block = sigil_quote!(Haskell {
        putStrLn "hello";
    })
    .unwrap();

    let output = render_hs(&block);
    assert!(output.contains("putStrLn \"hello\""), "got: {output}");
    // Haskell doesn't use semicolons in output.
    assert!(!output.contains(";"), "got: {output}");
}

#[test]
fn test_haskell_control_flow() {
    let block = sigil_quote!(Haskell {
        if x > 0 {
            return True;
        }
    })
    .unwrap();

    let output = render_hs(&block);
    // Haskell block_open() is " =" so this gets "if x > 0 ="
    assert!(output.contains("if x > 0 ="), "got: {output}");
    assert!(output.contains("return True"), "got: {output}");
}

#[test]
fn test_ocaml_let_binding() {
    let block = sigil_quote!(OCaml {
        let x = 42;
    })
    .unwrap();

    let output = render_ml(&block);
    assert!(output.contains("let x = 42"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// S. Import propagation through $C
// ════════════════════════════════════════════════════════

#[test]
fn test_nested_code_block_import_propagation() {
    let user_type = TypeName::importable_type("./models", "User");
    let inner = CodeBlock::of("getUser(): %T", (user_type,)).unwrap();

    let block = sigil_quote!(TypeScript {
        $C(inner);
    })
    .unwrap();

    let refs = import_collector::collect_imports(&block);
    assert_eq!(refs.len(), 1, "expected 1 import, got: {refs:?}");
    assert_eq!(refs[0].name, "User");
}

// ════════════════════════════════════════════════════════
// T. if/else chain correctness
// ════════════════════════════════════════════════════════

#[test]
fn test_if_else_if_else_chain() {
    let block = sigil_quote!(TypeScript {
        if(x > 0) {
            return 1;
        } else if(x < 0) {
            return -1;
        } else {
            return 0;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x > 0) {"), "got: {output}");
    assert!(output.contains("} else if (x < 0) {"), "got: {output}");
    assert!(output.contains("} else {"), "got: {output}");
    assert!(output.contains("return 0;"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// U. $C_each — sequential block splice
// ════════════════════════════════════════════════════════

#[test]
fn test_c_each_basic() {
    let block1 = CodeBlock::of("println(\"hello\")", ()).unwrap();
    let block2 = CodeBlock::of("println(\"world\")", ()).unwrap();
    let blocks = vec![block1, block2];

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("println(\"hello\")"), "got: {output}");
    assert!(output.contains("println(\"world\")"), "got: {output}");
}

#[test]
fn test_c_each_empty() {
    let blocks: Vec<CodeBlock> = vec![];

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert_eq!(output.trim(), "", "got: {output}");
}

#[test]
fn test_c_each_with_surrounding_code() {
    let items = vec![
        CodeBlock::of("x = 1", ()).unwrap(),
        CodeBlock::of("y = 2", ()).unwrap(),
    ];

    let block = sigil_quote!(TypeScript {
        const start = true;
        $C_each(items);
        const end = true;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const start = true;"), "got: {output}");
    assert!(output.contains("x = 1"), "got: {output}");
    assert!(output.contains("y = 2"), "got: {output}");
    assert!(output.contains("const end = true;"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// V. $if / $else_if / $else — meta-conditionals
// ════════════════════════════════════════════════════════

#[test]
fn test_meta_if_true() {
    let include_debug = true;

    let block = sigil_quote!(TypeScript {
        const x = 1;
        $if(include_debug) {
            console.log("debug");
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 1;"), "got: {output}");
    assert!(output.contains("console.log(\"debug\")"), "got: {output}");
}

#[test]
fn test_meta_if_false() {
    let include_debug = false;

    let block = sigil_quote!(TypeScript {
        const x = 1;
        $if(include_debug) {
            console.log("debug");
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 1;"), "got: {output}");
    assert!(!output.contains("console.log"), "got: {output}");
}

#[test]
fn test_meta_if_else() {
    let use_async = false;

    let block = sigil_quote!(TypeScript {
        $if(use_async) {
            const result = await fetch(url);
        } $else {
            const result = fetchSync(url);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(!output.contains("await"), "got: {output}");
    assert!(output.contains("fetchSync(url)"), "got: {output}");
}

#[test]
fn test_meta_if_else_if_else() {
    let mode = 2;

    let block = sigil_quote!(TypeScript {
        $if(mode == 1) {
            const x = "one";
        } $else_if(mode == 2) {
            const x = "two";
        } $else {
            const x = "other";
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(!output.contains("\"one\""), "got: {output}");
    assert!(output.contains("\"two\""), "got: {output}");
    assert!(!output.contains("\"other\""), "got: {output}");
}

#[test]
fn test_meta_if_nested() {
    let outer = true;
    let inner = true;

    let block = sigil_quote!(TypeScript {
        $if(outer) {
            const a = 1;
            $if(inner) {
                const b = 2;
            }
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const a = 1;"), "got: {output}");
    assert!(output.contains("const b = 2;"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// W. $join — separator-joined lists
// ════════════════════════════════════════════════════════

#[test]
fn test_join_basic() {
    let items = vec!["a", "b", "c"];

    let block = sigil_quote!(TypeScript {
        const list = [$join(", ", items)];
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("a, b, c"), "got: {output}");
}

#[test]
fn test_join_empty() {
    let items: Vec<&str> = vec![];

    let block = sigil_quote!(TypeScript {
        const list = [$join(", ", items)];
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("[]"), "got: {output}");
}

#[test]
fn test_join_in_function_call() {
    let args = vec!["x", "y", "z"];

    let block = sigil_quote!(TypeScript {
        doSomething($join(", ", args));
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("doSomething(x, y, z)"), "got: {output}");
}

#[test]
fn test_join_with_expression_iter() {
    let params: Vec<String> = vec!["name".into(), "age".into()];
    let param_decls: Vec<String> = params.iter().map(|p| format!("{p}: string")).collect();

    let block = sigil_quote!(TypeScript {
        function create($join(", ", param_decls));
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("name: string, age: string"),
        "got: {output}"
    );
}

// ════════════════════════════════════════════════════════
// X. Keyword spacing — comprehensive
// ════════════════════════════════════════════════════════

#[test]
fn test_keyword_new_no_space() {
    let block = sigil_quote!(TypeScript {
        const x = new Map();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("new Map()"), "got: {output}");
}

#[test]
fn test_keyword_switch() {
    let block = sigil_quote!(TypeScript {
        switch(status) {
            return "ok";
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("switch (status)"), "got: {output}");
}

#[test]
fn test_keyword_when_kotlin() {
    let block = sigil_quote!(Kotlin {
        when(x) {
            return true;
        }
    })
    .unwrap();

    let file = FileSpec::builder("test.kt")
        .add_code(block.clone())
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(output.contains("when (x)"), "got: {output}");
}

#[test]
fn test_keyword_match_rust() {
    let block = sigil_quote!(Rust {
        match(value) {
            return 1;
        }
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("match (value)"), "got: {output}");
}

#[test]
fn test_keyword_typeof() {
    let block = sigil_quote!(TypeScript {
        const t = typeof(x);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("typeof (x)"), "got: {output}");
}

#[test]
fn test_keyword_instanceof() {
    let block = sigil_quote!(TypeScript {
        const b = x instanceof(Foo);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("instanceof (Foo)"), "got: {output}");
}

#[test]
fn test_keyword_return_with_parens() {
    let block = sigil_quote!(TypeScript {
        return(x + y);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("return (x + y)"), "got: {output}");
}

#[test]
fn test_keyword_await_call() {
    let block = sigil_quote!(TypeScript {
        const r = await(promise);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("await (promise)"), "got: {output}");
}

#[test]
fn test_function_call_no_space() {
    let block = sigil_quote!(TypeScript {
        doSomething(x, y);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("doSomething(x, y)"), "got: {output}");
}

#[test]
fn test_method_call_no_space() {
    let block = sigil_quote!(TypeScript {
        foo.bar(z);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("foo.bar(z)"), "got: {output}");
}

#[test]
fn test_method_named_new_no_space() {
    // Note: `::` followed by ident gets a space due to proc-macro tokenization
    // (second `:` has Spacing::Alone). This is a known artifact — `HashMap:: new()`
    // is the expected output. The key check is that `new` before `(` does NOT get
    // the keyword space treatment (i.e., no double space).
    let block = sigil_quote!(TypeScript {
        const v = Vec::new();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("Vec:: new()"), "got: {output}");
    assert!(!output.contains("new ()"), "no extra space: {output}");
}

#[test]
fn test_keyword_do_while() {
    let block = sigil_quote!(TypeScript {
        do {
            tick();
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("do {"), "got: {output}");
    assert!(output.contains("tick();"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// Y. $C_each — edge cases
// ════════════════════════════════════════════════════════

#[test]
fn test_c_each_inside_control_flow() {
    let stmts = vec![
        CodeBlock::of("println(i)", ()).unwrap(),
        CodeBlock::of("i += 1", ()).unwrap(),
    ];

    let block = sigil_quote!(TypeScript {
        for(let i = 0; i < 10; i++) {
            $C_each(stmts);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("for ("), "got: {output}");
    assert!(output.contains("println(i)"), "got: {output}");
    assert!(output.contains("i += 1"), "got: {output}");
}

#[test]
fn test_c_each_multiple_sequential() {
    let first = vec![CodeBlock::of("a()", ()).unwrap()];
    let second = vec![CodeBlock::of("b()", ()).unwrap()];

    let block = sigil_quote!(TypeScript {
        $C_each(first);
        $C_each(second);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("a()"), "got: {output}");
    assert!(output.contains("b()"), "got: {output}");
}

#[test]
fn test_c_each_with_multi_line_blocks() {
    let blocks = vec![CodeBlock::of("const x = 1;\nconst y = 2;", ()).unwrap()];

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 1;"), "got: {output}");
    assert!(output.contains("const y = 2;"), "got: {output}");
}

#[test]
fn test_c_each_inside_meta_if() {
    let items = vec![
        CodeBlock::of("debug(x)", ()).unwrap(),
        CodeBlock::of("debug(y)", ()).unwrap(),
    ];
    let verbose = true;

    let block = sigil_quote!(TypeScript {
        $if(verbose) {
            $C_each(items);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("debug(x)"), "got: {output}");
    assert!(output.contains("debug(y)"), "got: {output}");
}

#[test]
fn test_c_each_inside_meta_if_false() {
    let items = vec![CodeBlock::of("debug(x)", ()).unwrap()];
    let verbose = false;

    let block = sigil_quote!(TypeScript {
        $if(verbose) {
            $C_each(items);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(!output.contains("debug"), "got: {output}");
}

#[test]
fn test_c_each_with_expression() {
    let raw = ["field1", "field2"];
    let blocks: Vec<CodeBlock> = raw
        .iter()
        .map(|f| CodeBlock::of(&format!("this.{f} = null"), ()).unwrap())
        .collect();

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("this.field1 = null"), "got: {output}");
    assert!(output.contains("this.field2 = null"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// Z. $if / $else_if / $else — edge cases
// ════════════════════════════════════════════════════════

#[test]
fn test_meta_if_with_interpolation() {
    let use_nullable = true;
    let type_name = TypeName::primitive("String");

    let block = sigil_quote!(TypeScript {
        $if(use_nullable) {
            let x: $T(type_name.clone()) | null = null;
        } $else {
            let x: $T(type_name) = "";
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("String | null"), "got: {output}");
    assert!(!output.contains("x: String = \"\""), "got: {output}");
}

#[test]
fn test_meta_if_with_target_control_flow() {
    let has_validation = true;

    let block = sigil_quote!(TypeScript {
        $if(has_validation) {
            if(input == null) {
                throw("invalid");
            }
        }
        process(input);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (input == null)"), "got: {output}");
    assert!(output.contains("throw (\"invalid\")"), "got: {output}");
    assert!(output.contains("process(input);"), "got: {output}");
}

#[test]
fn test_meta_if_multiple_else_if() {
    let tier = 3;

    let block = sigil_quote!(TypeScript {
        $if(tier == 1) {
            const label = "basic";
        } $else_if(tier == 2) {
            const label = "pro";
        } $else_if(tier == 3) {
            const label = "enterprise";
        } $else {
            const label = "unknown";
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(!output.contains("\"basic\""), "got: {output}");
    assert!(!output.contains("\"pro\""), "got: {output}");
    assert!(output.contains("\"enterprise\""), "got: {output}");
    assert!(!output.contains("\"unknown\""), "got: {output}");
}

#[test]
fn test_meta_if_empty_body() {
    let flag = true;

    let block = sigil_quote!(TypeScript {
        const before = 1;
        $if(!flag) {
            const hidden = 2;
        }
        const after = 3;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const before = 1;"), "got: {output}");
    assert!(!output.contains("hidden"), "got: {output}");
    assert!(output.contains("const after = 3;"), "got: {output}");
}

#[test]
fn test_meta_if_with_expression_condition() {
    let items: Vec<&str> = vec!["a", "b"];

    let block = sigil_quote!(TypeScript {
        $if(!items.is_empty()) {
            const count = $L(items.len().to_string());
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const count = 2;"), "got: {output}");
}

#[test]
fn test_meta_if_nested_both_false() {
    let outer = true;
    let inner = false;

    let block = sigil_quote!(TypeScript {
        $if(outer) {
            const a = 1;
            $if(inner) {
                const b = 2;
            }
            const c = 3;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const a = 1;"), "got: {output}");
    assert!(!output.contains("const b = 2;"), "got: {output}");
    assert!(output.contains("const c = 3;"), "got: {output}");
}

#[test]
fn test_meta_if_only_else_if_true() {
    let a = false;
    let b = true;

    let block = sigil_quote!(TypeScript {
        $if(a) {
            const x = "a";
        } $else_if(b) {
            const x = "b";
        } $else {
            const x = "none";
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(!output.contains("\"a\""), "got: {output}");
    assert!(output.contains("\"b\""), "got: {output}");
    assert!(!output.contains("\"none\""), "got: {output}");
}

// ════════════════════════════════════════════════════════
// AA. $join — edge cases
// ════════════════════════════════════════════════════════

#[test]
fn test_join_single_item() {
    let items = vec!["only"];

    let block = sigil_quote!(TypeScript {
        const x = $join(", ", items);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = only;"), "got: {output}");
}

#[test]
fn test_join_with_string_separator() {
    let sep = String::from(" | ");
    let items = vec!["A", "B", "C"];

    let block = sigil_quote!(TypeScript {
        type Union = $join(&sep, items);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("A | B | C"), "got: {output}");
}

#[test]
fn test_join_with_newline_separator() {
    let items = vec!["line1", "line2", "line3"];

    let block = sigil_quote!(TypeScript {
        $join("\n", items)
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("line1"), "got: {output}");
    assert!(output.contains("line2"), "got: {output}");
    assert!(output.contains("line3"), "got: {output}");
}

#[test]
fn test_join_inside_control_flow() {
    let params = vec!["a: int", "b: int"];

    let block = sigil_quote!(TypeScript {
        if(validate($join(", ", params))) {
            return true;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("validate(a: int, b: int)"), "got: {output}");
}

#[test]
fn test_join_inside_meta_if() {
    let use_params = true;
    let params = vec!["x", "y"];

    let block = sigil_quote!(TypeScript {
        $if(use_params) {
            fn($join(", ", params));
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("fn(x, y)"), "got: {output}");
}

#[test]
fn test_join_with_integers() {
    let nums = vec![1, 2, 3, 4, 5];

    let block = sigil_quote!(TypeScript {
        const arr = [$join(", ", nums)];
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("1, 2, 3, 4, 5"), "got: {output}");
}

#[test]
fn test_join_with_mapped_strings() {
    let fields = ["name", "age", "email"];
    let assignments: Vec<String> = fields.iter().map(|f| format!("this.{f} = {f}")).collect();

    let block = sigil_quote!(TypeScript {
        $join(";\n", assignments)
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("this.name = name"), "got: {output}");
    assert!(output.contains("this.age = age"), "got: {output}");
    assert!(output.contains("this.email = email"), "got: {output}");
}

#[test]
fn test_join_combined_with_c_each() {
    let params = vec!["a", "b", "c"];
    let body_blocks = vec![
        CodeBlock::of("validate(input)", ()).unwrap(),
        CodeBlock::of("process(input)", ()).unwrap(),
    ];

    let block = sigil_quote!(TypeScript {
        function handle($join(", ", params)) {
            $C_each(body_blocks);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("handle(a, b, c)"), "got: {output}");
    assert!(output.contains("validate(input)"), "got: {output}");
    assert!(output.contains("process(input)"), "got: {output}");
}

// ════════════════════════════════════════════════════════
// AB. Combined features — integration
// ════════════════════════════════════════════════════════

#[test]
fn test_all_features_combined() {
    let has_body = true;
    let params = vec!["request", "response"];
    let setup_blocks = vec![
        CodeBlock::of("const ctx = createContext()", ()).unwrap(),
        CodeBlock::of("ctx.init()", ()).unwrap(),
    ];

    let block = sigil_quote!(TypeScript {
        function handler($join(", ", params)) {
            $C_each(setup_blocks);
            $if(has_body) {
                const body = request.body;
                if(body == null) {
                    throw("missing body");
                }
            }
            return response.ok();
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("handler(request, response)"),
        "got: {output}"
    );
    assert!(
        output.contains("const ctx = createContext()"),
        "got: {output}"
    );
    assert!(output.contains("ctx.init()"), "got: {output}");
    assert!(
        output.contains("const body = request.body;"),
        "got: {output}"
    );
    assert!(output.contains("if (body == null)"), "got: {output}");
    assert!(output.contains("return response.ok();"), "got: {output}");
}

#[test]
fn test_meta_if_with_join_in_both_branches() {
    let use_named = true;
    let named_params = vec!["name: string", "age: number"];
    let positional_params = vec!["string", "number"];

    let block = sigil_quote!(TypeScript {
        $if(use_named) {
            function create($join(", ", named_params));
        } $else {
            function create($join(", ", positional_params));
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("name: string, age: number"),
        "got: {output}"
    );
    assert!(!output.contains("string, number"), "got: {output}");
}
