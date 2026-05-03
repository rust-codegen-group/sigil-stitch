use super::helpers::*;

// --- Basic meta-conditionals ---

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

// --- Edge cases ---

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
