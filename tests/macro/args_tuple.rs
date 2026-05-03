use super::helpers::*;

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
