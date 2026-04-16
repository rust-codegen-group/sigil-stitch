use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::dart::DartLang;
use sigil_stitch::lang::go_lang::GoLang;
use sigil_stitch::lang::java_lang::JavaLang;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::python::Python;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::import_spec::ImportSpec;
use sigil_stitch::type_name::TypeName;

// ── TypeScript ────────────────────────────────────────────

#[test]
fn test_ts_forced_named_import() {
    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    fb.add_import(ImportSpec::named("./models", "User"));
    fb.add_code(CodeBlock::<TypeScript>::of("console.log('hello')", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import { User } from './models';"));
    assert!(output.contains("console.log('hello')"));
}

#[test]
fn test_ts_aliased_import() {
    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    fb.add_import(ImportSpec::named_as("./models", "User", "AppUser"));
    fb.add_code(CodeBlock::<TypeScript>::of("const u: AppUser = get()", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("User as AppUser"));
}

#[test]
fn test_ts_type_only_import() {
    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    fb.add_import(ImportSpec::named_type("./models", "User"));
    fb.add_code(CodeBlock::<TypeScript>::of("// no usage", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import type { User } from './models';"));
}

#[test]
fn test_ts_side_effect_import() {
    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    fb.add_import(ImportSpec::side_effect("./polyfill"));
    fb.add_code(CodeBlock::<TypeScript>::of("// app code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import './polyfill';"));
}

#[test]
fn test_ts_wildcard_import() {
    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    fb.add_import(ImportSpec::wildcard("./utils"));
    fb.add_code(CodeBlock::<TypeScript>::of("// app code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import * as Utils from './utils';"));
}

#[test]
fn test_ts_mixed_explicit_and_auto() {
    let user = TypeName::<TypeScript>::importable_type("./models", "User");

    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    fb.add_import(ImportSpec::side_effect("./polyfill"));
    fb.add_import(ImportSpec::named("./helpers", "format"));
    fb.add_code(CodeBlock::<TypeScript>::of("const u: %T = get()", (user,)).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import './polyfill';"));
    assert!(output.contains("import type { User } from './models';"));
    assert!(output.contains("import { format } from './helpers';"));
}

#[test]
fn test_ts_explicit_alias_takes_precedence() {
    // Explicit alias should override auto-resolution.
    let user1 = TypeName::<TypeScript>::importable("./models", "User");
    let user2 = TypeName::<TypeScript>::importable("./other", "User");

    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    // Pre-claim ./other User as "OtherUser" explicitly.
    fb.add_import(ImportSpec::named_as("./other", "User", "OtherUser"));
    let mut code = CodeBlock::<TypeScript>::builder();
    code.add_statement("const u1: %T = get1()", (user1,));
    code.add_statement("const u2: %T = get2()", (user2,));
    fb.add_code(code.build().unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("User as OtherUser"));
    assert!(output.contains("const u2: OtherUser = get2();"));
}

// ── JavaScript ────────────────────────────────────────────

#[test]
fn test_js_side_effect_import() {
    let mut fb = FileSpec::builder_with("app.js", JavaScript::new());
    fb.add_import(ImportSpec::side_effect("./polyfill"));
    fb.add_code(CodeBlock::<JavaScript>::of("// app code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import './polyfill';"));
}

#[test]
fn test_js_wildcard_import() {
    let mut fb = FileSpec::builder_with("app.js", JavaScript::new());
    fb.add_import(ImportSpec::wildcard("./utils"));
    fb.add_code(CodeBlock::<JavaScript>::of("// app code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import * as Utils from './utils';"));
}

// ── Rust ──────────────────────────────────────────────────

#[test]
fn test_rust_forced_named_import() {
    let mut fb = FileSpec::builder_with("main.rs", RustLang::new());
    fb.add_import(ImportSpec::named("std::collections", "HashMap"));
    fb.add_code(CodeBlock::<RustLang>::of("// code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("use std::collections::HashMap;"));
}

#[test]
fn test_rust_wildcard_import() {
    let mut fb = FileSpec::builder_with("main.rs", RustLang::new());
    fb.add_import(ImportSpec::wildcard("std::collections"));
    fb.add_code(CodeBlock::<RustLang>::of("// code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("use std::collections::*;"));
}

// ── Go ────────────────────────────────────────────────────

#[test]
fn test_go_side_effect_import() {
    let mut fb = FileSpec::builder_with("main.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package main", ()).unwrap());
    fb.add_import(ImportSpec::side_effect("database/sql"));
    fb.add_code(CodeBlock::<GoLang>::of("// init", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import _ \"database/sql\""));
}

#[test]
fn test_go_wildcard_import() {
    let mut fb = FileSpec::builder_with("main.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package main", ()).unwrap());
    fb.add_import(ImportSpec::wildcard("math"));
    fb.add_code(CodeBlock::<GoLang>::of("// use math", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import . \"math\""));
}

#[test]
fn test_go_mixed_side_effect_and_named() {
    let fmt = TypeName::<GoLang>::importable("fmt", "Println");

    let mut fb = FileSpec::builder_with("main.go", GoLang::new());
    fb.header(CodeBlock::<GoLang>::of("package main", ()).unwrap());
    fb.add_import(ImportSpec::side_effect("github.com/lib/pq"));
    fb.add_code(CodeBlock::<GoLang>::of("%T(\"hello\")", (fmt,)).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import ("));
    assert!(output.contains("\"fmt\""));
    assert!(output.contains("_ \"github.com/lib/pq\""));
}

// ── Python ────────────────────────────────────────────────

#[test]
fn test_python_side_effect_import() {
    let mut fb = FileSpec::builder_with("app.py", Python::new());
    fb.add_import(ImportSpec::side_effect("logging"));
    fb.add_code(CodeBlock::<Python>::of("# code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import logging"));
}

#[test]
fn test_python_wildcard_import() {
    let mut fb = FileSpec::builder_with("app.py", Python::new());
    fb.add_import(ImportSpec::wildcard("os.path"));
    fb.add_code(CodeBlock::<Python>::of("# code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("from os.path import *"));
}

// ── Java ──────────────────────────────────────────────────

#[test]
fn test_java_wildcard_import() {
    let mut fb = FileSpec::builder_with("App.java", JavaLang::new());
    fb.add_import(ImportSpec::wildcard("java.util"));
    fb.add_code(CodeBlock::<JavaLang>::of("// code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import java.util.*;"));
}

#[test]
fn test_java_forced_named_import() {
    let mut fb = FileSpec::builder_with("App.java", JavaLang::new());
    fb.add_import(ImportSpec::named("java.util", "List"));
    fb.add_code(CodeBlock::<JavaLang>::of("// code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import java.util.List;"));
}

// ── Kotlin ────────────────────────────────────────────────

#[test]
fn test_kotlin_wildcard_import() {
    let mut fb = FileSpec::builder_with("App.kt", Kotlin::new());
    fb.add_import(ImportSpec::wildcard("kotlin.collections"));
    fb.add_code(CodeBlock::<Kotlin>::of("// code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import kotlin.collections.*"));
}

// ── Swift ─────────────────────────────────────────────────

#[test]
fn test_swift_forced_module_import() {
    let mut fb = FileSpec::builder_with("App.swift", Swift::new());
    fb.add_import(ImportSpec::side_effect("UIKit"));
    fb.add_code(CodeBlock::<Swift>::of("// code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import UIKit"));
}

// ── Dart ──────────────────────────────────────────────────

#[test]
fn test_dart_side_effect_import() {
    let mut fb = FileSpec::builder_with("app.dart", DartLang::new());
    fb.add_import(ImportSpec::side_effect("package:flutter/material.dart"));
    fb.add_code(CodeBlock::<DartLang>::of("// code", ()).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    assert!(output.contains("import 'package:flutter/material.dart';"));
}

// ── No duplicate when explicit matches auto ───────────────

#[test]
fn test_no_duplicate_when_explicit_matches_auto() {
    let user = TypeName::<TypeScript>::importable("./models", "User");

    let mut fb = FileSpec::<TypeScript>::builder("app.ts");
    // Explicit import of the same thing that %T will auto-collect.
    fb.add_import(ImportSpec::named("./models", "User"));
    fb.add_code(CodeBlock::<TypeScript>::of("const u: %T = get()", (user,)).unwrap());
    let output = fb.build().unwrap().render(80).unwrap();

    // Should appear only once.
    let count = output.matches("User").count();
    // "User" appears in the import line and in the code line, but the import line itself
    // should only appear once.
    let import_count = output.matches("import { User } from './models';").count();
    assert_eq!(
        import_count, 1,
        "Import should appear exactly once: {output}"
    );
    assert!(
        count >= 2,
        "User should appear in import and code: {output}"
    );
}
