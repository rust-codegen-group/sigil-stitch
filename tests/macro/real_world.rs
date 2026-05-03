use super::helpers::*;

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
    let block = sigil_quote!(TypeScript {
        const x: $T(TypeName::primitive("string")) = $S("hello");
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("string"), "got: {output}");
    assert!(output.contains("'hello'"), "got: {output}");
}
