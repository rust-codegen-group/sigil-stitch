use super::helpers::*;

// ============================================================
// Basic $let
// ============================================================

#[test]
fn test_meta_let_basic() {
    let block = sigil_quote!(TypeScript {
        $let(x = "world");
        console.log($S(x));
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("console.log('world')"), "got: {output}");
}

#[test]
fn test_meta_let_string_computation() {
    let name = "hello";
    let block = sigil_quote!(TypeScript {
        $let(upper = name.to_uppercase());
        const greeting = $S(upper);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("const greeting = 'HELLO';"),
        "got: {output}"
    );
}

#[test]
fn test_meta_let_emits_no_target_code() {
    let block = sigil_quote!(TypeScript {
        $let(x = 42);
        $let(_y = x + 1);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.trim().is_empty(),
        "expected no output lines, got: {output}"
    );
}

// ============================================================
// Binding patterns
// ============================================================

#[test]
fn test_meta_let_destructuring() {
    let pair = ("width", "100");
    let block = sigil_quote!(TypeScript {
        $let((name, value) = pair);
        const $N(name) = $L(value);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const width = 100;"), "got: {output}");
}

#[test]
fn test_meta_let_with_type_annotation() {
    let block = sigil_quote!(TypeScript {
        $let(x: &str = "computed");
        const result = $S(x);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("const result = 'computed';"),
        "got: {output}"
    );
}

#[test]
fn test_meta_let_with_enumerate() {
    let items = ["a", "b", "c"];
    let block = sigil_quote!(TypeScript {
        $for((i, item) in items.iter().enumerate()) {
            const $N(format!("item_{i}")) = $S(*item);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const item_0 = 'a';"), "got: {output}");
    assert!(output.contains("const item_1 = 'b';"), "got: {output}");
    assert!(output.contains("const item_2 = 'c';"), "got: {output}");
}

// ============================================================
// Chaining — one $let uses a previous $let's binding
// ============================================================

#[test]
fn test_meta_let_chaining() {
    let raw = "hello_world";
    let block = sigil_quote!(TypeScript {
        $let(parts: Vec<&str> = raw.split('_').collect());
        $let(capitalized: Vec<String> = parts.iter().map(|p| {
            let mut c = p.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        }).collect());
        $let(pascal = capitalized.join(""));
        const name = $S(pascal);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        output.contains("const name = 'HelloWorld';"),
        "got: {output}"
    );
}

// ============================================================
// All interpolation kinds with $let
// ============================================================

#[test]
fn test_meta_let_with_name_interpolation() {
    let prefix = "my";
    let block = sigil_quote!(TypeScript {
        $let(var_name = format!("{prefix}Var"));
        const $N(var_name) = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const myVar = 0;"), "got: {output}");
}

#[test]
fn test_meta_let_with_type_interpolation() {
    let block = sigil_quote!(TypeScript {
        $let(ty = TypeName::primitive("number"));
        const x: $T(ty) = 0;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x: number = 0;"), "got: {output}");
}

#[test]
fn test_meta_let_with_code_block_interpolation() {
    let block = sigil_quote!(TypeScript {
        $let(inner = CodeBlock::of("doSomething()", ()).unwrap());
        $C(inner);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("doSomething()"), "got: {output}");
}

// ============================================================
// $let inside control structures
// ============================================================

#[test]
fn test_meta_let_in_for_with_conditional() {
    let variants = vec![("red", "Red"), ("green_ish", "GreenIsh"), ("blue", "Blue")];

    let block = sigil_quote!(RustLang {
        $for((raw, pascal) in &variants) {
            $let(needs_rename = *raw != pascal.to_lowercase());
            $if(needs_rename) {
                #[serde(rename = $S(*raw))]
            }
            $L(format!("{pascal},"))
        }
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(
        output.contains("#[serde(rename = \"green_ish\")]"),
        "got: {output}"
    );
    assert!(output.contains("Red,"), "got: {output}");
    assert!(output.contains("GreenIsh,"), "got: {output}");
    assert!(output.contains("Blue,"), "got: {output}");
    assert!(
        !output.contains("#[serde(rename = \"red\")]"),
        "red should not need rename, got: {output}"
    );
    assert!(
        !output.contains("#[serde(rename = \"blue\")]"),
        "blue should not need rename, got: {output}"
    );
}

#[test]
fn test_meta_let_in_if_body() {
    let use_prefix = true;
    let name = "user";
    let block = sigil_quote!(TypeScript {
        $if(use_prefix) {
            $let(full_name = format!("prefix_{name}"));
            const x = $S(full_name);
        } $else {
            const x = $S(name);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 'prefix_user';"), "got: {output}");
}

#[test]
fn test_meta_let_in_nested_for() {
    let groups = vec![("math", vec!["add", "sub"]), ("str", vec!["len", "trim"])];

    let block = sigil_quote!(TypeScript {
        $for((module, fns) in &groups) {
            $let(mod_upper = module.to_uppercase());
            $for(f in fns) {
                $let(full_name = format!("{mod_upper}_{}", f.to_uppercase()));
                const $N(full_name) = true;
            }
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const MATH_ADD = true;"), "got: {output}");
    assert!(output.contains("const MATH_SUB = true;"), "got: {output}");
    assert!(output.contains("const STR_LEN = true;"), "got: {output}");
    assert!(output.contains("const STR_TRIM = true;"), "got: {output}");
}

#[test]
fn test_meta_let_in_for_with_empty_iter() {
    let items: Vec<&str> = vec![];
    let block = sigil_quote!(TypeScript {
        const before = 1;
        $for(item in &items) {
            $let(upper = item.to_uppercase());
            const $N(upper) = true;
        }
        const after = 2;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const before = 1;"), "got: {output}");
    assert!(output.contains("const after = 2;"), "got: {output}");
    assert_eq!(
        output.matches("const").count(),
        2,
        "should have exactly 2 const lines, got: {output}"
    );
}

// ============================================================
// Shadowing
// ============================================================

#[test]
fn test_meta_let_shadowing() {
    let block = sigil_quote!(TypeScript {
        $let(x = "first");
        const a = $S(x);
        $let(x = "second");
        const b = $S(x);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const a = 'first';"), "got: {output}");
    assert!(output.contains("const b = 'second';"), "got: {output}");
}

#[test]
fn test_meta_let_shadowing_in_for() {
    let items = vec![("a", "A"), ("b", "B")];
    let block = sigil_quote!(TypeScript {
        $for((lower, upper) in &items) {
            $let(x = format!("{lower}_{upper}"));
            const $N(x) = true;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const a_A = true;"), "got: {output}");
    assert!(output.contains("const b_B = true;"), "got: {output}");
}

// ============================================================
// Statement ordering — $let between target-language lines
// ============================================================

#[test]
fn test_meta_let_preserves_statement_order() {
    let block = sigil_quote!(TypeScript {
        const a = 1;
        $let(x = "middle");
        const b = $S(x);
        const c = 3;
    })
    .unwrap();

    let output = render_ts(&block);
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 3, "got: {output}");
    assert!(lines[0].contains("const a = 1;"), "got: {output}");
    assert!(lines[1].contains("const b = 'middle';"), "got: {output}");
    assert!(lines[2].contains("const c = 3;"), "got: {output}");
}

// ============================================================
// ? propagation — Option
// ============================================================

fn helper_question_mark_in_interpolation() -> Option<String> {
    let values: Vec<Option<&str>> = vec![Some("hello"), Some("world")];
    let block = sigil_quote!(TypeScript {
        $for(v in &values) {
            console.log($S(v.as_deref()?.to_uppercase()));
        }
    })
    .ok()?;

    Some(render_ts(&block))
}

#[test]
fn test_question_mark_in_interpolation() {
    let output = helper_question_mark_in_interpolation().unwrap();
    assert!(output.contains("console.log('HELLO')"), "got: {output}");
    assert!(output.contains("console.log('WORLD')"), "got: {output}");
}

fn helper_question_mark_returns_none() -> Option<String> {
    let values: Vec<Option<&str>> = vec![Some("hello"), None, Some("world")];
    let block = sigil_quote!(TypeScript {
        $for(v in &values) {
            console.log($S(v.as_deref()?.to_uppercase()));
        }
    })
    .ok()?;

    Some(render_ts(&block))
}

#[test]
fn test_question_mark_propagates_none() {
    let result = helper_question_mark_returns_none();
    assert!(result.is_none(), "should return None when ? triggers");
}

fn helper_let_with_question_mark() -> Option<String> {
    let values: Vec<Option<&str>> = vec![Some("hello"), Some("world")];
    let block = sigil_quote!(TypeScript {
        $for(v in &values) {
            $let(s = v.as_deref()?);
            $let(upper = s.to_uppercase());
            console.log($S(upper));
        }
    })
    .ok()?;

    Some(render_ts(&block))
}

#[test]
fn test_meta_let_with_question_mark() {
    let output = helper_let_with_question_mark().unwrap();
    assert!(output.contains("console.log('HELLO')"), "got: {output}");
    assert!(output.contains("console.log('WORLD')"), "got: {output}");
}

fn helper_let_question_mark_returns_none() -> Option<String> {
    let values: Vec<Option<&str>> = vec![Some("hello"), None];
    let block = sigil_quote!(TypeScript {
        $for(v in &values) {
            $let(s = v.as_deref()?);
            console.log($S(s.to_uppercase()));
        }
    })
    .ok()?;

    Some(render_ts(&block))
}

#[test]
fn test_meta_let_question_mark_propagates_none() {
    let result = helper_let_question_mark_returns_none();
    assert!(result.is_none(), "should return None when ? triggers");
}

// ============================================================
// ? propagation — Result
// ============================================================

fn helper_result_question_mark() -> Result<String, String> {
    let values: Vec<Result<&str, &str>> = vec![Ok("hello"), Ok("world")];
    let block = sigil_quote!(TypeScript {
        $for(v in &values) {
            $let(s = v.map_err(|e| e.to_string())?);
            console.log($S(s.to_uppercase()));
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(render_ts(&block))
}

#[test]
fn test_meta_let_with_result_question_mark() {
    let output = helper_result_question_mark().unwrap();
    assert!(output.contains("console.log('HELLO')"), "got: {output}");
    assert!(output.contains("console.log('WORLD')"), "got: {output}");
}

fn helper_result_question_mark_err() -> Result<String, String> {
    let values: Vec<Result<&str, &str>> = vec![Ok("hello"), Err("bad value"), Ok("world")];
    let block = sigil_quote!(TypeScript {
        $for(v in &values) {
            $let(s = v.map_err(|e| e.to_string())?);
            console.log($S(s.to_uppercase()));
        }
    })
    .map_err(|e| e.to_string())?;

    Ok(render_ts(&block))
}

#[test]
fn test_meta_let_result_question_mark_propagates_err() {
    let result = helper_result_question_mark_err();
    assert!(result.is_err(), "should propagate Err");
    assert_eq!(result.unwrap_err(), "bad value");
}

// ============================================================
// Multiple languages
// ============================================================

#[test]
fn test_meta_let_with_rust_lang() {
    let block = sigil_quote!(RustLang {
        $let(name = "MyStruct");
        $let(field_name = "value");
        pub struct $N(name) {
            pub $N(field_name): i32,
        }
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("pub struct MyStruct"), "got: {output}");
    assert!(output.contains("pub value: i32"), "got: {output}");
}

#[test]
fn test_meta_let_with_python() {
    let methods = vec!["get", "post", "delete"];
    let block = sigil_quote!(Python {
        $for(m in &methods) {
            $let(upper = m.to_uppercase());
            $N(upper) = $S(*m)
        }
    })
    .unwrap();

    let output = render_py(&block);
    assert!(output.contains("GET = 'get'"), "got: {output}");
    assert!(output.contains("POST = 'post'"), "got: {output}");
    assert!(output.contains("DELETE = 'delete'"), "got: {output}");
}

#[test]
fn test_meta_let_with_go() {
    let constants = vec![("StatusOK", "200"), ("StatusNotFound", "404")];
    let block = sigil_quote!(GoLang {
        $for((name, code) in &constants) {
            $let(full = format!("HTTP{name}"));
            const $N(full) = $L(*code);
        }
    })
    .unwrap();

    let output = render_go(&block);
    assert!(output.contains("const HTTPStatusOK = 200"), "got: {output}");
    assert!(
        output.contains("const HTTPStatusNotFound = 404"),
        "got: {output}"
    );
}

// ============================================================
// Real-world pattern: enum generation from the prompt
// ============================================================

#[test]
fn test_real_world_rust_string_enum() {
    let values = vec![
        ("red", Some("red")),
        ("green_olive", Some("green_olive")),
        ("BLUE", Some("BLUE")),
    ];

    let block = sigil_quote!(RustLang {
        $for((raw, json_val) in &values) {
            $let(pascal = raw
                .split('_')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + &c.as_str().to_lowercase(),
                    }
                })
                .collect::<Vec<_>>()
                .join(""));
            $if(json_val.is_some() && json_val.unwrap() != pascal.to_lowercase()) {
                $L(format!("#[serde(rename = \"{}\")]", json_val.unwrap()))
            }
            $L(format!("{pascal},"))
        }
    })
    .unwrap();

    let output = render_rs(&block);
    assert!(output.contains("Red,"), "got: {output}");
    assert!(output.contains("GreenOlive,"), "got: {output}");
    assert!(output.contains("Blue,"), "got: {output}");
    assert!(
        output.contains("#[serde(rename = \"green_olive\")]"),
        "got: {output}"
    );
    assert!(
        output.contains("#[serde(rename = \"BLUE\")]"),
        "got: {output}"
    );
}

fn helper_real_world_fallible_enum() -> Option<String> {
    use serde_json::Value;
    let values: Vec<Value> = vec![
        Value::String("red".into()),
        Value::String("green_olive".into()),
        Value::String("blue".into()),
    ];

    let block = sigil_quote!(RustLang {
        $for(v in &values) {
            $let(s = v.as_str()?);
            $let(variant = s
                .split('_')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + &c.as_str().to_lowercase(),
                    }
                })
                .collect::<Vec<_>>()
                .join(""));
            $if(s != variant.to_lowercase()) {
                $L(format!("#[serde(rename = \"{s}\")]"))
            }
            $L(format!("{variant},"))
        }
    })
    .ok()?;

    Some(render_rs(&block))
}

#[test]
fn test_real_world_fallible_enum_all_strings() {
    let output = helper_real_world_fallible_enum().unwrap();
    assert!(output.contains("Red,"), "got: {output}");
    assert!(output.contains("GreenOlive,"), "got: {output}");
    assert!(output.contains("Blue,"), "got: {output}");
    assert!(
        output.contains("#[serde(rename = \"green_olive\")]"),
        "got: {output}"
    );
}

fn helper_real_world_fallible_enum_with_non_string() -> Option<String> {
    use serde_json::Value;
    let values: Vec<Value> = vec![Value::String("red".into()), Value::Number(42.into())];

    let block = sigil_quote!(RustLang {
        $for(v in &values) {
            $let(s = v.as_str()?);
            $L(format!("{},", s))
        }
    })
    .ok()?;

    Some(render_rs(&block))
}

#[test]
fn test_real_world_fallible_enum_returns_none_on_non_string() {
    let result = helper_real_world_fallible_enum_with_non_string();
    assert!(
        result.is_none(),
        "should return None when encountering a non-string value"
    );
}

// ============================================================
// $let with $C_each combo
// ============================================================

#[test]
fn test_meta_let_building_blocks_for_c_each() {
    let fields = ["name", "age"];
    let block = sigil_quote!(TypeScript {
        $let(blocks: Vec<CodeBlock> = fields.iter().map(|f| {
            CodeBlock::of(&format!("this.{f} = null"), ()).unwrap()
        }).collect());
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("this.name = null"), "got: {output}");
    assert!(output.contains("this.age = null"), "got: {output}");
}

// ============================================================
// $let with target-language control flow
// ============================================================

#[test]
fn test_meta_let_with_target_control_flow() {
    let block = sigil_quote!(TypeScript {
        $let(var_name = "result");
        $let(check = "x > 0");
        if ($L(check)) {
            const $N(var_name) = true;
        } else {
            const $N(var_name) = false;
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("if (x > 0) {"), "got: {output}");
    assert!(output.contains("const result = true;"), "got: {output}");
    assert!(output.contains("const result = false;"), "got: {output}");
}

// ============================================================
// Edge: $let with complex Rust expressions
// ============================================================

#[test]
fn test_meta_let_with_match_expression() {
    let kind = "greeting";
    let block = sigil_quote!(TypeScript {
        $let(msg = match kind {
            "greeting" => "Hello!",
            "farewell" => "Goodbye!",
            _ => "...",
        });
        console.log($S(msg));
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("console.log('Hello!')"), "got: {output}");
}

#[test]
fn test_meta_let_with_if_expression() {
    let debug = true;
    let block = sigil_quote!(TypeScript {
        $let(level = if debug { "DEBUG" } else { "INFO" });
        console.log($S(level));
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("console.log('DEBUG')"), "got: {output}");
}

#[test]
fn test_meta_let_with_closure() {
    let block = sigil_quote!(TypeScript {
        $let(transform: Box<dyn Fn(&str) -> String> = Box::new(|s: &str| s.to_uppercase()));
        const x = $S(transform("hello"));
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 'HELLO';"), "got: {output}");
}
