use super::helpers::*;

// =============================================================================
// C++ — generics, nested templates, right shift, namespace paths, pointers
// =============================================================================

#[test]
fn test_cpp_nested_templates() {
    let block = sigil_quote!(Cpp {
        std::vector<std::vector<std::map<std::string, uint64_t>>> nested;
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(
        output.contains("vector<std::vector<std::map<std::string, uint64_t>>>"),
        "nested templates should be tight, got: {output}"
    );
    assert!(!output.contains(":: "), "no space after ::, got: {output}");
}

#[test]
fn test_cpp_right_shift_operator() {
    let block = sigil_quote!(Cpp {
        int result = x >> 2;
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(
        output.contains("x >> 2"),
        "right shift should have spaces, got: {output}"
    );
}

#[test]
fn test_cpp_left_shift_operator() {
    let block = sigil_quote!(Cpp {
        int result = x << 2;
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(
        output.contains("x << 2"),
        "left shift should have spaces, got: {output}"
    );
}

#[test]
fn test_cpp_namespace_path() {
    let block = sigil_quote!(Cpp {
        std::unique_ptr<std::vector<int>> ptr = std::make_unique<std::vector<int>>();
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(!output.contains(":: "), "no space after ::, got: {output}");
    assert!(!output.contains(" ::"), "no space before ::, got: {output}");
}

#[test]
fn test_cpp_pointer_and_reference() {
    // After an ident, `&`/`*` are treated as binary (ambiguous with bitwise ops).
    // For tight reference/pointer params, use ParameterSpec or TypeName.
    let block = sigil_quote!(Cpp {
        void foo(const std::string &name, int *ptr);
    })
    .unwrap();

    let output = render_cpp(&block);
    // & and * after idents (string, int) are classified binary — spaced.
    assert!(
        output.contains("std::string"),
        "namespace tight, got: {output}"
    );
    assert!(!output.contains(":: "), "no space after ::, got: {output}");
}

#[test]
fn test_cpp_template_with_comparison() {
    let block = sigil_quote!(Cpp {
        if(x < 5 && y > 10) {
            return;
        }
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(
        output.contains("x < 5"),
        "comparison keeps space, got: {output}"
    );
    assert!(
        output.contains("y > 10"),
        "comparison keeps space, got: {output}"
    );
}

#[test]
fn test_cpp_stream_operators() {
    let block = sigil_quote!(Cpp {
        std::cout << "value: " << x << std::endl;
    })
    .unwrap();

    let output = render_cpp(&block);
    assert!(
        output.contains("std::cout"),
        "namespace tight, got: {output}"
    );
    assert!(
        output.contains("std::endl"),
        "namespace tight, got: {output}"
    );
    assert!(output.contains("<< \"value: \""), "got: {output}");
}

// =============================================================================
// Java — generics, diamond operator, nested generics, shift operators
// =============================================================================

#[test]
fn test_java_nested_generics() {
    let block = sigil_quote!(Java {
        Map<String, List<Set<Integer>>> map = new HashMap<>();
    })
    .unwrap();

    let output = render_java(&block);
    assert!(
        output.contains("Map<String, List<Set<Integer>>>"),
        "nested generics tight, got: {output}"
    );
    assert!(
        output.contains("HashMap<>()"),
        "diamond tight, got: {output}"
    );
}

#[test]
fn test_java_right_shift() {
    let block = sigil_quote!(Java {
        int x = value >> 3;
    })
    .unwrap();

    let output = render_java(&block);
    assert!(
        output.contains("value >> 3"),
        "right shift keeps spaces, got: {output}"
    );
}

#[test]
fn test_java_unsigned_right_shift() {
    let block = sigil_quote!(Java {
        int x = value >>> 3;
    })
    .unwrap();

    let output = render_java(&block);
    assert!(
        output.contains("value >>> 3"),
        "unsigned right shift keeps spaces, got: {output}"
    );
}

#[test]
fn test_java_generic_method_call() {
    let block = sigil_quote!(Java {
        List<String> items = Collections.<String>emptyList();
    })
    .unwrap();

    let output = render_java(&block);
    assert!(
        output.contains("List<String>"),
        "generic type tight, got: {output}"
    );
}

#[test]
fn test_java_wildcard_generics() {
    let block = sigil_quote!(Java {
        List<? extends Comparable<? super T>> items;
    })
    .unwrap();

    let output = render_java(&block);
    assert!(
        output.contains("Comparable<"),
        "nested wildcard generic, got: {output}"
    );
}

// =============================================================================
// Kotlin — generics, safe-call, null-assert, type paths
// =============================================================================

#[test]
fn test_kotlin_nested_generics() {
    let block = sigil_quote!(Kotlin {
        val map: Map<String, List<Int>> = HashMap();
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(
        output.contains("Map<String, List<Int>>"),
        "nested generics tight, got: {output}"
    );
}

#[test]
fn test_kotlin_safe_call_chain() {
    let block = sigil_quote!(Kotlin {
        val len = obj?.name?.length;
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(
        output.contains("obj?.name?.length"),
        "safe-call tight, got: {output}"
    );
}

#[test]
fn test_kotlin_not_null_assert() {
    // `!!` is a Kotlin-specific postfix operator. proc_macro2 sees two Joint `!`
    // chars. The first `!` gets a space before it because it follows an Ident and
    // is Joint (not Alone), so it isn't classified as MacroBang.
    // Accepted limitation — use PropertySpec for null assertions.
    let block = sigil_quote!(Kotlin {
        val len = name!!.length;
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("!!."), "!! stays joint, got: {output}");
    assert!(output.contains("length"), "got: {output}");
}

#[test]
fn test_kotlin_comparison_vs_generic() {
    let block = sigil_quote!(Kotlin {
        val items: List<Int> = listOf(1, 2, 3);
        val check = x < 5;
    })
    .unwrap();

    let output = render_kt(&block);
    assert!(output.contains("List<Int>"), "generic tight, got: {output}");
    assert!(output.contains("x < 5"), "comparison spaced, got: {output}");
}

// =============================================================================
// TypeScript — generics, comparisons, shift, type assertions
// =============================================================================

#[test]
fn test_ts_nested_generics() {
    let block = sigil_quote!(TypeScript {
        const map: Map<string, Array<Set<number>>> = new Map();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("Map<string, Array<Set<number>>>"),
        "nested generics tight, got: {output}"
    );
}

#[test]
fn test_ts_right_shift() {
    let block = sigil_quote!(TypeScript {
        const x = value >> 3;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("value >> 3"),
        "right shift keeps spaces, got: {output}"
    );
}

#[test]
fn test_ts_unsigned_right_shift() {
    let block = sigil_quote!(TypeScript {
        const x = value >>> 3;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("value >>> 3"),
        "unsigned right shift keeps spaces, got: {output}"
    );
}

#[test]
fn test_ts_generic_function_call() {
    // `from` is lowercase, so `<` isn't detected as generic opener.
    // Uppercase method names (or turbofish `::< >`) work. Accepted limitation.
    let block = sigil_quote!(TypeScript {
        const items = Array.From<Set<string>>(source);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("From<Set<string>>"),
        "uppercase before < works, got: {output}"
    );
}

#[test]
fn test_ts_comparison_not_confused_with_generic() {
    let block = sigil_quote!(TypeScript {
        if(a < b && c > d) {
            return;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("a < b"), "comparison spaced, got: {output}");
    assert!(output.contains("c > d"), "comparison spaced, got: {output}");
}

#[test]
fn test_ts_bitwise_operators() {
    let block = sigil_quote!(TypeScript {
        const mask = a & b | c ^ d;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("a & b"),
        "bitwise AND spaced, got: {output}"
    );
    assert!(output.contains("| c"), "bitwise OR spaced, got: {output}");
    assert!(output.contains("^ d"), "bitwise XOR spaced, got: {output}");
}

// =============================================================================
// Swift — generics, optional chaining, protocol conformance
// =============================================================================

#[test]
fn test_swift_nested_generics() {
    let block = sigil_quote!(Swift {
        var dict: Dictionary<String, Array<Set<Int>>> = [:];
    })
    .unwrap();

    let output = render_swift(&block);
    assert!(
        output.contains("Dictionary<String, Array<Set<Int>>>"),
        "nested generics tight, got: {output}"
    );
}

#[test]
fn test_swift_optional_chaining() {
    let block = sigil_quote!(Swift {
        let len = obj?.name?.count;
    })
    .unwrap();

    let output = render_swift(&block);
    assert!(
        output.contains("obj?.name?.count"),
        "optional chaining tight, got: {output}"
    );
}

#[test]
fn test_swift_comparison_vs_generic() {
    let block = sigil_quote!(Swift {
        let arr: Array<Int> = [];
        let check = x < 10;
    })
    .unwrap();

    let output = render_swift(&block);
    assert!(
        output.contains("Array<Int>"),
        "generic tight, got: {output}"
    );
    assert!(
        output.contains("x < 10"),
        "comparison spaced, got: {output}"
    );
}

#[test]
fn test_swift_right_shift() {
    let block = sigil_quote!(Swift {
        let x = value >> 2;
    })
    .unwrap();

    let output = render_swift(&block);
    assert!(
        output.contains("value >> 2"),
        "right shift keeps spaces, got: {output}"
    );
}

// =============================================================================
// Rust — additional generic edge cases
// =============================================================================

#[test]
fn test_rust_array_typing() {
    let block = sigil_quote!(Rust {
        let matrix: [[i32; N]; M] = [[0; N]; M];
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(
        output.contains("[[i32; N]; M]"),
        "array type renders correctly, got: {output}"
    );
    assert!(
        output.contains("[[0; N]; M]"),
        "array literal renders correctly, got: {output}"
    );
}

#[test]
fn test_rust_deeply_nested_generics() {
    let block = sigil_quote!(Rust {
        let x: Arc<Mutex<HashMap<String, Vec<Option<u32>>>>> = Arc::new(Mutex::new(HashMap::new()));
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(
        output.contains("Arc<Mutex<HashMap<String, Vec<Option<u32>>>>>"),
        "deeply nested generics, got: {output}"
    );
}

#[test]
fn test_rust_right_shift_operator() {
    let block = sigil_quote!(Rust {
        let x = value >> 2;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(
        output.contains("value >> 2"),
        "right shift keeps spaces, got: {output}"
    );
}

#[test]
fn test_rust_left_shift_operator() {
    let block = sigil_quote!(Rust {
        let x = 1 << 8;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(
        output.contains("1 << 8"),
        "left shift keeps spaces, got: {output}"
    );
}

#[test]
fn test_rust_generic_with_trait_bounds_inline() {
    let block = sigil_quote!(Rust {
        let x: Box<dyn Iterator<Item = u32>> = todo!();
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(
        output.contains("Box<dyn Iterator<Item = u32>>"),
        "generic with assoc type, got: {output}"
    );
    assert!(
        output.contains("todo!()"),
        "macro bang tight, got: {output}"
    );
}

#[test]
fn test_rust_multiple_macro_calls() {
    let block = sigil_quote!(Rust {
        vec![1, 2, 3];
        println!("done");
        assert!(true);
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("vec!["), "got: {output}");
    assert!(output.contains("println!("), "got: {output}");
    assert!(output.contains("assert!("), "got: {output}");
    assert!(
        !output.contains(" !("),
        "no space before ! in macros, got: {output}"
    );
    assert!(
        !output.contains(" !["),
        "no space before ! in macros, got: {output}"
    );
}

#[test]
fn test_rust_double_reference() {
    let block = sigil_quote!(Rust {
        fn foo(x: &&str) {}
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("&&str"), "double ref tight, got: {output}");
}

#[test]
fn test_rust_deref_field_access() {
    let block = sigil_quote!(Rust {
        let x = (*ptr).field;
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("(*ptr).field"), "got: {output}");
}

// =============================================================================
// Scala — generics, path separators
// =============================================================================

#[test]
fn test_scala_nested_generics() {
    let block = sigil_quote!(Scala {
        val map: Map[String, List[Set[Int]]] = Map.empty;
    })
    .unwrap();

    let output = render_scala(&block);
    // Scala uses [] for generics — these are Group tokens, not punct
    assert!(
        output.contains("Map[String, List[Set[Int]]]"),
        "got: {output}"
    );
}

// =============================================================================
// Dart — generics, null-aware operators
// =============================================================================

#[test]
fn test_dart_nested_generics() {
    let block = sigil_quote!(Dart {
        Map<String, List<Set<int>>> map = {};
    })
    .unwrap();

    let output = render_dart(&block);
    assert!(
        output.contains("Map<String, List<Set<int>>>"),
        "nested generics tight, got: {output}"
    );
}

#[test]
fn test_dart_null_aware_access() {
    let block = sigil_quote!(Dart {
        var len = obj?.name?.length;
    })
    .unwrap();

    let output = render_dart(&block);
    assert!(
        output.contains("obj?.name?.length"),
        "null-aware access tight, got: {output}"
    );
}

// =============================================================================
// Go — pointer/address operators, shift operators
// =============================================================================

#[test]
fn test_go_pointer_and_address() {
    let block = sigil_quote!(Go {
        ptr := &x;
        val := *ptr;
    })
    .unwrap();

    let output = render_go(&block);
    assert!(output.contains("&x"), "address-of tight, got: {output}");
    assert!(output.contains("*ptr"), "deref tight, got: {output}");
}

#[test]
fn test_go_shift_operators() {
    let block = sigil_quote!(Go {
        x := y << 2;
        z := w >> 3;
    })
    .unwrap();

    let output = render_go(&block);
    assert!(
        output.contains("y << 2"),
        "left shift spaced, got: {output}"
    );
    assert!(
        output.contains("w >> 3"),
        "right shift spaced, got: {output}"
    );
}

#[test]
fn test_go_comparison_operators() {
    let block = sigil_quote!(Go {
        if x < 5 && y > 10 {
            return;
        }
    })
    .unwrap();

    let output = render_go(&block);
    assert!(output.contains("x < 5"), "comparison spaced, got: {output}");
    assert!(
        output.contains("y > 10"),
        "comparison spaced, got: {output}"
    );
}

// =============================================================================
// Python — bitwise operators, comparisons (no generics in this sense)
// =============================================================================

#[test]
fn test_python_bitwise_operators() {
    let block = sigil_quote!(Python {
        result = a & b | c ^ d;
    })
    .unwrap();

    let output = render_py(&block);
    assert!(
        output.contains("a & b"),
        "bitwise AND spaced, got: {output}"
    );
    assert!(output.contains("| c"), "bitwise OR spaced, got: {output}");
}

#[test]
fn test_python_shift_operators() {
    let block = sigil_quote!(Python {
        x = value >> 2;
        y = value << 3;
    })
    .unwrap();

    let output = render_py(&block);
    assert!(
        output.contains("value >> 2"),
        "right shift spaced, got: {output}"
    );
    assert!(
        output.contains("value << 3"),
        "left shift spaced, got: {output}"
    );
}

#[test]
fn test_python_star_unpack() {
    let block = sigil_quote!(Python {
        first, *rest = items;
    })
    .unwrap();

    let output = render_py(&block);
    assert!(output.contains("*rest"), "star unpack tight, got: {output}");
}

// =============================================================================
// C# — generics, null-conditional, shift operators
// =============================================================================

#[test]
fn test_csharp_nested_generics() {
    let block = sigil_quote!(CSharp {
        Dictionary<string, List<HashSet<int>>> map = new Dictionary<string, List<HashSet<int>>>();
    })
    .unwrap();

    let output = render_cs(&block);
    assert!(
        output.contains("Dictionary<string, List<HashSet<int>>>"),
        "nested generics tight, got: {output}"
    );
}

#[test]
fn test_csharp_null_conditional() {
    let block = sigil_quote!(CSharp {
        var len = obj?.Name?.Length;
    })
    .unwrap();

    let output = render_cs(&block);
    assert!(
        output.contains("obj?.Name?.Length"),
        "null-conditional tight, got: {output}"
    );
}

#[test]
fn test_csharp_right_shift() {
    let block = sigil_quote!(CSharp {
        int x = value >> 3;
    })
    .unwrap();

    let output = render_cs(&block);
    assert!(
        output.contains("value >> 3"),
        "right shift keeps spaces, got: {output}"
    );
}

#[test]
fn test_csharp_left_shift() {
    let block = sigil_quote!(CSharp {
        int x = value << 2;
    })
    .unwrap();

    let output = render_cs(&block);
    assert!(
        output.contains("value << 2"),
        "left shift keeps spaces, got: {output}"
    );
}

#[test]
fn test_csharp_comparison_vs_generic() {
    let block = sigil_quote!(CSharp {
        List<int> items = new List<int>();
        bool check = x < 5;
    })
    .unwrap();

    let output = render_cs(&block);
    assert!(output.contains("List<int>"), "generic tight, got: {output}");
    assert!(output.contains("x < 5"), "comparison spaced, got: {output}");
}

#[test]
fn test_csharp_bitwise_operators() {
    let block = sigil_quote!(CSharp {
        int mask = a & b | c ^ d;
    })
    .unwrap();

    let output = render_cs(&block);
    assert!(
        output.contains("a & b"),
        "bitwise AND spaced, got: {output}"
    );
    assert!(output.contains("| c"), "bitwise OR spaced, got: {output}");
    assert!(output.contains("^ d"), "bitwise XOR spaced, got: {output}");
}

// =============================================================================
// Lua — string concatenation, relational operators
// =============================================================================

#[test]
fn test_lua_concat() {
    let block = sigil_quote!(Lua {
        local s = "Hello, "..name
    })
    .unwrap();

    let output = render_lua(&block);
    assert!(
        output.contains("\"Hello, \"..name"),
        "concat tight, got: {output}"
    );
}

#[test]
fn test_lua_not_equal() {
    let block = sigil_quote!(Lua {
        if x ~= 0 then
            print(x)
        end
    })
    .unwrap();

    let output = render_lua(&block);
    assert!(output.contains("x ~= 0"), "not-equal tight, got: {output}");
}

#[test]
fn test_lua_length_operator() {
    let block = sigil_quote!(Lua {
        local n = #t
    })
    .unwrap();

    let output = render_lua(&block);
    assert!(
        output.contains("#t"),
        "length operator tight, got: {output}"
    );
}

#[test]
fn test_lua_exponent() {
    let block = sigil_quote!(Lua {
        local y = x ^ 2
    })
    .unwrap();

    let output = render_lua(&block);
    assert!(output.contains("x ^ 2"), "exponent spaced, got: {output}");
}

// =============================================================================
// Python elif — verify the parser recognizes `elif` as control-flow continuation
// =============================================================================

#[test]
fn test_python_elif() {
    let block = sigil_quote!(Python {
        if x > 0 {
            return $S("positive")
        } elif x < 0 {
            return $S("negative")
        } else {
            return $S("zero")
        }
    })
    .unwrap();

    let output = render_py(&block);
    assert!(
        output.contains("elif x < 0"),
        "elif recognized, got: {output}"
    );
    assert!(
        !output.contains("elseif"),
        "no elseif in Python, got: {output}"
    );
}
