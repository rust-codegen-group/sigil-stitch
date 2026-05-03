use super::helpers::*;

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
