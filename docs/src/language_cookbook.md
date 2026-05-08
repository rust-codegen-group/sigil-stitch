# Language Cookbook

This chapter collects practical, copy-paste-ready recipes for each supported language. Each example shows the builder calls and the rendered output. For the full API of each spec type, see [Building Functions & Fields](functions_and_fields.md), [Building Types & Enums](types_and_enums.md), and [Files & Projects](files_and_projects.md).

## Languages

- [TypeScript](cookbook_typescript.md) -- class with imports, interface with generics, type alias, enum, abstract class
- [Rust](cookbook_rust.md) -- struct with impl, enum with variants, newtype, trait, type alias
- [Go](cookbook_go.md) -- struct with tags, newtype, interface, generic function
- [Python](cookbook_python.md) -- function with type hints, type alias, class with bases, dataclass, enum
- [Java](cookbook_java.md) -- class with annotations, interface, enum, abstract class
- [Kotlin](cookbook_kotlin.md) -- data class, enum, interface, suspend function
- [Swift](cookbook_swift.md) -- struct with protocol conformance, enum, enum with associated values, protocol
- [C++](cookbook_cpp.md) -- class with template, using alias, enum class, virtual method, namespace wrapping
- [C](cookbook_c.md) -- typedef, struct with fields, function declaration, enum
- [C#](cookbook_csharp.md) -- class with XML doc, interface, enum, record with imports
- [Lua](cookbook_lua.md) -- function, module with require, control flow, table constructor
- [Scala](cookbook_scala.md) -- case class, trait with type parameter, enum, bounded type parameter, newtype
- [Haskell](cookbook_haskell.md) -- data record with deriving, type class, function with split signature, newtype, type alias
- [OCaml](cookbook_ocaml.md) -- record type, function with curried params, module block, type alias, pattern match

## Cross-language comparison

The same logical concept -- a simple data type with two fields -- rendered across four languages from the same builder structure:

| Language   | Output |
|------------|--------|
| TypeScript | `export class Point { x: number; y: number; }` |
| Rust       | `pub struct Point { pub x: f64, pub y: f64, }` + separate `impl` block |
| Go         | `type Point struct { X float64; Y float64 }` |
| Python     | `class Point: x: float; y: float` |
| C#         | `public class Point { public double X; public double Y; }` |
| Lua        | (no type system -- use `CodeBlock` directly for table constructors) |
| Scala      | `case class Point(x: Double, y: Double)` |
| Haskell    | `data Point = Point { pointX :: Double, pointY :: Double }` |
| OCaml      | `type point = { x : float; y : float }` |

The language's `CodeLang` trait controls every syntax detail: keywords, delimiters, field ordering, visibility rendering, and whether methods live inside the type body or in a separate `impl` block. You build the spec once and the language passed to `render()` does the rest.
