use super::helpers::*;

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
    let macro_block = sigil_quote!(TypeScript {
        foo($W a,$W b,$W c);
    })
    .unwrap();

    let output = render_ts(&macro_block);
    assert!(output.contains("foo("), "got: {output}");
    assert!(output.contains("a,"), "got: {output}");
    assert!(output.contains("b,"), "got: {output}");
}

#[test]
fn test_name_escaping_via_macro_rust() {
    let field_name = "type";
    let macro_block = sigil_quote!(Rust {
        let $N(field_name) = value;
    })
    .unwrap();

    let output = render_rs(&macro_block);
    assert!(
        output.contains("r#type"),
        "Expected r#type in output: {output}"
    );
}

#[test]
fn test_name_escaping_via_macro_go() {
    let var_name = "func";
    let macro_block = sigil_quote!(Go {
        var $N(var_name) int
    })
    .unwrap();

    let output = render_go(&macro_block);
    assert!(
        output.contains("func_"),
        "Expected func_ in output: {output}"
    );
}

#[test]
fn test_name_no_escape_via_macro() {
    let name = "myVariable";
    let macro_block = sigil_quote!(TypeScript {
        const $N(name) = 42;
    })
    .unwrap();

    let output = render_ts(&macro_block);
    assert!(
        output.contains("myVariable"),
        "Expected myVariable in output: {output}"
    );
    assert!(
        !output.contains("myVariable_"),
        "Should not escape non-keyword: {output}"
    );
}

#[test]
fn test_name_and_type_combined() {
    let field_name = "type";
    let field_type = TypeName::primitive("String");
    let macro_block = sigil_quote!(Rust {
        pub $N(field_name): $T(field_type)
    })
    .unwrap();

    let output = render_rs(&macro_block);
    assert!(
        output.contains("pub r#type: String"),
        "Expected 'pub r#type: String', got: {output}"
    );
}
