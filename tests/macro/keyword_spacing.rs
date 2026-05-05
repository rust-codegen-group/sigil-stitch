use super::helpers::*;

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
    let block = sigil_quote!(TypeScript {
        const v = Vec::new();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("Vec::new()"), "got: {output}");
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
