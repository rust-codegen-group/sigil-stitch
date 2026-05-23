use super::golden;
use super::helpers::*;

// --- Basic $for loop ---

#[test]
fn test_meta_for_simple() {
    let fields = vec!["name", "age", "email"];

    let block = sigil_quote!(TypeScript {
        $for(field in &fields) {
            console.log($N(*field));
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

// ── $T_join — type join with import tracking ─────────────

#[test]
fn test_t_join_basic() {
    let types = vec![
        TypeName::importable_type("./models", "User"),
        TypeName::importable_type("./models", "Admin"),
        TypeName::primitive("null"),
    ];

    let block = sigil_quote!(TypeScript {
        export type MyType = $T_join(" | ", &types);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("export type MyType = User | Admin | null;"),
        "got:\n{output}"
    );
    // Verify imports are tracked (dedup merges same-module imports)
    assert!(
        output.contains("import type { Admin, User } from './models'"),
        "imports should be tracked, got:\n{output}"
    );
}

#[test]
fn test_t_join_single_item() {
    let types = vec![TypeName::primitive("string")];

    let block = sigil_quote!(TypeScript {
        export type MyType = $T_join(" | ", &types);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("export type MyType = string;"),
        "got:\n{output}"
    );
}

#[test]
fn test_t_join_empty() {
    let types: Vec<TypeName> = vec![];

    let block = sigil_quote!(TypeScript {
        export type MyType = $T_join(" | ", &types);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("export type MyType = ;"), "got:\n{output}");
}

#[test]
fn test_t_join_typescript_intersection() {
    let types = vec![
        TypeName::importable_type("./types", "Serializable"),
        TypeName::importable_type("./types", "Comparable"),
    ];

    let block = sigil_quote!(TypeScript {
        export type MyType = $T_join(" & ", &types);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("export type MyType = Serializable & Comparable;"),
        "got:\n{output}"
    );
    assert!(
        output.contains("import type { Comparable, Serializable }"),
        "got:\n{output}"
    );
}

#[test]
fn test_t_join_python_union() {
    let types = vec![
        TypeName::primitive("str"),
        TypeName::primitive("int"),
        TypeName::primitive("None"),
    ];

    let block = sigil_quote!(Python {
        MyType = $T_join(" | ", &types)
    })
    .unwrap();

    let output = render_py(&block);
    assert!(
        output.contains("MyType = str | int | None"),
        "got:\n{output}"
    );
}

#[test]
fn test_t_join_swift_protocol() {
    let types = vec![
        TypeName::primitive("Codable"),
        TypeName::primitive("Hashable"),
    ];

    let block = sigil_quote!(Swift {
        typealias MyType = $T_join(" & ", &types)
    })
    .unwrap();

    let output = render_swift(&block);
    assert!(
        output.contains("typealias MyType = Codable & Hashable"),
        "got:\n{output}"
    );
}

#[test]
fn test_t_join_scala_mixin() {
    let types = vec![
        TypeName::primitive("A"),
        TypeName::primitive("B"),
        TypeName::primitive("C"),
    ];

    let block = sigil_quote!(Scala {
        trait Foo extends $T_join(" with ", &types)
    })
    .unwrap();

    let output = render_scala(&block);
    assert!(
        output.contains("trait Foo extends A with B with C"),
        "got:\n{output}"
    );
}

#[test]
fn test_t_join_rust_trait_bounds() {
    let traits = vec![
        TypeName::importable_type("./traits", "Serializable"),
        TypeName::importable_type("./traits", "Cloneable"),
        TypeName::primitive("Send"),
    ];

    let block = sigil_quote!(Rust {
        fn process(x: &(dyn $T_join(" + ", &traits))) -> Result<(), Error>;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(
        output.contains("fn process(x: &(dyn Serializable + Cloneable + Send))"),
        "got:\n{output}"
    );
    assert!(
        output.contains("use ./traits::{Cloneable, Serializable};"),
        "got:\n{output}"
    );
}

#[test]
fn test_t_join_java_intersection() {
    let bounds = vec![
        TypeName::importable_type("./io", "Serializable"),
        TypeName::importable_type("./util", "Comparable"),
    ];

    let block = sigil_quote!(Java {
        class Foo<T extends $T_join(" & ", &bounds)> {}
    })
    .unwrap();

    let output = render_java(&block);
    assert!(
        output.contains("class Foo<T extends Serializable & Comparable>"),
        "got:\n{output}"
    );
    assert!(
        output.contains("import ./io.Serializable;"),
        "got:\n{output}"
    );
    assert!(
        output.contains("import ./util.Comparable;"),
        "got:\n{output}"
    );
}

#[test]
fn test_t_join_dart_mixins() {
    let mixins = vec![
        TypeName::importable_type("./mixins", "JsonSerializable"),
        TypeName::importable_type("./mixins", "EquatableMixin"),
    ];

    let block = sigil_quote!(Dart {
        class User extends BaseModel with $T_join(", ", &mixins) {}
    })
    .unwrap();

    let output = render_dart(&block);
    assert!(
        output.contains("class User extends BaseModel with JsonSerializable, EquatableMixin"),
        "got:\n{output}"
    );
    assert!(output.contains("import './mixins'"), "got:\n{output}");
}

#[test]
fn test_t_join_kotlin_supertypes() {
    let interfaces = vec![
        TypeName::importable_type("./interfaces", "Serializable"),
        TypeName::importable_type("./interfaces", "Parcelable"),
    ];

    let block = sigil_quote!(Kotlin {
        class User : $T_join(", ", &interfaces)
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(
        output.contains("class User: Serializable, Parcelable"),
        "got:\n{output}"
    );
    assert!(
        output.contains("import ./interfaces.Parcelable"),
        "got:\n{output}"
    );
    assert!(
        output.contains("import ./interfaces.Serializable"),
        "got:\n{output}"
    );
}

#[test]
fn test_t_join_csharp_constraints() {
    let constraints = vec![
        TypeName::importable_type("./interfaces", "IDisposable"),
        TypeName::importable_type("./interfaces", "IComparable"),
    ];

    let block = sigil_quote!(CSharp {
        class Foo<T> where T : $T_join(", ", &constraints)
    })
    .unwrap();

    let output = render_cs(&block);
    assert!(
        output.contains("class Foo<T> where T: IDisposable, IComparable"),
        "got:\n{output}"
    );
    assert!(output.contains("using ./interfaces;"), "got:\n{output}");
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
                this.$N(*field) = validate($N(*field));
            } $else {
                this.$N(*field) = $N(*field);
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
            $N(*name) = $L(*value),
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("Red = 0,"), "got: {output}");
    assert!(output.contains("Green = 1,"), "got: {output}");
    assert!(output.contains("Blue = 2,"), "got: {output}");
}

#[test]
fn test_meta_for_golden() {
    let fields = vec!["name", "age", "email"];

    let block = sigil_quote!(TypeScript {
        $for(field in &fields) {
            console.log($N(*field));
        }
    })
    .unwrap();

    let output = render_ts(&block);
    golden::assert_golden("macro/quote_meta_for.txt", &output);
}
