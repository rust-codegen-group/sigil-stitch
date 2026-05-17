use super::golden;
use super::helpers::*;

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

#[test]
fn test_consecutive_specifiers_no_space() {
    let block = sigil_quote!(TypeScript {
        const x = $L("pre")$L("mid")$L("post");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("premidpost"),
        "consecutive $L should have no space between them, got: {output}"
    );
    golden::assert_golden("macro/quote_consecutive_specifiers.txt", &output);
}
