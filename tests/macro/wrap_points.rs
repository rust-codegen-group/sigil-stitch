use super::golden;
use super::helpers::*;

#[test]
fn test_wrap_point_in_params() {
    let config_type = TypeName::primitive("Config");
    let block = sigil_quote!(TypeScript {
        export async function createUser($W name: string,$W age: number,$W config: $T(config_type) $W): Promise<void> {
            return undefined;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("createUser"), "got: {output}");
    assert!(output.contains("Config"), "got: {output}");
}

#[test]
fn test_wrap_point_narrow_width() {
    let block = sigil_quote!(TypeScript {
        doSomething($W alpha,$W beta,$W gamma);
    })
    .unwrap();

    let file = FileSpec::builder("test.ts")
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(20).unwrap();
    assert!(output.contains("doSomething"), "got: {output}");
}

#[test]
fn test_wrap_point_no_double_space() {
    let block = sigil_quote!(TypeScript {
        createUser($W firstName, $W lastName);
    })
    .unwrap();

    let output = render_ts(&block);
    // %W produces a single space (soft break) — there should not be a double
    // space between the wrap point and the following token.
    assert!(
        output.contains("createUser( firstName"),
        "should have single space after $W, got: {output}"
    );
    assert!(
        !output.contains("createUser(  firstName"),
        "double space after $W, got: {output}"
    );
    golden::assert_golden("macro/quote_wrap_point.txt", &output);
}
