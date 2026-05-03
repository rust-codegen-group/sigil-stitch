use super::helpers::*;

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
    let idx_a = output.find("const a = 1;").unwrap();
    let idx_b = output.find("const b = 2;").unwrap();
    let between = &output[idx_a + "const a = 1;".len()..idx_b];
    let newlines = between.chars().filter(|c| *c == '\n').count();
    assert!(
        newlines >= 3,
        "expected multiple blank lines, got {newlines} newlines in: {between:?}"
    );
}
