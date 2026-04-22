#![warn(missing_docs)]

//! # Sigil-Stitch
//!
//! Type-safe, import-aware, width-aware code generation for multiple languages.
//!
//! Sigil-Stitch combines JavaPoet's builder + CodeBlock model with Wadler-Lindig
//! pretty printing and multi-language support. Reference types with `%T` in format
//! strings, and the library tracks every import for you, resolves naming conflicts,
//! and emits width-aware formatted output.
//!
//! ## Format Specifiers
//!
//! | Specifier | Name | Argument Type | Purpose |
//! |-----------|------|---------------|---------|
//! | `%T` | Type | `TypeName<L>` | Emit type reference, track import |
//! | `%N` | Name | `&str` or `Nameable` | Emit identifier name |
//! | `%S` | String | `&str` | Emit escaped string literal |
//! | `%L` | Literal | `&str`, number, `CodeBlock<L>` | Emit raw value or nested block |
//! | `%W` | Wrap | (none) | Soft line break point |
//! | `%>` | Indent | (none) | Increase indent level |
//! | `%<` | Dedent | (none) | Decrease indent level |
//! | `%[` | Statement begin | (none) | Start of statement |
//! | `%]` | Statement end | (none) | End of statement (appends `;` if needed) |
//!
//! ## Quick Example
//!
//! ```rust
//! use sigil_stitch::prelude::*;
//! use sigil_stitch::lang::typescript::TypeScript;
//!
//! let user_type = TypeName::<TypeScript>::importable_type("./models", "User");
//!
//! let mut cb = CodeBlock::<TypeScript>::builder();
//! cb.add_statement("const user = await getUser(%S)", ("id",));
//! cb.add_statement("return user as %T", (user_type.clone(),));
//! let body = cb.build().unwrap();
//!
//! let mut fb = FileSpec::<TypeScript>::builder("user.ts");
//! fb.add_code(body);
//! let file = fb.build().unwrap();
//!
//! let output = file.render(80).unwrap();
//! assert!(output.contains("import type { User } from './models'"));
//! ```
//!
//! ## `sigil_quote!` Macro
//!
//! For less ceremony, write target-language code inline with the `sigil_quote!` macro:
//!
//! ```
//! use sigil_stitch::prelude::*;
//! use sigil_stitch::lang::typescript::TypeScript;
//!
//! let user_type = TypeName::<TypeScript>::importable_type("./models", "User");
//!
//! let block = sigil_quote!(TypeScript {
//!     const user: $T(user_type) = await getUser($S("id"));
//!     if (!user) {
//!         throw new Error($S("not found"));
//!     }
//!     return user;
//! }).unwrap();
//! ```
//!
//! Interpolation markers: `$T` (type), `$N` (name), `$S` (string literal),
//! `$L` (literal), `$C` (nested code block), `$W` (soft line break), `$$` (escape).
//!
//! See the full guide at `docs/src/sigil_quote.md` for control flow, limitations,
//! and advanced usage.

/// Composable code fragments with format specifiers (`%T`, `%N`, `%S`, `%L`, etc.).
pub mod code_block;
/// Rendering engine that walks `CodeBlock` format parts into final output.
pub mod code_renderer;
/// Reusable named-parameter templates that produce `CodeBlock`s.
pub mod code_template;
/// Error types for sigil-stitch.
pub mod error;
/// Import types, grouping, and conflict resolution.
pub mod import;
/// Walks `CodeBlock` trees to extract all import references.
pub mod import_collector;
/// Language abstraction trait and per-language implementations.
pub mod lang;
/// Collision-free name allocation for generated identifiers.
pub mod name_allocator;
/// Structural builders (TypeSpec, FunSpec, FileSpec, etc.) that emit `CodeBlock`s.
pub mod spec;
/// Type references with recursive import tracking and pretty-printing.
pub mod type_name;

/// Common re-exports for convenient usage.
pub mod prelude {
    pub use crate::code_block::{CodeBlock, CodeBlockBuilder};
    pub use crate::code_template::{CodeTemplate, ParamKind};
    pub use crate::error::SigilStitchError;
    pub use crate::lang::CodeLang;
    pub use crate::spec::field_spec::FieldSpec;
    pub use crate::spec::file_spec::FileSpec;
    pub use crate::spec::fun_spec::{FunSpec, TypeParamSpec};
    pub use crate::spec::modifiers::{DeclarationContext, Modifiers, TypeKind, Visibility};
    pub use crate::spec::parameter_spec::ParameterSpec;
    pub use crate::spec::project_spec::{ProjectSpec, RenderedFile};
    pub use crate::spec::type_spec::TypeSpec;
    pub use crate::type_name::TypeName;
    pub use sigil_stitch_macros::sigil_quote;
}
