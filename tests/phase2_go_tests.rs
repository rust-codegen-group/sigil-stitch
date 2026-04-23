use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go_lang::GoLang;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

mod golden;

// === Struct with fields and struct tags ===

#[test]
fn test_go_struct_with_tags() {
    let mut tb = TypeSpec::<GoLang>::builder("Config", TypeKind::Struct);
    tb.doc("Config holds application configuration.");

    let mut f1 = FieldSpec::builder("Name", TypeName::primitive("string"));
    f1.tag("json:\"name\"");
    tb.add_field(f1.build().unwrap());

    let mut f2 = FieldSpec::builder("Port", TypeName::primitive("int"));
    f2.tag("json:\"port\" yaml:\"port\"");
    tb.add_field(f2.build().unwrap());

    let mut f3 = FieldSpec::builder("Debug", TypeName::primitive("bool"));
    f3.tag("json:\"debug,omitempty\"");
    tb.add_field(f3.build().unwrap());

    let mut fb = FileSpec::builder_with("config.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package config", ()).unwrap());
    fb.add_type(tb.build().unwrap());
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/struct_with_tags.go", &output);
}

// === Struct + separate receiver methods ===

#[test]
fn test_go_struct_with_methods() {
    let json_marshal = TypeName::<GoLang>::importable("encoding/json", "Marshal");

    // Struct definition.
    let mut tb = TypeSpec::<GoLang>::builder("Server", TypeKind::Struct);
    tb.doc("Server is an HTTP server.");
    tb.add_field(
        FieldSpec::builder("host", TypeName::primitive("string"))
            .build()
            .unwrap(),
    );
    tb.add_field(
        FieldSpec::builder("port", TypeName::primitive("int"))
            .build()
            .unwrap(),
    );

    // Method 1: Start.
    let mut m1 = FunSpec::<GoLang>::builder("Start");
    m1.receiver(ParameterSpec::new("s", TypeName::pointer(TypeName::primitive("Server"))).unwrap());
    m1.returns(TypeName::primitive("error"));
    m1.body(CodeBlock::<GoLang>::of("return nil", ()).unwrap());

    // Method 2: ToJSON.
    let mut m2 = FunSpec::<GoLang>::builder("ToJSON");
    m2.receiver(ParameterSpec::new("s", TypeName::pointer(TypeName::primitive("Server"))).unwrap());
    m2.returns(TypeName::raw("([]byte, error)"));
    m2.body(CodeBlock::<GoLang>::of("return %T(s)", (json_marshal,)).unwrap());

    let mut fb = FileSpec::builder_with("server.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package server", ()).unwrap());
    fb.add_type(tb.build().unwrap());
    fb.add_function(m1.build().unwrap());
    fb.add_function(m2.build().unwrap());
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/struct_with_methods.go", &output);
}

// === Interface ===

#[test]
fn test_go_interface() {
    let mut tb = TypeSpec::<GoLang>::builder("Repository", TypeKind::Interface);
    tb.doc("Repository defines data access methods.");

    let mut find = FunSpec::<GoLang>::builder("FindByID");
    find.add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap());
    find.returns(TypeName::raw("(Entity, error)"));
    tb.add_method(find.build().unwrap());

    let mut save = FunSpec::<GoLang>::builder("Save");
    save.add_param(ParameterSpec::new("entity", TypeName::primitive("Entity")).unwrap());
    save.returns(TypeName::primitive("error"));
    tb.add_method(save.build().unwrap());

    let mut fb = FileSpec::builder_with("repo.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package repo", ()).unwrap());
    fb.add_type(tb.build().unwrap());
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/interface.go", &output);
}

// === Generic function ===

#[test]
fn test_go_generic_function() {
    let tp = TypeParamSpec::<GoLang>::new("T").with_bound(TypeName::primitive("comparable"));
    let mut fb = FunSpec::<GoLang>::builder("Max");
    fb.add_type_param(tp);
    fb.add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap());
    fb.add_param(ParameterSpec::new("b", TypeName::primitive("T")).unwrap());
    fb.returns(TypeName::primitive("T"));
    let mut body_b = CodeBlock::<GoLang>::builder();
    body_b.begin_control_flow("if a > b", ());
    body_b.add_statement("return a", ());
    body_b.end_control_flow();
    body_b.add_statement("return b", ());
    let body = body_b.build().unwrap();
    fb.body(body);

    let mut file_b = FileSpec::builder_with("max.go", GoLang::new());
    file_b.header(CodeBlock::<GoLang>::of("package main", ()).unwrap());
    file_b.add_function(fb.build().unwrap());
    let file = file_b.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/generic_function.go", &output);
}

// === Top-level function with imports ===

#[test]
fn test_go_top_level_function() {
    let fmt_sprintf = TypeName::<GoLang>::importable("fmt", "Sprintf");

    let mut fb = FunSpec::<GoLang>::builder("Greet");
    fb.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
    fb.returns(TypeName::primitive("string"));
    fb.body(CodeBlock::<GoLang>::of("return %T(\"Hello, %%s!\", name)", (fmt_sprintf,)).unwrap());

    let mut file_b = FileSpec::builder_with("greet.go", GoLang::new());
    file_b.header(CodeBlock::<GoLang>::of("package greet", ()).unwrap());
    file_b.add_function(fb.build().unwrap());
    let file = file_b.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/top_level_function.go", &output);
}

// === Enum (Go uses const + iota pattern) ===

#[test]
fn test_go_enum() {
    // Go has no native enum syntax. The idiomatic pattern is:
    //   type Direction int
    //   const ( North Direction = iota; East; ... )
    //
    // This doesn't fit TypeSpec, so we build it as a raw CodeBlock.
    use sigil_stitch::lang::CodeLang;
    let go = GoLang::new();

    let mut cb = CodeBlock::<GoLang>::builder();
    let doc = go.render_doc_comment(&["Direction represents a cardinal direction."]);
    cb.add("%L", doc);
    cb.add_line();
    cb.add("type Direction int", ());
    cb.add_line();
    cb.add_line();
    cb.add("const (", ());
    cb.add_line();
    cb.add("%>", ());
    cb.add("North Direction = iota", ());
    cb.add_line();
    cb.add("East", ());
    cb.add_line();
    cb.add("South", ());
    cb.add_line();
    cb.add("West", ());
    cb.add_line();
    cb.add("%<", ());
    cb.add(")", ());
    cb.add_line();
    let block = cb.build().unwrap();

    let mut fb = FileSpec::builder_with("direction.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package direction", ()).unwrap());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/enum.go", &output);
}
