use super::helpers::*;

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
