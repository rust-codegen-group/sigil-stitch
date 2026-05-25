use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::dart::Dart;
use sigil_stitch::lang::go::Go;
use sigil_stitch::lang::java::Java;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::python::Python;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::import_spec::ImportSpec;
use sigil_stitch::type_name::TypeName;

// ── TypeScript ────────────────────────────────────────────

#[test]
fn test_ts_forced_named_import() {
    let output = FileSpec::builder("app.ts")
        .add_import(ImportSpec::named("./models", "User"))
        .add_code(CodeBlock::of("console.log('hello')", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import { User } from './models';"));
    assert!(output.contains("console.log('hello')"));
}

#[test]
fn test_ts_aliased_import() {
    let output = FileSpec::builder("app.ts")
        .add_import(ImportSpec::named_as("./models", "User", "AppUser"))
        .add_code(CodeBlock::of("const u: AppUser = get()", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("User as AppUser"));
}

#[test]
fn test_ts_type_only_import() {
    let output = FileSpec::builder("app.ts")
        .add_import(ImportSpec::named_type("./models", "User"))
        .add_code(CodeBlock::of("// no usage", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import type { User } from './models';"));
}

#[test]
fn test_ts_side_effect_import() {
    let output = FileSpec::builder("app.ts")
        .add_import(ImportSpec::side_effect("./polyfill"))
        .add_code(CodeBlock::of("// app code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import './polyfill';"));
}

#[test]
fn test_ts_wildcard_import() {
    let output = FileSpec::builder("app.ts")
        .add_import(ImportSpec::wildcard("./utils"))
        .add_code(CodeBlock::of("// app code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import * as Utils from './utils';"));
}

#[test]
fn test_ts_mixed_explicit_and_auto() {
    let user = TypeName::importable_type("./models", "User");

    let output = FileSpec::builder("app.ts")
        .add_import(ImportSpec::side_effect("./polyfill"))
        .add_import(ImportSpec::named("./helpers", "format"))
        .add_code(CodeBlock::of("const u: %T = get()", (user,)).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import './polyfill';"));
    assert!(output.contains("import type { User } from './models';"));
    assert!(output.contains("import { format } from './helpers';"));
}

#[test]
fn test_ts_explicit_alias_takes_precedence() {
    // Explicit alias should override auto-resolution.
    let user1 = TypeName::importable("./models", "User");
    let user2 = TypeName::importable("./other", "User");

    let mut code = CodeBlock::builder();
    code.add_statement("const u1: %T = get1()", (user1,));
    code.add_statement("const u2: %T = get2()", (user2,));
    // Pre-claim ./other User as "OtherUser" explicitly.
    let output = FileSpec::builder("app.ts")
        .add_import(ImportSpec::named_as("./other", "User", "OtherUser"))
        .add_code(code.build().unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("User as OtherUser"));
    assert!(output.contains("const u2: OtherUser = get2();"));
}

// ── JavaScript ────────────────────────────────────────────

#[test]
fn test_js_side_effect_import() {
    let output = FileSpec::builder_with("app.js", JavaScript::new())
        .add_import(ImportSpec::side_effect("./polyfill"))
        .add_code(CodeBlock::of("// app code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import './polyfill';"));
}

#[test]
fn test_js_wildcard_import() {
    let output = FileSpec::builder_with("app.js", JavaScript::new())
        .add_import(ImportSpec::wildcard("./utils"))
        .add_code(CodeBlock::of("// app code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import * as Utils from './utils';"));
}

// ── Rust ──────────────────────────────────────────────────

#[test]
fn test_rust_forced_named_import() {
    let output = FileSpec::builder_with("main.rs", Rust::new())
        .add_import(ImportSpec::named("std::collections", "HashMap"))
        .add_code(CodeBlock::of("// code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("use std::collections::HashMap;"));
}

#[test]
fn test_rust_wildcard_import() {
    let output = FileSpec::builder_with("main.rs", Rust::new())
        .add_import(ImportSpec::wildcard("std::collections"))
        .add_code(CodeBlock::of("// code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("use std::collections::*;"));
}

// ── Go ────────────────────────────────────────────────────

#[test]
fn test_go_side_effect_import() {
    let output = FileSpec::builder_with("main.go", Go::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_import(ImportSpec::side_effect("database/sql"))
        .add_code(CodeBlock::of("// init", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import _ \"database/sql\""));
}

#[test]
fn test_go_wildcard_import() {
    let output = FileSpec::builder_with("main.go", Go::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_import(ImportSpec::wildcard("math"))
        .add_code(CodeBlock::of("// use math", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import . \"math\""));
}

#[test]
fn test_go_mixed_side_effect_and_named() {
    let fmt = TypeName::importable("fmt", "Println");

    let output = FileSpec::builder_with("main.go", Go::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_import(ImportSpec::side_effect("github.com/lib/pq"))
        .add_code(CodeBlock::of("%T(\"hello\")", (fmt,)).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import ("));
    assert!(output.contains("\"fmt\""));
    assert!(output.contains("_ \"github.com/lib/pq\""));
}

// ── Python ────────────────────────────────────────────────

#[test]
fn test_python_side_effect_import() {
    let output = FileSpec::builder_with("app.py", Python::new())
        .add_import(ImportSpec::side_effect("logging"))
        .add_code(CodeBlock::of("# code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import logging"));
}

#[test]
fn test_python_wildcard_import() {
    let output = FileSpec::builder_with("app.py", Python::new())
        .add_import(ImportSpec::wildcard("os.path"))
        .add_code(CodeBlock::of("# code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("from os.path import *"));
}

// ── Java ──────────────────────────────────────────────────

#[test]
fn test_java_wildcard_import() {
    let output = FileSpec::builder_with("App.java", Java::new())
        .add_import(ImportSpec::wildcard("java.util"))
        .add_code(CodeBlock::of("// code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import java.util.*;"));
}

#[test]
fn test_java_forced_named_import() {
    let output = FileSpec::builder_with("App.java", Java::new())
        .add_import(ImportSpec::named("java.util", "List"))
        .add_code(CodeBlock::of("// code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import java.util.List;"));
}

// ── Kotlin ────────────────────────────────────────────────

#[test]
fn test_kotlin_wildcard_import() {
    let output = FileSpec::builder_with("App.kt", Kotlin::new())
        .add_import(ImportSpec::wildcard("kotlin.collections"))
        .add_code(CodeBlock::of("// code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import kotlin.collections.*"));
}

// ── Swift ─────────────────────────────────────────────────

#[test]
fn test_swift_forced_module_import() {
    let output = FileSpec::builder_with("App.swift", Swift::new())
        .add_import(ImportSpec::side_effect("UIKit"))
        .add_code(CodeBlock::of("// code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import UIKit"));
}

// ── Dart ──────────────────────────────────────────────────

#[test]
fn test_dart_side_effect_import() {
    let output = FileSpec::builder_with("app.dart", Dart::new())
        .add_import(ImportSpec::side_effect("package:flutter/material.dart"))
        .add_code(CodeBlock::of("// code", ()).unwrap())
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import 'package:flutter/material.dart';"));
}

// ── No duplicate when explicit matches auto ───────────────

#[test]
fn test_no_duplicate_when_explicit_matches_auto() {
    let user = TypeName::importable("./models", "User");

    let fb = FileSpec::builder("app.ts")
        .add_import(ImportSpec::named("./models", "User"))
        .add_code(CodeBlock::of("const u: %T = get()", (user,)).unwrap());
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
