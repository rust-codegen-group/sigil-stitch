mod golden;

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

/// Shorthand for a JS parameter (no type annotation).
fn param(name: &str) -> ParameterSpec<JavaScript> {
    ParameterSpec::new(name, TypeName::primitive(""))
}

/// Shorthand for a JS field (no type annotation).
fn field(name: &str) -> FieldSpec<JavaScript> {
    FieldSpec::builder(name, TypeName::primitive("")).build()
}

#[test]
fn test_js_class_with_methods() {
    let mut tb = TypeSpec::<JavaScript>::builder("Counter", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.doc("A simple counter.");

    tb.add_field(field("count"));

    // Constructor.
    let ctor_body = CodeBlock::<JavaScript>::of("this.count = 0;", ()).unwrap();
    let mut ctor = FunSpec::<JavaScript>::builder("constructor");
    ctor.body(ctor_body);
    tb.add_method(ctor.build());

    // increment method.
    let inc_body = CodeBlock::<JavaScript>::of("this.count++;", ()).unwrap();
    let mut inc = FunSpec::<JavaScript>::builder("increment");
    inc.body(inc_body);
    tb.add_method(inc.build());

    // getCount method.
    let get_body = CodeBlock::<JavaScript>::of("return this.count;", ()).unwrap();
    let mut get = FunSpec::<JavaScript>::builder("getCount");
    get.body(get_body);
    tb.add_method(get.build());

    let ts = tb.build();

    let mut fb = FileSpec::builder_with("counter.js", JavaScript::new());
    fb.add_type(ts);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/class_with_methods.js", &output);
}

#[test]
fn test_js_class_with_constructor() {
    let mut tb = TypeSpec::<JavaScript>::builder("User", TypeKind::Class);
    tb.visibility(Visibility::Public);

    tb.add_field(field("name"));
    tb.add_field(field("email"));

    let ctor_body =
        CodeBlock::<JavaScript>::of("this.name = name;\nthis.email = email;", ()).unwrap();
    let mut ctor = FunSpec::<JavaScript>::builder("constructor");
    ctor.add_param(param("name"));
    ctor.add_param(param("email"));
    ctor.body(ctor_body);
    tb.add_method(ctor.build());

    let ts = tb.build();

    let mut fb = FileSpec::builder_with("user.js", JavaScript::new());
    fb.add_type(ts);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/class_with_constructor.js", &output);
}

#[test]
fn test_js_export_function() {
    let body = CodeBlock::<JavaScript>::of(
        "console.log(%S + name);",
        (StringLitArg("Hello, ".to_string()),),
    )
    .unwrap();
    let mut fb = FunSpec::<JavaScript>::builder("greet");
    fb.visibility(Visibility::Public);
    fb.add_param(param("name"));
    fb.body(body);
    let fun = fb.build();

    let mut file_b = FileSpec::builder_with("greet.js", JavaScript::new());
    file_b.add_function(fun);
    let file = file_b.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/export_function.js", &output);
}

#[test]
fn test_js_class_extends() {
    let animal_import = TypeName::<JavaScript>::importable("./animal", "Animal");

    let mut tb = TypeSpec::<JavaScript>::builder("Dog", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.extends(TypeName::primitive("Animal"));

    let ctor_body = CodeBlock::<JavaScript>::of("super(name);\nthis.breed = breed;", ()).unwrap();
    let mut ctor = FunSpec::<JavaScript>::builder("constructor");
    ctor.add_param(param("name"));
    ctor.add_param(param("breed"));
    ctor.body(ctor_body);
    tb.add_method(ctor.build());

    let speak_body = CodeBlock::<JavaScript>::of("return 'Woof!';", ()).unwrap();
    let mut speak = FunSpec::<JavaScript>::builder("speak");
    speak.body(speak_body);
    tb.add_method(speak.build());

    let ts = tb.build();

    // Trigger import via code block.
    let import_trigger = CodeBlock::<JavaScript>::of("// Uses %T", (animal_import,)).unwrap();

    let mut fb = FileSpec::builder_with("dog.js", JavaScript::new());
    fb.add_code(import_trigger);
    fb.add_type(ts);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/class_extends.js", &output);
}

#[test]
fn test_js_async_function() {
    let fetch_type = TypeName::<JavaScript>::importable("node-fetch", "fetch");

    let body = CodeBlock::<JavaScript>::of(
        "const response = await %T(url);\nreturn response.json();",
        (fetch_type,),
    )
    .unwrap();
    let mut fb = FunSpec::<JavaScript>::builder("fetchData");
    fb.visibility(Visibility::Public);
    fb.is_async();
    fb.add_param(param("url"));
    fb.body(body);
    let fun = fb.build();

    let mut file_b = FileSpec::builder_with("api.js", JavaScript::new());
    file_b.add_function(fun);
    let file = file_b.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/async_function.js", &output);
}

#[test]
fn test_js_static_method() {
    let mut tb = TypeSpec::<JavaScript>::builder("MathUtils", TypeKind::Class);
    tb.visibility(Visibility::Public);

    let body = CodeBlock::<JavaScript>::of("return a + b;", ()).unwrap();
    let mut add = FunSpec::<JavaScript>::builder("add");
    add.is_static();
    add.add_param(param("a"));
    add.add_param(param("b"));
    add.body(body);
    tb.add_method(add.build());

    let ts = tb.build();

    let mut fb = FileSpec::builder_with("math.js", JavaScript::new());
    fb.add_type(ts);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/static_method.js", &output);
}

#[test]
fn test_js_private_field() {
    let mut tb = TypeSpec::<JavaScript>::builder("BankAccount", TypeKind::Class);
    tb.visibility(Visibility::Public);

    // ES2022 private fields use # prefix.
    tb.add_field(field("#balance"));

    let ctor_body = CodeBlock::<JavaScript>::of("this.#balance = initialBalance;", ()).unwrap();
    let mut ctor = FunSpec::<JavaScript>::builder("constructor");
    ctor.add_param(param("initialBalance"));
    ctor.body(ctor_body);
    tb.add_method(ctor.build());

    let get_body = CodeBlock::<JavaScript>::of("return this.#balance;", ()).unwrap();
    let mut get = FunSpec::<JavaScript>::builder("getBalance");
    get.body(get_body);
    tb.add_method(get.build());

    let ts = tb.build();

    let mut fb = FileSpec::builder_with("account.js", JavaScript::new());
    fb.add_type(ts);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/private_field.js", &output);
}

#[test]
fn test_js_full_module() {
    let event_emitter = TypeName::<JavaScript>::importable("events", "EventEmitter");
    let uuid = TypeName::<JavaScript>::importable("uuid", "v4");

    // EventBus class extending EventEmitter.
    let mut tb = TypeSpec::<JavaScript>::builder("EventBus", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.extends(TypeName::primitive("EventEmitter"));
    tb.doc("Application event bus.");

    tb.add_field(field("#handlers"));

    let ctor_body =
        CodeBlock::<JavaScript>::of("super();\nthis.#handlers = new Map();", ()).unwrap();
    let mut ctor = FunSpec::<JavaScript>::builder("constructor");
    ctor.body(ctor_body);
    tb.add_method(ctor.build());

    let pub_body = CodeBlock::<JavaScript>::of(
        "const id = %T();\nthis.emit(event, data);\nreturn id;",
        (uuid,),
    )
    .unwrap();
    let mut publish = FunSpec::<JavaScript>::builder("publish");
    publish.add_param(param("event"));
    publish.add_param(param("data"));
    publish.body(pub_body);
    tb.add_method(publish.build());

    let ts = tb.build();

    // Trigger EventEmitter import.
    let import_trigger = CodeBlock::<JavaScript>::of("// extends %T", (event_emitter,)).unwrap();

    // Standalone exported function.
    let create_body = CodeBlock::<JavaScript>::of("return new EventBus();", ()).unwrap();
    let mut create_fn = FunSpec::<JavaScript>::builder("createEventBus");
    create_fn.visibility(Visibility::Public);
    create_fn.body(create_body);
    let create = create_fn.build();

    let mut fb = FileSpec::builder_with("event-bus.js", JavaScript::new());
    fb.add_code(import_trigger);
    fb.add_type(ts);
    fb.add_function(create);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/full_module.js", &output);
}

// === Enum (JS uses frozen object pattern via TypeKind::Enum → class) ===

#[test]
fn test_js_enum() {
    // JavaScript has no native enums. TypeKind::Enum maps to `class`,
    // producing a class with constant-like variant members.
    let mut tb = TypeSpec::<JavaScript>::builder("Direction", TypeKind::Enum);
    tb.visibility(Visibility::Public);
    tb.doc("Cardinal directions.");

    let mut v_up = EnumVariantSpec::<JavaScript>::builder("Up");
    v_up.value(CodeBlock::<JavaScript>::of("'UP'", ()).unwrap());
    tb.add_variant(v_up.build());

    let mut v_down = EnumVariantSpec::<JavaScript>::builder("Down");
    v_down.value(CodeBlock::<JavaScript>::of("'DOWN'", ()).unwrap());
    tb.add_variant(v_down.build());

    let mut v_left = EnumVariantSpec::<JavaScript>::builder("Left");
    v_left.value(CodeBlock::<JavaScript>::of("'LEFT'", ()).unwrap());
    tb.add_variant(v_left.build());

    let mut v_right = EnumVariantSpec::<JavaScript>::builder("Right");
    v_right.value(CodeBlock::<JavaScript>::of("'RIGHT'", ()).unwrap());
    tb.add_variant(v_right.build());

    let mut fb = FileSpec::builder_with("direction.js", JavaScript::new());
    fb.add_type(tb.build());
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("javascript/enum.js", &output);
}
