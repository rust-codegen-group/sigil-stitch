use super::helpers::*;

#[test]
fn test_multiline_kotlin_block_body() {
    let block = sigil_quote!(Kotlin {
        if (x > 0) {
            val name = user.name
            println(name)
        }
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("val name = user.name\n"), "got: {output}");
    assert!(output.contains("println(name)\n"), "got: {output}");
}

#[test]
fn test_multiline_kotlin_top_level() {
    let block = sigil_quote!(Kotlin {
        val x = 1
        val y = 2
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("val x = 1\n"), "got: {output}");
    assert!(output.contains("val y = 2\n"), "got: {output}");
}

#[test]
fn test_same_line_tokens_stay_together() {
    let block = sigil_quote!(Kotlin {
        val result = x + y + z;
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("val result = x + y + z"), "got: {output}");
}

#[test]
fn test_continuation_marker_joins_lines() {
    let block = sigil_quote!(Haskell {
        mapM_ $+
            putStrLn $+
            items
    })
    .unwrap();

    let output = render_hs(&block);
    assert!(output.contains("mapM_ putStrLn items"), "got: {output}");
}

#[test]
fn test_continuation_in_kotlin_long_expression() {
    let block = sigil_quote!(Kotlin {
        val result = someFunction( $+
            arg1, arg2);
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("someFunction(arg1, arg2)"), "got: {output}");
}

#[test]
fn test_semicolons_still_work_with_line_splitting() {
    let block = sigil_quote!(Kotlin {
        val x = 1;
        val y = 2;
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("val x = 1\n"), "got: {output}");
    assert!(output.contains("val y = 2\n"), "got: {output}");
}

#[test]
fn test_continuation_at_end_of_input_is_stripped() {
    let block = sigil_quote!(Kotlin {
        val x = 1 $+
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("val x = 1\n"), "got: {output}");
    assert!(!output.contains('+'), "got: {output}");
}

#[test]
fn test_continuation_mid_line_is_noop() {
    let block = sigil_quote!(Kotlin {
        val x = $+ 1;
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("val x = 1"), "got: {output}");
    assert!(!output.contains('+'), "got: {output}");
}
