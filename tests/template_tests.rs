use sigil_stitch::code_block::{CodeBlock, CodeFragment, NameArg};
use sigil_stitch::code_template::CodeTemplate;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

// ── CodeTemplate integration ────────────────────────────

#[test]
fn test_template_full_render_ts() {
    let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = #{init:L}").unwrap();
    let user_type = TypeName::importable_type("./models", "User");

    let block = tmpl
        .apply()
        .set("var", NameArg("currentUser".into()))
        .set("type", user_type)
        .set("init", "getUser()")
        .build()
        .unwrap();

    let output = FileSpec::builder("app.ts")
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import type { User } from './models'"));
    assert!(output.contains("const currentUser: User = getUser()"));
}

#[test]
fn test_template_reuse_with_dedup() {
    let tmpl = CodeTemplate::new("const #{var:N}: #{type:T} = new #{type:T}()").unwrap();
    let user = TypeName::importable_type("./models", "User");
    let config = TypeName::importable("./config", "Config");

    let block1 = tmpl
        .apply()
        .set("var", NameArg("user".into()))
        .set("type", user)
        .build()
        .unwrap();

    let block2 = tmpl
        .apply()
        .set("var", NameArg("config".into()))
        .set("type", config)
        .build()
        .unwrap();

    let output = FileSpec::builder("service.ts")
        .add_code(block1)
        .add_code(block2)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

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

    let inner = inner_tmpl.apply().set("val", "42").build().unwrap();

    let outer = outer_tmpl
        .apply()
        .set("name", NameArg("getAnswer".into()))
        .set("body", inner)
        .build()
        .unwrap();

    let output = FileSpec::builder("answer.ts")
        .add_code(outer)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("function getAnswer()"));
    assert!(output.contains("return 42"));
}

#[test]
fn test_template_composition_via_code_fragment() {
    let body = CodeFragment::of("%>return 42;%<", ()).unwrap();
    let tmpl = CodeTemplate::new("function #{name:N}() {\n#{body:L}\n}").unwrap();

    let block = tmpl
        .apply()
        .set("name", NameArg("getAnswer".into()))
        .set("body", body)
        .build()
        .unwrap();

    let output = FileSpec::builder("answer.ts")
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("function getAnswer() {\n  return 42;\n}"));
    assert!(!output.contains("%>"));
    assert!(!output.contains("%<"));
}

// ── RawContentWithImports integration ───────────────────

#[test]
fn test_raw_with_imports_triggers_import() {
    let user_type = TypeName::importable_type("./models", "User");

    let output = FileSpec::builder("handler.ts")
        .add_raw_with_imports("const u: User = fetchUser();\n", vec![user_type])
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    // Import should be generated.
    assert!(output.contains("import type { User } from './models'"));
    // Raw content should appear verbatim.
    assert!(output.contains("const u: User = fetchUser();"));
}

#[test]
fn test_raw_with_imports_no_substitution() {
    let ty = TypeName::importable("./lib", "Helper");

    let output = FileSpec::builder("test.ts")
        .add_raw_with_imports("// Helper is used here\n", vec![ty])
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    // Raw content unchanged.
    assert!(output.contains("// Helper is used here"));
    // Import header present.
    assert!(output.contains("import { Helper } from './lib'"));
}

#[test]
fn test_raw_with_imports_mixed_with_code() {
    let user_type = TypeName::importable_type("./models", "User");
    let same_user = TypeName::importable_type("./models", "User");

    let fb = FileSpec::builder("mixed.ts");

    // Raw content referencing User.
    // CodeBlock also referencing User.
    let mut cb = CodeBlock::builder();
    cb.add_statement("const u: %T = getUser()", (same_user,));
    let fb = fb
        .add_raw_with_imports("// User type is used below\n", vec![user_type])
        .add_code(cb.build().unwrap());

    let output = fb.build().unwrap().render(80).unwrap();

    // Import appears only once (dedup).
    let import_count = output.matches("import type { User }").count();
    assert_eq!(import_count, 1);
}

// ── Rust template ───────────────────────────────────────

#[test]
fn test_template_rust_render() {
    let tmpl = CodeTemplate::new("let #{var:N}: #{type:T} = #{init:L}").unwrap();
    let config_type = TypeName::importable("crate::config", "Config");

    let block = tmpl
        .apply()
        .set("var", NameArg("cfg".into()))
        .set("type", config_type)
        .set("init", "Config::default()")
        .build()
        .unwrap();

    let output = FileSpec::builder("main.rs")
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("use crate::config::Config;"));
    assert!(output.contains("let cfg: Config = Config::default()"));
}
