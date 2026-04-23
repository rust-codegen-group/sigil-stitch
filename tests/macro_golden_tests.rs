//! Per-language golden tests for the `sigil_quote!` macro.

mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::bash::Bash;
use sigil_stitch::lang::c_lang::CLang;
use sigil_stitch::lang::cpp_lang::CppLang;
use sigil_stitch::lang::dart::DartLang;
use sigil_stitch::lang::go_lang::GoLang;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::lang::java_lang::JavaLang;
use sigil_stitch::lang::javascript::JavaScript;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::lang::ocaml::OCaml;
use sigil_stitch::lang::python::Python;
use sigil_stitch::lang::rust_lang::RustLang;
use sigil_stitch::lang::scala::Scala;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::lang::zsh::Zsh;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

// ════════════════════════════════════════════════════════
// TypeScript
// ════════════════════════════════════════════════════════

#[test]
fn test_ts_macro_basic() {
    let block = sigil_quote!(TypeScript {
        const name = $S("Alice");
        const age = $L("30");
        console.log(name, age);
    })
    .unwrap();
    golden::assert_golden("typescript/macro_basic.ts", &render_ts(&block));
}

#[test]
fn test_ts_macro_control_flow() {
    let error_type = TypeName::<TypeScript>::importable_type("./errors", "NotFoundError");
    let block = sigil_quote!(TypeScript {
        if(!user) {
            throw new $T(error_type)($S("not found"));
        } else {
            return user;
        }
    })
    .unwrap();
    golden::assert_golden("typescript/macro_control_flow.ts", &render_ts(&block));
}

#[test]
fn test_ts_macro_imports() {
    let user_type = TypeName::<TypeScript>::importable_type("./models", "User");
    let repo_type = TypeName::<TypeScript>::importable_type("./repos", "UserRepository");
    let logger_type = TypeName::<TypeScript>::importable_type("./logging", "Logger");
    let block = sigil_quote!(TypeScript {
        const repo: $T(repo_type) = getRepo();
        const logger: $T(logger_type) = getLogger();
        const user: $T(user_type) = repo.findOne();
        logger.info($S("found user"));
    })
    .unwrap();
    golden::assert_golden("typescript/macro_imports.ts", &render_ts(&block));
}

#[test]
fn test_ts_macro_object_literal() {
    let block = sigil_quote!(TypeScript {
        const config = { timeout: 5000, retries: 3 };
        const nested = { a: 1, b: { c: 2 } };
    })
    .unwrap();
    golden::assert_golden("typescript/macro_object_literal.ts", &render_ts(&block));
}

// ════════════════════════════════════════════════════════
// JavaScript
// ════════════════════════════════════════════════════════

#[test]
fn test_js_macro_basic() {
    let block = sigil_quote!(JavaScript {
        const name = $S("Alice");
        const age = $L("30");
        console.log(name, age);
    })
    .unwrap();
    golden::assert_golden("javascript/macro_basic.js", &render_js(&block));
}

#[test]
fn test_js_macro_control_flow() {
    let block = sigil_quote!(JavaScript {
        if(x > 0) {
            return $S("positive");
        } else if(x < 0) {
            return $S("negative");
        } else {
            return $S("zero");
        }
    })
    .unwrap();
    golden::assert_golden("javascript/macro_control_flow.js", &render_js(&block));
}

#[test]
fn test_js_macro_imports() {
    let fs_type = TypeName::<JavaScript>::importable("fs", "readFileSync");
    let path_type = TypeName::<JavaScript>::importable("path", "join");
    let block = sigil_quote!(JavaScript {
        const data = $T(fs_type)($S("input.txt"));
        const full = $T(path_type)($S("dir"), $S("file"));
    })
    .unwrap();
    golden::assert_golden("javascript/macro_imports.js", &render_js(&block));
}

// ════════════════════════════════════════════════════════
// Java
// ════════════════════════════════════════════════════════

#[test]
fn test_java_macro_basic() {
    let block = sigil_quote!(JavaLang {
        String name = $S("Alice");
        int age = 30;
        System.out.println(name);
    })
    .unwrap();
    golden::assert_golden("java/macro_basic.java", &render_java(&block));
}

#[test]
fn test_java_macro_control_flow() {
    let block = sigil_quote!(JavaLang {
        if(x > 0) {
            return $S("positive");
        } else {
            return $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("java/macro_control_flow.java", &render_java(&block));
}

#[test]
fn test_java_macro_imports() {
    let list_type = TypeName::<JavaLang>::importable("java.util", "List");
    let map_type = TypeName::<JavaLang>::importable("java.util", "Map");
    let block = sigil_quote!(JavaLang {
        $T(list_type) items = new ArrayList<>();
        $T(map_type) lookup = new HashMap<>();
    })
    .unwrap();
    golden::assert_golden("java/macro_imports.java", &render_java(&block));
}

// ════════════════════════════════════════════════════════
// C
// ════════════════════════════════════════════════════════

#[test]
fn test_c_macro_basic() {
    let block = sigil_quote!(CLang {
        int x = 42;
        float y = 3.14;
        printf($S("x=%d y=%f"), x, y);
    })
    .unwrap();
    golden::assert_golden("c/macro_basic.c", &render_c(&block));
}

#[test]
fn test_c_macro_control_flow() {
    let block = sigil_quote!(CLang {
        if(x > 0) {
            return 1;
        } else if(x < 0) {
            return -1;
        } else {
            return 0;
        }
    })
    .unwrap();
    golden::assert_golden("c/macro_control_flow.c", &render_c(&block));
}

#[test]
fn test_c_macro_imports() {
    let stdio = TypeName::<CLang>::importable("stdio.h", "printf");
    let stdlib = TypeName::<CLang>::importable("stdlib.h", "malloc");
    let block = sigil_quote!(CLang {
        $T(stdio)($S("hello"));
        void* p = $T(stdlib)(sizeof(int));
    })
    .unwrap();
    golden::assert_golden("c/macro_imports.c", &render_c(&block));
}

// ════════════════════════════════════════════════════════
// C++
// ════════════════════════════════════════════════════════

#[test]
fn test_cpp_macro_basic() {
    let block = sigil_quote!(CppLang {
        int x = 42;
        std::string name = $S("Alice");
        std::cout << name << std::endl;
    })
    .unwrap();
    golden::assert_golden("cpp/macro_basic.cpp", &render_cpp(&block));
}

#[test]
fn test_cpp_macro_control_flow() {
    let block = sigil_quote!(CppLang {
        if(x > 0) {
            return true;
        } else {
            return false;
        }
    })
    .unwrap();
    golden::assert_golden("cpp/macro_control_flow.cpp", &render_cpp(&block));
}

#[test]
fn test_cpp_macro_imports() {
    let vector = TypeName::<CppLang>::importable("vector", "vector");
    let string = TypeName::<CppLang>::importable("string", "string");
    let block = sigil_quote!(CppLang {
        $T(vector) items;
        $T(string) name = $S("Alice");
    })
    .unwrap();
    golden::assert_golden("cpp/macro_imports.cpp", &render_cpp(&block));
}

#[test]
fn test_cpp_macro_includes() {
    let iostream = TypeName::<CppLang>::importable("iostream", "cout");
    let memory = TypeName::<CppLang>::importable("memory", "unique_ptr");
    let block = sigil_quote!(CppLang {
        auto ptr = std::make_unique<int>(42);
        $T(iostream) << $T(memory)(ptr.get()) << std::endl;
    })
    .unwrap();
    golden::assert_golden("cpp/macro_includes.cpp", &render_cpp(&block));
}

// ════════════════════════════════════════════════════════
// Rust
// ════════════════════════════════════════════════════════

#[test]
fn test_rust_macro_basic() {
    let block = sigil_quote!(RustLang {
        let x: i32 = 42;
        let name = $S("Alice");
        println!($S("{}: {}"), name, x);
    })
    .unwrap();
    golden::assert_golden("rust/macro_basic.rs", &render_rs(&block));
}

#[test]
fn test_rust_macro_control_flow() {
    let block = sigil_quote!(RustLang {
        if x > 0 {
            return Ok(x);
        } else {
            return Err($S("negative"));
        }
    })
    .unwrap();
    golden::assert_golden("rust/macro_control_flow.rs", &render_rs(&block));
}

#[test]
fn test_rust_macro_imports() {
    let hashmap = TypeName::<RustLang>::importable("std::collections", "HashMap");
    let vec_deque = TypeName::<RustLang>::importable("std::collections", "VecDeque");
    let block = sigil_quote!(RustLang {
        let map: $T(hashmap.clone()) = $T(hashmap)::new();
        let deque: $T(vec_deque.clone()) = $T(vec_deque)::new();
    })
    .unwrap();
    golden::assert_golden("rust/macro_imports.rs", &render_rs(&block));
}

#[test]
fn test_rust_macro_path_separator() {
    let block = sigil_quote!(RustLang {
        let size = std::mem::size_of::<u32>();
        let x = std::cmp::max(1, 2);
    })
    .unwrap();
    golden::assert_golden("rust/macro_path_separator.rs", &render_rs(&block));
}

// ════════════════════════════════════════════════════════
// Swift
// ════════════════════════════════════════════════════════

#[test]
fn test_swift_macro_basic() {
    let block = sigil_quote!(Swift {
        let name: String = $S("Alice");
        let age: Int = 30;
        print(name, age);
    })
    .unwrap();
    golden::assert_golden("swift/macro_basic.swift", &render_swift(&block));
}

#[test]
fn test_swift_macro_control_flow() {
    let block = sigil_quote!(Swift {
        if x > 0 {
            return true;
        } else {
            return false;
        }
    })
    .unwrap();
    golden::assert_golden("swift/macro_control_flow.swift", &render_swift(&block));
}

#[test]
fn test_swift_macro_imports() {
    let foundation = TypeName::<Swift>::importable("Foundation", "URL");
    let uikit = TypeName::<Swift>::importable("UIKit", "UIView");
    let block = sigil_quote!(Swift {
        let url: $T(foundation.clone()) = $T(foundation)(string: $S("https://example.com"));
        let view: $T(uikit.clone()) = $T(uikit)();
    })
    .unwrap();
    golden::assert_golden("swift/macro_imports.swift", &render_swift(&block));
}

// ════════════════════════════════════════════════════════
// Dart
// ════════════════════════════════════════════════════════

#[test]
fn test_dart_macro_basic() {
    let block = sigil_quote!(DartLang {
        final name = $S("Alice");
        final age = 30;
        print(name);
    })
    .unwrap();
    golden::assert_golden("dart/macro_basic.dart", &render_dart(&block));
}

#[test]
fn test_dart_macro_control_flow() {
    let block = sigil_quote!(DartLang {
        if(x > 0) {
            return true;
        } else {
            return false;
        }
    })
    .unwrap();
    golden::assert_golden("dart/macro_control_flow.dart", &render_dart(&block));
}

#[test]
fn test_dart_macro_imports() {
    let http = TypeName::<DartLang>::importable("package:http/http.dart", "Client");
    let convert = TypeName::<DartLang>::importable("dart:convert", "jsonDecode");
    let block = sigil_quote!(DartLang {
        final client = $T(http)();
        final data = $T(convert)(response.body);
    })
    .unwrap();
    golden::assert_golden("dart/macro_imports.dart", &render_dart(&block));
}

// ════════════════════════════════════════════════════════
// Go
// ════════════════════════════════════════════════════════

#[test]
fn test_go_macro_basic() {
    let block = sigil_quote!(GoLang {
        x := 42;
        name := $S("Alice");
        fmt.Println(name, x);
    })
    .unwrap();
    golden::assert_golden("go/macro_basic.go", &render_go(&block));
}

#[test]
fn test_go_macro_control_flow() {
    let block = sigil_quote!(GoLang {
        if x > 0 {
            return $S("positive");
        } else {
            return $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("go/macro_control_flow.go", &render_go(&block));
}

#[test]
fn test_go_macro_indent() {
    let block = sigil_quote!(GoLang {
        namespace Foo {
        $>
        North = iota;
        East;
        South;
        West;
        $<
        }
    })
    .unwrap();
    golden::assert_golden("go/macro_indent.go", &render_go(&block));
}

// ════════════════════════════════════════════════════════
// Kotlin
// ════════════════════════════════════════════════════════

#[test]
fn test_kotlin_macro_basic() {
    let block = sigil_quote!(Kotlin {
        val name = $S("Alice");
        val age = 30;
        println(name);
    })
    .unwrap();
    golden::assert_golden("kotlin/macro_basic.kt", &render_kt(&block));
}

#[test]
fn test_kotlin_macro_control_flow() {
    let block = sigil_quote!(Kotlin {
        if(x > 0) {
            return $S("positive");
        } else {
            return $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("kotlin/macro_control_flow.kt", &render_kt(&block));
}

#[test]
fn test_kotlin_macro_imports() {
    let list_of = TypeName::<Kotlin>::importable("kotlin.collections", "listOf");
    let block = sigil_quote!(Kotlin {
        val items = $T(list_of)(1, 2, 3);
    })
    .unwrap();
    golden::assert_golden("kotlin/macro_imports.kt", &render_kt(&block));
}

// ════════════════════════════════════════════════════════
// Scala
// ════════════════════════════════════════════════════════

#[test]
fn test_scala_macro_basic() {
    let block = sigil_quote!(Scala {
        val name = $S("Alice");
        val age = 30;
        println(name);
    })
    .unwrap();
    golden::assert_golden("scala/macro_basic.scala", &render_scala(&block));
}

#[test]
fn test_scala_macro_control_flow() {
    let block = sigil_quote!(Scala {
        if(x > 0) {
            return $S("positive");
        } else {
            return $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("scala/macro_control_flow.scala", &render_scala(&block));
}

#[test]
fn test_scala_macro_imports() {
    let list_buffer = TypeName::<Scala>::importable("scala.collection.mutable", "ListBuffer");
    let block = sigil_quote!(Scala {
        val buf = new $T(list_buffer)();
    })
    .unwrap();
    golden::assert_golden("scala/macro_imports.scala", &render_scala(&block));
}

// ════════════════════════════════════════════════════════
// Python
// ════════════════════════════════════════════════════════

#[test]
fn test_python_macro_basic() {
    let block = sigil_quote!(Python {
        name = $S("Alice");
        age = 30;
        print(name, age);
    })
    .unwrap();
    golden::assert_golden("python/macro_basic.py", &render_py(&block));
}

#[test]
fn test_python_macro_control_flow() {
    let block = sigil_quote!(Python {
        if x > 0 {
            return $S("positive");
        } else {
            return $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("python/macro_control_flow.py", &render_py(&block));
}

// ════════════════════════════════════════════════════════
// Haskell
// ════════════════════════════════════════════════════════

#[test]
fn test_haskell_macro_basic() {
    let block = sigil_quote!(Haskell {
        let x = 42;
        putStrLn $S("hello");
    })
    .unwrap();
    golden::assert_golden("haskell/macro_basic.hs", &render_hs(&block));
}

#[test]
fn test_haskell_macro_control_flow() {
    let block = sigil_quote!(Haskell {
        if x > 0 {
            return True;
        } else {
            return False;
        }
    })
    .unwrap();
    golden::assert_golden("haskell/macro_control_flow.hs", &render_hs(&block));
}

#[test]
fn test_haskell_macro_open_where() {
    let block = sigil_quote!(Haskell {
        class Functor f $open(" where") {
            fmap :: (a -> b) -> f a -> f b;
        }
    })
    .unwrap();
    golden::assert_golden("haskell/macro_open_where.hs", &render_hs(&block));
}

// ════════════════════════════════════════════════════════
// OCaml
// ════════════════════════════════════════════════════════

#[test]
fn test_ocaml_macro_basic() {
    let block = sigil_quote!(OCaml {
        let x = 42;
        let name = $S("Alice");
    })
    .unwrap();
    golden::assert_golden("ocaml/macro_basic.ml", &render_ml(&block));
}

#[test]
fn test_ocaml_macro_control_flow() {
    let block = sigil_quote!(OCaml {
        if x > 0 {
            return true;
        } else {
            return false;
        }
    })
    .unwrap();
    golden::assert_golden("ocaml/macro_control_flow.ml", &render_ml(&block));
}

#[test]
fn test_ocaml_macro_open_struct() {
    let block = sigil_quote!(OCaml {
        module Foo $open(" = struct") {
            let x = 42;
            let name = $S("Alice");
        }
    })
    .unwrap();
    golden::assert_golden("ocaml/macro_open_struct.ml", &render_ml(&block));
}

// ════════════════════════════════════════════════════════
// Bash
// ════════════════════════════════════════════════════════

#[test]
fn test_bash_macro_basic() {
    let block = sigil_quote!(Bash {
        NAME=$S("Alice");
        AGE=30;
        echo $L("$NAME") $L("$AGE");
    })
    .unwrap();
    golden::assert_golden("bash/macro_basic.bash", &render_bash(&block));
}

#[test]
fn test_bash_macro_control_flow() {
    let block = sigil_quote!(Bash {
        if [ $L("$x") -gt 0 ] {
            echo $S("positive");
        } else {
            echo $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("bash/macro_control_flow.bash", &render_bash(&block));
}

// ════════════════════════════════════════════════════════
// Zsh
// ════════════════════════════════════════════════════════

#[test]
fn test_zsh_macro_basic() {
    let block = sigil_quote!(Zsh {
        NAME=$S("Alice");
        AGE=30;
        echo $L("$NAME") $L("$AGE");
    })
    .unwrap();
    golden::assert_golden("zsh/macro_basic.zsh", &render_zsh(&block));
}

#[test]
fn test_zsh_macro_control_flow() {
    let block = sigil_quote!(Zsh {
        if [ $L("$x") -gt 0 ] {
            echo $S("positive");
        } else {
            echo $S("negative");
        }
    })
    .unwrap();
    golden::assert_golden("zsh/macro_control_flow.zsh", &render_zsh(&block));
}

// ════════════════════════════════════════════════════════
// Render helpers
// ════════════════════════════════════════════════════════

fn render_ts(block: &CodeBlock<TypeScript>) -> String {
    let mut fb = FileSpec::<TypeScript>::builder("test.ts");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_js(block: &CodeBlock<JavaScript>) -> String {
    let mut fb = FileSpec::<JavaScript>::builder("test.js");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_java(block: &CodeBlock<JavaLang>) -> String {
    let mut fb = FileSpec::<JavaLang>::builder("Test.java");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_c(block: &CodeBlock<CLang>) -> String {
    let mut fb = FileSpec::<CLang>::builder("test.c");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_cpp(block: &CodeBlock<CppLang>) -> String {
    let mut fb = FileSpec::<CppLang>::builder("test.cpp");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_rs(block: &CodeBlock<RustLang>) -> String {
    let mut fb = FileSpec::<RustLang>::builder("test.rs");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_swift(block: &CodeBlock<Swift>) -> String {
    let mut fb = FileSpec::<Swift>::builder("test.swift");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_dart(block: &CodeBlock<DartLang>) -> String {
    let mut fb = FileSpec::<DartLang>::builder("test.dart");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_go(block: &CodeBlock<GoLang>) -> String {
    let mut fb = FileSpec::<GoLang>::builder("test.go");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_kt(block: &CodeBlock<Kotlin>) -> String {
    let mut fb = FileSpec::<Kotlin>::builder("test.kt");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_scala(block: &CodeBlock<Scala>) -> String {
    let mut fb = FileSpec::<Scala>::builder("test.scala");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_py(block: &CodeBlock<Python>) -> String {
    let mut fb = FileSpec::<Python>::builder("test.py");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_hs(block: &CodeBlock<Haskell>) -> String {
    let mut fb = FileSpec::builder_with("test.hs", Haskell::new());
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_ml(block: &CodeBlock<OCaml>) -> String {
    let mut fb = FileSpec::builder_with("test.ml", OCaml::new());
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_bash(block: &CodeBlock<Bash>) -> String {
    let mut fb = FileSpec::<Bash>::builder("test.bash");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}

fn render_zsh(block: &CodeBlock<Zsh>) -> String {
    let mut fb = FileSpec::<Zsh>::builder("test.zsh");
    fb.add_code(block.clone());
    fb.build().unwrap().render(80).unwrap()
}
