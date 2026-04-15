use sigil_stitch::code_block::{CodeBlock, NameArg};
use sigil_stitch::code_template::CodeTemplate;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

// ── CodeTemplate integration ────────────────────────────

#[test]
fn test_template_full_render_ts() {
    let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = #{init:L}").unwrap();
    let user_type = TypeName::<TypeScript>::importable_type("./models", "User");

    let block = tmpl
        .apply::<TypeScript>()
        .set("var", NameArg("currentUser".into()))
        .set("type", user_type)
        .set("init", "getUser()")
        .build()
        .unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    fb.add_code(block);
    let output = fb.build().render(80).unwrap();

    assert!(output.contains("import type { User } from './models'"));
    assert!(output.contains("const currentUser: User = getUser()"));
}

#[test]
fn test_template_reuse_with_dedup() {
    let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = new #{type:T}()").unwrap();
    let user = TypeName::<TypeScript>::importable_type("./models", "User");
    let config = TypeName::<TypeScript>::importable("./config", "Config");

    let block1 = tmpl
        .apply::<TypeScript>()
        .set("var", NameArg("user".into()))
        .set("type", user)
        .build()
        .unwrap();

    let block2 = tmpl
        .apply::<TypeScript>()
        .set("var", NameArg("config".into()))
        .set("type", config)
        .build()
        .unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("service.ts");
    fb.add_code(block1);
    fb.add_code(block2);
    let output = fb.build().render(80).unwrap();

    assert!(output.contains("import type { User } from './models'"));
    assert!(output.contains("import { Config } from './config'"));
    // Each import appears once.
    assert_eq!(output.matches("User").count(), 3); // import + 2 in code
    assert_eq!(output.matches("Config").count(), 3); // import + 2 in code
}

#[test]
fn test_template_composition_via_literal() {
    let inner_tmpl = CodeTemplate::new("return #{val:L}").unwrap();
    let outer_tmpl = CodeTemplate::new("function #{name:N}() { #{body:L} }").unwrap();

    let inner = inner_tmpl
        .apply::<TypeScript>()
        .set("val", "42")
        .build()
        .unwrap();

    let outer = outer_tmpl
        .apply::<TypeScript>()
        .set("name", NameArg("getAnswer".into()))
        .set("body", inner)
        .build()
        .unwrap();

    let mut fb = FileSpec::<TypeScript>::builder("answer.ts");
    fb.add_code(outer);
    let output = fb.build().render(80).unwrap();

    assert!(output.contains("function getAnswer()"));
    assert!(output.contains("return 42"));
}

// ── RawContentWithImports integration ───────────────────

#[test]
fn test_raw_with_imports_triggers_import() {
    let user_type = TypeName::<TypeScript>::importable_type("./models", "User");

    let mut fb = FileSpec::<TypeScript>::builder("handler.ts");
    fb.add_raw_with_imports("const u: User = fetchUser();\n", vec![user_type]);
    let output = fb.build().render(80).unwrap();

    // Import should be generated.
    assert!(output.contains("import type { User } from './models'"));
    // Raw content should appear verbatim.
    assert!(output.contains("const u: User = fetchUser();"));
}

#[test]
fn test_raw_with_imports_no_substitution() {
    let ty = TypeName::<TypeScript>::importable("./lib", "Helper");

    let mut fb = FileSpec::<TypeScript>::builder("test.ts");
    fb.add_raw_with_imports("// Helper is used here\n", vec![ty]);
    let output = fb.build().render(80).unwrap();

    // Raw content unchanged.
    assert!(output.contains("// Helper is used here"));
    // Import header present.
    assert!(output.contains("import { Helper } from './lib'"));
}

#[test]
fn test_raw_with_imports_mixed_with_code() {
    let user_type = TypeName::<TypeScript>::importable_type("./models", "User");
    let same_user = TypeName::<TypeScript>::importable_type("./models", "User");

    let mut fb = FileSpec::<TypeScript>::builder("mixed.ts");

    // Raw content referencing User.
    fb.add_raw_with_imports("// User type is used below\n", vec![user_type]);

    // CodeBlock also referencing User.
    let mut cb = CodeBlock::<TypeScript>::builder();
    cb.add_statement("const u: %T = getUser()", (same_user,));
    fb.add_code(cb.build().unwrap());

    let output = fb.build().render(80).unwrap();

    // Import appears only once (dedup).
    let import_count = output.matches("import type { User }").count();
    assert_eq!(import_count, 1);
}

// ── Rust template ───────────────────────────────────────

#[test]
fn test_template_rust_render() {
    let tmpl = CodeTemplate::new("let #{var:N}: #{type:T} = #{init:L}").unwrap();
    let config_type = TypeName::<RustLang>::importable("crate::config", "Config");

    let block = tmpl
        .apply::<RustLang>()
        .set("var", NameArg("cfg".into()))
        .set("type", config_type)
        .set("init", "Config::default()")
        .build()
        .unwrap();

    let mut fb = FileSpec::<RustLang>::builder("main.rs");
    fb.add_code(block);
    let output = fb.build().render(80).unwrap();

    assert!(output.contains("use crate::config::Config;"));
    assert!(output.contains("let cfg: Config = Config::default()"));
}
