use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::code_node::BlockIntent;
use sigil_stitch::lang::CodeLang;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn languages() -> Vec<Box<dyn CodeLang>> {
    vec![
        Box::new(sigil_stitch::lang::bash::Bash::new()),
        Box::new(sigil_stitch::lang::c::C::new()),
        Box::new(sigil_stitch::lang::cpp::Cpp::new()),
        Box::new(sigil_stitch::lang::csharp::CSharp::new()),
        Box::new(sigil_stitch::lang::dart::Dart::new()),
        Box::new(sigil_stitch::lang::go::Go::new()),
        Box::new(sigil_stitch::lang::haskell::Haskell::new()),
        Box::new(sigil_stitch::lang::java::Java::new()),
        Box::new(sigil_stitch::lang::javascript::JavaScript::new()),
        Box::new(sigil_stitch::lang::kotlin::Kotlin::new()),
        Box::new(sigil_stitch::lang::lua::Lua::new()),
        Box::new(sigil_stitch::lang::ocaml::OCaml::new()),
        Box::new(sigil_stitch::lang::php::Php::new()),
        Box::new(sigil_stitch::lang::python::Python::new()),
        Box::new(sigil_stitch::lang::ruby::Ruby::new()),
        Box::new(sigil_stitch::lang::rust::Rust::new()),
        Box::new(sigil_stitch::lang::scala::Scala::new()),
        Box::new(sigil_stitch::lang::swift::Swift::new()),
        Box::new(sigil_stitch::lang::typescript::TypeScript::new()),
        Box::new(sigil_stitch::lang::zsh::Zsh::new()),
    ]
}

#[test]
fn direct_and_pretty_adapters_preserve_all_language_indentation() {
    let direct = CodeBlock::of("call(alpha, beta)\n%>body%<", ()).unwrap();
    let pretty = CodeBlock::of("call(%>alpha,%Wbeta%<)\n%>body%<", ()).unwrap();

    for lang in languages() {
        let wide_direct = direct.render_standalone(lang.as_ref(), 120).unwrap();
        let wide_pretty = pretty.render_standalone(lang.as_ref(), 120).unwrap();
        assert_eq!(
            wide_pretty,
            wide_direct,
            "wide adapter parity failed for .{}",
            lang.file_extension()
        );

        let narrow = pretty.render_standalone(lang.as_ref(), 8).unwrap();
        let indent = lang.block_syntax().indent_unit;
        assert!(
            narrow.contains(&format!("\n{indent}beta)")),
            "wrapped argument lost indentation for .{}:\n{narrow}",
            lang.file_extension()
        );
        assert!(
            narrow.contains(&format!("\n{indent}body")),
            "body lost indentation for .{}:\n{narrow}",
            lang.file_extension()
        );
    }
}

#[test]
fn function_parameter_builder_indents_wrapped_continuations() {
    let function = FunSpec::builder("processRequest")
        .add_param(ParameterSpec::of(
            "configuration",
            TypeName::primitive("Configuration"),
        ))
        .add_param(ParameterSpec::of(
            "requestContext",
            TypeName::primitive("RequestContext"),
        ))
        .body(CodeBlock::of("return", ()).unwrap())
        .build()
        .unwrap();
    let output = FileSpec::builder_with(
        "function.ts",
        sigil_stitch::lang::typescript::TypeScript::new(),
    )
    .add_function(function)
    .build()
    .unwrap()
    .render(32)
    .unwrap();

    assert!(output.contains("\n  requestContext: RequestContext"));
    assert!(!output.contains("\nrequestContext"));
}

#[test]
fn primary_constructor_builder_indents_wrapped_continuations() {
    let type_spec = TypeSpec::builder("RequestConfiguration", TypeKind::Class)
        .add_primary_constructor_param(ParameterSpec::of(
            "val endpointAddress",
            TypeName::primitive("String"),
        ))
        .add_primary_constructor_param(ParameterSpec::of(
            "val requestTimeout",
            TypeName::primitive("Duration"),
        ))
        .build()
        .unwrap();
    let output = FileSpec::builder_with(
        "RequestConfiguration.kt",
        sigil_stitch::lang::kotlin::Kotlin::new(),
    )
    .add_type(type_spec)
    .build()
    .unwrap()
    .render(36)
    .unwrap();

    assert!(
        output.contains("\n    val requestTimeout: Duration"),
        "{output}"
    );
    assert!(!output.contains("\nval requestTimeout"));
}

#[test]
fn language_rewrites_run_before_the_pretty_adapter() {
    let mut cpp = CodeBlock::builder();
    cpp.begin_control_flow("auto fn = [&](int x)", ());
    cpp.add_statement("return x * 2", ());
    cpp.end_control_flow();
    cpp.add("%Wnext", ());
    let cpp = cpp.build().unwrap();
    let cpp_output = cpp
        .render_standalone(&sigil_stitch::lang::cpp::Cpp::new(), 120)
        .unwrap();
    assert!(cpp_output.contains("};"), "{cpp_output}");
    assert!(cpp_output.ends_with(" next"), "{cpp_output}");

    let go = CodeBlock::of("value := <- channel%Wnext", ()).unwrap();
    let go_output = go
        .render_standalone(&sigil_stitch::lang::go::Go::new(), 120)
        .unwrap();
    assert!(go_output.contains("value := <-channel next"), "{go_output}");

    let haskell = CodeBlock::of("apply $value%Wnext", ()).unwrap();
    let haskell_output = haskell
        .render_standalone(&sigil_stitch::lang::haskell::Haskell::new(), 120)
        .unwrap();
    assert!(
        haskell_output.contains("apply $ value next"),
        "{haskell_output}"
    );

    let lua = CodeBlock::of("object: method()%Wnext", ()).unwrap();
    let lua_output = lua
        .render_standalone(&sigil_stitch::lang::lua::Lua::new(), 120)
        .unwrap();
    assert!(lua_output.contains("object:method() next"), "{lua_output}");
}

fn if_intent_block(soft_break: bool) -> CodeBlock {
    let mut block = CodeBlock::builder();
    block.begin_control_flow_with_intent(BlockIntent::If, "if (x > 0)", ());
    if soft_break {
        block.add_statement("call(alpha,%Wbeta)", ());
    } else {
        block.add_statement("call(alpha, beta)", ());
    }
    block.end_control_flow();
    block.build().unwrap()
}

#[test]
fn intent_blocks_match_across_direct_and_pretty_adapters() {
    let direct = if_intent_block(false);
    let pretty = if_intent_block(true);

    for lang in languages() {
        let wide_direct = direct.render_standalone(lang.as_ref(), 240).unwrap();
        let wide_pretty = pretty.render_standalone(lang.as_ref(), 240).unwrap();
        assert_eq!(
            wide_pretty,
            wide_direct,
            "wide adapter parity failed for .{}",
            lang.file_extension()
        );

        let narrow = pretty.render_standalone(lang.as_ref(), 8).unwrap();
        let indent = lang.block_syntax().indent_unit;
        assert!(
            narrow.contains(&format!("\n{indent}beta)")),
            "wrapped intent block body lost indentation for .{}:\n{narrow}",
            lang.file_extension()
        );
    }
}

#[test]
fn block_intent_negative_near_matches_pass_on_the_pretty_adapter() {
    let mut cpp = CodeBlock::builder();
    cpp.begin_control_flow("if (matrix[0] > 0)", ());
    cpp.add_statement("return matrix[0]", ());
    cpp.end_control_flow();
    cpp.add("%Wnext", ());
    let cpp = cpp.build().unwrap();
    let cpp_output = cpp
        .render_standalone(&sigil_stitch::lang::cpp::Cpp::new(), 120)
        .unwrap();
    assert!(!cpp_output.contains("};"), "{cpp_output}");
    assert!(cpp_output.ends_with(" next"), "{cpp_output}");

    let mut go = CodeBlock::builder();
    go.begin_control_flow("if fn.String() != \"func\"", ());
    go.end_control_flow();
    go.add_statement("(\"ordinary\")", ());
    go.add("%Wnext", ());
    let go = go.build().unwrap();
    let go_output = go
        .render_standalone(&sigil_stitch::lang::go::Go::new(), 120)
        .unwrap();
    assert!(go_output.contains("}\n(\"ordinary\")"), "{go_output}");
    assert!(!go_output.contains("}(\"ordinary\")"), "{go_output}");
}
