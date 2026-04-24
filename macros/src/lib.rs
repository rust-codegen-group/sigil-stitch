//! Proc macros for sigil-stitch code generation.
//!
//! This crate provides the `sigil_quote!` macro for writing target-language code
//! inline with interpolation markers that expand to `CodeBlockBuilder` calls.

mod codegen;
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
/// Returns `Result<CodeBlock, SigilStitchError>`.
///
/// ## Interpolation Markers
///
/// | Syntax | Specifier | Purpose |
/// |--------|-----------|---------|
/// | `$T(expr)` | `%T` | Type reference (tracks imports) |
/// | `$N(expr)` | `%N` | Name identifier |
/// | `$S(expr)` | `%S` | String literal |
/// | `$L(expr)` | `%L` | Literal or nested code |
/// | `$C(expr)` | `%L` | Nested `CodeBlock` |
/// | `$W` | `%W` | Soft line-break point |
/// | `$open("text")` | — | Custom block opener (see below) |
/// | `$>` | `%>` | Increase indent |
/// | `$<` | `%<` | Decrease indent |
/// | `$$` | `$` | Literal dollar sign |
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
/// ## Custom Block Openers (`$open`)
///
/// By default, `{ ... }` uses the language's `block_syntax().block_open` (e.g., `" {"` for
/// brace languages, `":"` for Python, `" ="` for Haskell). Use `$open("text")`
/// before `{` to override the opener for that block:
///
/// ```ignore
/// // Haskell type class: "class Functor f where" instead of "class Functor f ="
/// sigil_quote!(Haskell {
///     class Functor f $open(" where") {
///         fmap :: (a -> b) -> f a -> f b;
///     }
/// })
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
        Ok(parsed) => codegen::generate(parsed).into(),
        Err(err) => err.into_compile_error().into(),
    }
}
