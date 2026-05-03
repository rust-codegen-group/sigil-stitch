use super::helpers::*;

// --- Basic $join ---

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

// --- Edge cases ---

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
