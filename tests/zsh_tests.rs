use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::zsh::Zsh;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::type_name::TypeName;

mod golden;

// === Basic variable assignment ===

#[test]
fn test_zsh_variable_assignment() {
    let mut b = CodeBlock::<Zsh>::builder();
    b.add("NAME=%S\n", (StringLitArg("world".into()),));
    b.add("COUNT=42\n", ());
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("vars.zsh", Zsh::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("zsh/variable_assignment.zsh", &output);
}

// === Zsh-specific percent escaping ===

#[test]
fn test_zsh_percent_escaping() {
    let mut b = CodeBlock::<Zsh>::builder();
    b.add("MSG=%S\n", (StringLitArg("100% done".into()),));
    b.add("PROMPT=%S\n", (StringLitArg("%F{red}error%f".into()),));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("prompt.zsh", Zsh::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("zsh/percent_escaping.zsh", &output);
}

// === Function declaration via FunSpec ===

#[test]
fn test_zsh_function_spec() {
    let mut body_b = CodeBlock::<Zsh>::builder();
    body_b.add("local name=$1\n", ());
    body_b.add_statement("echo \"Hello, $name\"", ());
    let body = body_b.build().unwrap();

    let mut fb = FunSpec::<Zsh>::builder("greet");
    fb.body(body);
    let fun = fb.build().unwrap();

    let mut file_b = FileSpec::builder_with("funcs.zsh", Zsh::new());
    file_b.add_function(fun);
    let file = file_b.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("zsh/function_spec.zsh", &output);
}

// === If/then/fi control flow ===

#[test]
fn test_zsh_if_then_fi() {
    let mut b = CodeBlock::<Zsh>::builder();
    b.add("if [[ -f \"$1\" ]]; then\n", ());
    b.add("%>", ());
    b.add_statement("echo \"file exists\"", ());
    b.add("%<", ());
    b.add("else\n", ());
    b.add("%>", ());
    b.add_statement("echo \"file not found\"", ());
    b.add("%<", ());
    b.add("fi\n", ());
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("check.zsh", Zsh::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("zsh/if_then_fi.zsh", &output);
}

// === Source imports ===

#[test]
fn test_zsh_source_imports() {
    let utils_fn = TypeName::<Zsh>::importable("./lib/utils.zsh", "log_info");
    let config_fn = TypeName::<Zsh>::importable("./lib/config.zsh", "load_config");

    let mut b = CodeBlock::<Zsh>::builder();
    b.add_statement("%T", (config_fn,));
    b.add_statement("%T \"starting up\"", (utils_fn,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("main.zsh", Zsh::new());
    fb.add_code(block);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("zsh/source_imports.zsh", &output);
}

// === Complete script ===

#[test]
fn test_zsh_complete_script() {
    let utils_fn = TypeName::<Zsh>::importable("./lib/utils.zsh", "log_info");

    let mut header_b = CodeBlock::<Zsh>::builder();
    header_b.add("#!/usr/bin/env zsh\n", ());
    header_b.add("setopt ERR_EXIT PIPE_FAIL", ());
    let header = header_b.build().unwrap();

    let mut fun_body = CodeBlock::<Zsh>::builder();
    fun_body.add("local target=$1\n", ());
    fun_body.add("if [[ -z \"$target\" ]]; then\n", ());
    fun_body.add("%>", ());
    fun_body.add_statement("echo \"error: no target\"", ());
    fun_body.add_statement("return 1", ());
    fun_body.add("%<", ());
    fun_body.add("fi\n", ());
    fun_body.add_statement("%T \"deploying to $target\"", (utils_fn,));
    let fun_body = fun_body.build().unwrap();

    let mut fun = FunSpec::<Zsh>::builder("deploy");
    fun.body(fun_body);
    let fun = fun.build().unwrap();

    let mut main = CodeBlock::<Zsh>::builder();
    main.add_statement("deploy \"$@\"", ());
    let main = main.build().unwrap();

    let mut fb = FileSpec::builder_with("deploy.zsh", Zsh::new());
    fb.header(header);
    fb.add_function(fun);
    fb.add_code(main);
    let file = fb.build().unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("zsh/complete_script.zsh", &output);
}
