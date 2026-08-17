//! Proc macros for sigil-stitch code generation.
//!
//! This crate provides the `sigil_quote!` macro for writing target-language code
//! inline with interpolation markers that expand to `CodeBlockBuilder` calls.

mod codegen;
mod guard_plan;
mod ir;
mod parse;

use proc_macro::TokenStream;

/// Write target-language code inline, expanding to `CodeBlockBuilder` calls.
///
/// # Syntax
///
/// ```ignore
/// sigil_quote!(LangType {
///     statement with $T(type_expr) and $S("string");
///     if (condition) {
///         body;
///     }
/// })
/// ```
///
/// Returns `Result<CodeBlock, SigilStitchError>`. Failures from generated
/// nested builders are returned without panicking, and later generated
/// expressions are not evaluated after the first such failure.
///
/// ## Interpolation Markers
///
/// | Syntax | Specifier | Purpose |
/// |--------|-----------|---------|
/// | `$T(expr)` | `%T` | Type reference (tracks imports) |
/// | `$N(expr)` | `%N` | Name identifier |
/// | `$S(expr)` | `%S` | String literal |
/// | `$V(expr)` | `%V` | Verbatim string |
/// | `$L(expr)` | `%L` | Literal or nested code |
/// | `$C(expr)` | `%L` | Nested `CodeBlock` |
/// | `$W` | `%W` | Soft line-break point |
/// | `$>` | `%>` | Increase indent |
/// | `$<` | `%<` | Decrease indent |
/// | `$$` | `$` | Literal dollar sign |
///
/// Direct ordinary and raw string literals passed to `$V`, `$L`, `$comment`,
/// or `$attr` can embed Rust expressions as `@{expr}`. The macro decodes the
/// literal and validates each expression at compile time. Use `@@` for one
/// literal `@`. Non-literal expressions are evaluated normally at runtime and
/// are never scanned for interpolation syntax.
///
/// Rust expressions, `$for` patterns, and `$let` bindings are parsed before
/// code generation. Independent errors are reported together when recovery can
/// reach a reliable sibling boundary.
///
/// ## Statement Rules
///
/// - Lines ending with `;` become `add_statement()` calls
/// - Lines ending with `{ ... }` become control flow (`begin/end_control_flow`)
/// - `{ ... };` (brace group followed by `;`) is treated as a statement, not control flow
/// - Blank lines become `add_line()` calls
/// - `$comment("text")` becomes `add_comment("text")`
/// - `$>` / `$<` increase / decrease indent level
///
/// ## Meta-Conditionals and Meta-Loops
///
/// Use `$if`/`$else_if`/`$else` to conditionally emit builder calls at runtime,
/// and `$for` to loop over a collection and emit per-item statements:
///
/// ```ignore
/// sigil_quote!(TypeScript {
///     $if(use_strict) {
///         "use strict";
///     }
///     $for((name, ty) in &fields) {
///         let $N(*name): $L(*ty);
///     }
/// })
/// ```
///
/// ## Control Flow
///
/// The macro detects `if`/`else`/`else if` chains, `for`, `while`, `try`/`catch`,
/// and any other construct that ends with a brace group:
///
/// ```ignore
/// sigil_quote!(TypeScript {
///     if (x > 0) {
///         return 1;
///     } else if (x < 0) {
///         return -1;
///     } else {
///         return 0;
///     }
/// })
/// ```
///
/// ## Context-Aware Block Delimiters
///
/// By default, `{ ... }` uses the language's `block_syntax().block_open`. Language
/// backends can override the opener and closer per condition via `block_open_for`
/// and `block_close_for`. For example, Bash maps `if` → `then`/`fi` and
/// `for` → `do`/`done`, while Haskell maps `class` → `where`:
///
/// ```ignore
/// sigil_quote!(Bash {
///     if [ -f "$$file" ]; {
///         echo "found"
///     }
/// })
/// // renders: if [ -f "$file" ]; then\n    echo "found"\nfi
/// ```
///
/// ## Limitations
///
/// - `//` comments are invisible to proc macros; use `$comment("text")` instead
/// - Single-quoted strings (`'hello'`) tokenize as Rust lifetimes; use `$S("hello")`
/// - No space is inserted before `(` after identifiers: `if(x)` not `if (x)`
/// - Template literals (`` `${expr}` ``) aren't supported; use `$L(expr)`
///
/// See the full guide at `docs/src/sigil_quote.md` for more details and examples.
///
/// # Examples
///
/// Basic statements with type interpolation:
///
/// ```ignore
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let user_type = TypeName::importable_type("./models", "User");
///
/// let block = sigil_quote!(TypeScript {
///     const user: $T(user_type) = await getUser($S("id"));
///     return user;
/// })?;
/// ```
///
/// Control flow with interpolation:
///
/// ```ignore
/// let error_type = TypeName::importable_type("./errors", "NotFoundError");
///
/// let block = sigil_quote!(TypeScript {
///     if (!user) {
///         throw new $T(error_type)($S("not found"));
///     }
/// })?;
/// ```
#[proc_macro]
pub fn sigil_quote(input: TokenStream) -> TokenStream {
    let input2: proc_macro2::TokenStream = input.into();
    match parse::parse_input(input2) {
        Ok(parsed) => match guard_plan::validate(&parsed) {
            Ok(()) => codegen::generate(parsed).into(),
            Err(err) => {
                let errors = err.into_compile_error();
                quote::quote!({ #errors }).into()
            }
        },
        Err(err) => {
            let errors = err.into_compile_error();
            quote::quote!({ #errors }).into()
        }
    }
}
