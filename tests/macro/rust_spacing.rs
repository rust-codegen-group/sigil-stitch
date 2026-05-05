use super::helpers::*;

#[test]
fn test_path_separator_no_space() {
    let block = sigil_quote!(RustLang {
        let x = std::fmt::Display;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(!output.contains(":: "), "no space after ::, got: {output}");
    assert!(!output.contains(" ::"), "no space before ::, got: {output}");
    assert!(output.contains("std::fmt::Display"), "got: {output}");
}

#[test]
fn test_reference_prefix() {
    let block = sigil_quote!(RustLang {
        fn foo(&self, x: &mut T) {}
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("&self"), "got: {output}");
    assert!(output.contains("&mut"), "got: {output}");
    assert!(
        !output.contains("& self"),
        "no space after &, got: {output}"
    );
    assert!(!output.contains("& mut"), "no space after &, got: {output}");
}

#[test]
fn test_deref_prefix() {
    let block = sigil_quote!(RustLang {
        let x = *self;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("*self"), "got: {output}");
    assert!(
        !output.contains("* self"),
        "no space after *, got: {output}"
    );
}

#[test]
fn test_macro_call_no_space() {
    let block = sigil_quote!(RustLang {
        println!("hello");
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("println!("), "got: {output}");
    assert!(
        !output.contains("println !("),
        "no space before !, got: {output}"
    );
}

#[test]
fn test_generic_angle_brackets() {
    let block = sigil_quote!(RustLang {
        let x: Vec<T> = Vec::new();
        let y: Option<String> = None;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("Vec<T>"), "got: {output}");
    assert!(output.contains("Option<String>"), "got: {output}");
}

#[test]
fn test_turbofish() {
    let block = sigil_quote!(RustLang {
        let size = std::mem::size_of::<u32>();
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("size_of::<u32>()"), "got: {output}");
}

#[test]
fn test_nested_generics() {
    let block = sigil_quote!(RustLang {
        let x: HashMap<K, Vec<V>> = HashMap::new();
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("HashMap<K, Vec<V>>"), "got: {output}");
}

#[test]
fn test_lifetime_in_generic() {
    let block = sigil_quote!(RustLang {
        fn fmt(&self, f: &mut Formatter<'_>) {}
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("Formatter<'_>"), "got: {output}");
}

#[test]
fn test_generic_close_then_call() {
    let block = sigil_quote!(RustLang {
        let v = iter.collect::<Vec<i32>>();
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("collect::<Vec<i32>>()"), "got: {output}");
}

#[test]
fn test_binary_and_still_spaced() {
    let block = sigil_quote!(RustLang {
        let x = a & b;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("a & b"), "got: {output}");
}

#[test]
fn test_comparison_still_spaced() {
    let block = sigil_quote!(TypeScript {
        if(x < 5) {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("x < 5"), "got: {output}");
}

#[test]
fn test_arrow_return_type() {
    let block = sigil_quote!(RustLang {
        let f: fn() -> T = foo;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("-> T"), "got: {output}");
}
