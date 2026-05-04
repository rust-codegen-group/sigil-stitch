use super::helpers::*;

// --- Basic $for loop ---

#[test]
fn test_meta_for_simple() {
    let fields = vec!["name", "age", "email"];

    let block = sigil_quote!(TypeScript {
        $for(field in &fields) {
            console.log($L(*field));
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("console.log(name)"), "got: {output}");
    assert!(output.contains("console.log(age)"), "got: {output}");
    assert!(output.contains("console.log(email)"), "got: {output}");
}

#[test]
fn test_meta_for_with_interpolation() {
    let fields = vec!["name", "age"];

    let block = sigil_quote!(TypeScript {
        $for(field in &fields) {
            this.$N(*field) = $N(*field);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("this.name = name;"), "got: {output}");
    assert!(output.contains("this.age = age;"), "got: {output}");
}

#[test]
fn test_meta_for_with_string_literal() {
    let items = vec!["hello", "world"];

    let block = sigil_quote!(TypeScript {
        $for(item in &items) {
            console.log($S(*item));
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("console.log('hello')"), "got: {output}");
    assert!(output.contains("console.log('world')"), "got: {output}");
}

#[test]
fn test_meta_for_with_type_interpolation() {
    let types = vec![TypeName::primitive("string"), TypeName::primitive("number")];

    let block = sigil_quote!(TypeScript {
        $for(ty in &types) {
            let x: $T(ty.clone());
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("let x: string;"), "got: {output}");
    assert!(output.contains("let x: number;"), "got: {output}");
}

// --- Destructuring patterns ---

#[test]
fn test_meta_for_destructuring() {
    let pairs = vec![("x", "number"), ("y", "string")];

    let block = sigil_quote!(TypeScript {
        $for((name, ty) in &pairs) {
            let $N(*name): $L(*ty);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("let x: number;"), "got: {output}");
    assert!(output.contains("let y: string;"), "got: {output}");
}

// --- Nesting ---

#[test]
fn test_meta_for_nested_in_meta_if() {
    let include_fields = true;
    let fields = vec!["a", "b"];

    let block = sigil_quote!(TypeScript {
        $if(include_fields) {
            $for(field in &fields) {
                this.$N(*field) = null;
            }
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("this.a = null;"), "got: {output}");
    assert!(output.contains("this.b = null;"), "got: {output}");
}

#[test]
fn test_meta_if_nested_in_meta_for() {
    let fields: Vec<(&str, bool)> = vec![("name", true), ("age", false)];

    let block = sigil_quote!(TypeScript {
        $for((field, required) in &fields) {
            $if(*required) {
                this.$N(*field) = validate($L(*field));
            } $else {
                this.$N(*field) = $L(*field);
            }
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("this.name = validate(name);"),
        "got: {output}"
    );
    assert!(output.contains("this.age = age;"), "got: {output}");
}

#[test]
fn test_meta_for_nested_for() {
    let outer = vec!["A", "B"];
    let inner = vec!["1", "2"];

    let block = sigil_quote!(TypeScript {
        $for(o in &outer) {
            $for(i in &inner) {
                const $N(format!("{}_{}", o, i)) = true;
            }
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const A_1 = true;"), "got: {output}");
    assert!(output.contains("const A_2 = true;"), "got: {output}");
    assert!(output.contains("const B_1 = true;"), "got: {output}");
    assert!(output.contains("const B_2 = true;"), "got: {output}");
}

// --- Empty iteration ---

#[test]
fn test_meta_for_empty_iter() {
    let fields: Vec<&str> = vec![];

    let block = sigil_quote!(TypeScript {
        const before = 1;
        $for(field in &fields) {
            const $N(*field) = null;
        }
        const after = 2;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const before = 1;"), "got: {output}");
    assert!(output.contains("const after = 2;"), "got: {output}");
    // No field lines should appear.
    assert!(!output.contains("null"), "got: {output}");
}

// --- Real-world pattern: enum variant generation ---

#[test]
fn test_meta_for_enum_variants() {
    let variants = vec![("Red", "0"), ("Green", "1"), ("Blue", "2")];

    let block = sigil_quote!(TypeScript {
        $for((name, value) in &variants) {
            $L(*name) = $L(*value),
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("Red = 0,"), "got: {output}");
    assert!(output.contains("Green = 1,"), "got: {output}");
    assert!(output.contains("Blue = 2,"), "got: {output}");
}
