use super::helpers::*;

// --- Basic $C_each ---

#[test]
fn test_c_each_basic() {
    let block1 = CodeBlock::of("println(\"hello\")", ()).unwrap();
    let block2 = CodeBlock::of("println(\"world\")", ()).unwrap();
    let blocks = vec![block1, block2];

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("println(\"hello\")"), "got: {output}");
    assert!(output.contains("println(\"world\")"), "got: {output}");
}

#[test]
fn test_c_each_empty() {
    let blocks: Vec<CodeBlock> = vec![];

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert_eq!(output.trim(), "", "got: {output}");
}

#[test]
fn test_c_each_with_surrounding_code() {
    let items = vec![
        CodeBlock::of("x = 1", ()).unwrap(),
        CodeBlock::of("y = 2", ()).unwrap(),
    ];

    let block = sigil_quote!(TypeScript {
        const start = true;
        $C_each(items);
        const end = true;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const start = true;"), "got: {output}");
    assert!(output.contains("x = 1"), "got: {output}");
    assert!(output.contains("y = 2"), "got: {output}");
    assert!(output.contains("const end = true;"), "got: {output}");
}

// --- Edge cases ---

#[test]
fn test_c_each_inside_control_flow() {
    let stmts = vec![
        CodeBlock::of("println(i)", ()).unwrap(),
        CodeBlock::of("i += 1", ()).unwrap(),
    ];

    let block = sigil_quote!(TypeScript {
        for(let i = 0; i < 10; i++) {
            $C_each(stmts);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("for ("), "got: {output}");
    assert!(output.contains("println(i)"), "got: {output}");
    assert!(output.contains("i += 1"), "got: {output}");
}

#[test]
fn test_c_each_multiple_sequential() {
    let first = vec![CodeBlock::of("a()", ()).unwrap()];
    let second = vec![CodeBlock::of("b()", ()).unwrap()];

    let block = sigil_quote!(TypeScript {
        $C_each(first);
        $C_each(second);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("a()"), "got: {output}");
    assert!(output.contains("b()"), "got: {output}");
}

#[test]
fn test_c_each_with_multi_line_blocks() {
    let blocks = vec![CodeBlock::of("const x = 1;\nconst y = 2;", ()).unwrap()];

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const x = 1;"), "got: {output}");
    assert!(output.contains("const y = 2;"), "got: {output}");
}

#[test]
fn test_c_each_inside_meta_if() {
    let items = vec![
        CodeBlock::of("debug(x)", ()).unwrap(),
        CodeBlock::of("debug(y)", ()).unwrap(),
    ];
    let verbose = true;

    let block = sigil_quote!(TypeScript {
        $if(verbose) {
            $C_each(items);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("debug(x)"), "got: {output}");
    assert!(output.contains("debug(y)"), "got: {output}");
}

#[test]
fn test_c_each_inside_meta_if_false() {
    let items = vec![CodeBlock::of("debug(x)", ()).unwrap()];
    let verbose = false;

    let block = sigil_quote!(TypeScript {
        $if(verbose) {
            $C_each(items);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(!output.contains("debug"), "got: {output}");
}

#[test]
fn test_c_each_with_expression() {
    let raw = ["field1", "field2"];
    let blocks: Vec<CodeBlock> = raw
        .iter()
        .map(|f| CodeBlock::of(&format!("this.{f} = null"), ()).unwrap())
        .collect();

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("this.field1 = null"), "got: {output}");
    assert!(output.contains("this.field2 = null"), "got: {output}");
}

// --- Trailing blank line coverage ---

#[test]
fn test_c_each_with_add_statement_no_trailing_blank_line() {
    let fields = ["name", "age"];
    let blocks: Vec<CodeBlock> = fields
        .iter()
        .map(|f| {
            let mut cb = CodeBlock::builder();
            cb.add_statement(&format!("this.{f} = null"), ());
            cb.build().unwrap()
        })
        .collect();

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
        return this;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(
        !output.contains("this.age = null;\n\nreturn"),
        "should not have blank line between spliced blocks and next statement. got: {output}"
    );
    assert!(output.contains("this.name = null;"), "got: {output}");
    assert!(output.contains("this.age = null;"), "got: {output}");
    assert!(output.contains("return this;"), "got: {output}");
}

#[test]
fn test_c_each_with_code_block_of() {
    let fields = ["name", "age"];
    let blocks: Vec<CodeBlock> = fields
        .iter()
        .map(|f| CodeBlock::of(&format!("this.{f} = null"), ()).unwrap())
        .collect();

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
        return this;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("this.name = null"), "got: {output}");
    assert!(output.contains("this.age = null"), "got: {output}");
    assert!(output.contains("return this;"), "got: {output}");
}

// --- Comprehensive newline/blank-line coverage ---

#[test]
fn test_c_each_add_statement_blocks_no_double_newline() {
    let blocks: Vec<CodeBlock> = (0..3)
        .map(|i| {
            let mut cb = CodeBlock::builder();
            cb.add_statement(&format!("step{i}()"), ());
            cb.build().unwrap()
        })
        .collect();

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("step0();"), "got: {output}");
    assert!(output.contains("step1();"), "got: {output}");
    assert!(output.contains("step2();"), "got: {output}");
    assert!(
        !output.contains("\n\n"),
        "no blank lines between add_statement blocks. got: {output}"
    );
}

#[test]
fn test_c_each_code_block_of_gets_newlines() {
    let blocks: Vec<CodeBlock> = (0..3)
        .map(|i| CodeBlock::of(&format!("step{i}()"), ()).unwrap())
        .collect();

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("step0()"), "got: {output}");
    assert!(output.contains("step1()"), "got: {output}");
    assert!(output.contains("step2()"), "got: {output}");
}

#[test]
fn test_c_each_mixed_block_types() {
    let mut blocks: Vec<CodeBlock> = Vec::new();

    let mut cb = CodeBlock::builder();
    cb.add_statement("statement_block()", ());
    blocks.push(cb.build().unwrap());

    blocks.push(CodeBlock::of("of_block()", ()).unwrap());

    let mut cb = CodeBlock::builder();
    cb.add_statement("another_statement()", ());
    blocks.push(cb.build().unwrap());

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("statement_block();"), "got: {output}");
    assert!(output.contains("of_block()"), "got: {output}");
    assert!(output.contains("another_statement();"), "got: {output}");
}

#[test]
fn test_c_each_before_and_after_statements() {
    let blocks: Vec<CodeBlock> = vec![CodeBlock::of("middle()", ()).unwrap()];

    let block = sigil_quote!(TypeScript {
        const before = 1;
        $C_each(blocks);
        const after = 2;
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("const before = 1;"), "got: {output}");
    assert!(output.contains("middle()"), "got: {output}");
    assert!(output.contains("const after = 2;"), "got: {output}");
}

#[test]
fn test_c_each_single_add_statement_block() {
    let mut cb = CodeBlock::builder();
    cb.add_statement("only_one()", ());
    let blocks = vec![cb.build().unwrap()];

    let block = sigil_quote!(TypeScript {
        $C_each(blocks);
        done();
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("only_one();"), "got: {output}");
    assert!(output.contains("done();"), "got: {output}");
    assert!(
        !output.contains("only_one();\n\ndone"),
        "no blank line between single spliced block and next. got: {output}"
    );
}

#[test]
fn test_c_each_in_control_flow_no_blank_lines() {
    let assignments: Vec<CodeBlock> = ["x", "y"]
        .iter()
        .map(|f| {
            let mut cb = CodeBlock::builder();
            cb.add_statement(&format!("this.{f} = {f}"), ());
            cb.build().unwrap()
        })
        .collect();

    let block = sigil_quote!(TypeScript {
        if(init) {
            $C_each(assignments);
        }
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("this.x = x;"), "got: {output}");
    assert!(output.contains("this.y = y;"), "got: {output}");
}

#[test]
fn test_c_each_no_trailing_blank_line_in_fun_spec_body() {
    use sigil_stitch::lang::java_lang::JavaLang;

    let fields = ["statusCode", "raw", "status200"];
    let assignments: Vec<CodeBlock> = fields
        .iter()
        .map(|f| {
            sigil_quote!(JavaLang {
                this.$L(*f) = $L(*f);
            })
            .unwrap()
        })
        .collect();

    let ctor_body = sigil_quote!(JavaLang {
        $C_each(assignments);
    })
    .unwrap();

    let mut cls = TypeSpec::builder("TestClass", TypeKind::Struct).visibility(Visibility::Public);
    let mut ctor = FunSpec::builder("TestClass");
    ctor = ctor.visibility(Visibility::Public);
    ctor = ctor.body(ctor_body);
    cls = cls.add_method(ctor.build().unwrap());

    let file = FileSpec::builder_with("TestClass.java", JavaLang::new())
        .add_type(cls.build().unwrap())
        .build()
        .unwrap();
    let output = file.render(100).unwrap();

    assert!(
        !output.contains("status200;\n\n    }"),
        "trailing blank line before closing brace. got:\n{output}"
    );
    assert!(
        output.contains("this.statusCode = statusCode;"),
        "got:\n{output}"
    );
}
