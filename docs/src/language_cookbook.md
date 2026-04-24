# Language Cookbook

This chapter collects practical, copy-paste-ready recipes for each supported language. Each example shows the builder calls and the rendered output. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Languages

- [TypeScript](cookbook_typescript.md) -- class with imports, interface with generics, type alias
- [Rust](cookbook_rust.md) -- struct with impl, enum with variants, newtype
- [Go](cookbook_go.md) -- struct with tags, newtype
- [Python](cookbook_python.md) -- function with type hints, type alias
- [Java](cookbook_java.md) -- class with annotations
- [Kotlin](cookbook_kotlin.md) -- data class
- [Swift](cookbook_swift.md) -- struct with protocol conformance
- [C++](cookbook_cpp.md) -- class with template, using alias
- [C](cookbook_c.md) -- typedef

## Cross-language comparison

The same logical concept -- a simple data type with two fields -- rendered across four languages from the same builder structure:

| Language   | Output |
|------------|--------|
| TypeScript | `export class Point { x: number; y: number; }` |
| Rust       | `pub struct Point { pub x: f64, pub y: f64, }` + separate `impl` block |
| Go         | `type Point struct { X float64; Y float64 }` |
| Python     | `class Point: x: float; y: float` |

The language's `CodeLang` trait controls every syntax detail: keywords, delimiters, field ordering, visibility rendering, and whether methods live inside the type body or in a separate `impl` block. You build the spec once and the language passed to `render()` does the rest.
