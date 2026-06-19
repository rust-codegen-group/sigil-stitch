//! Showcase: every `sigil_quote!` interpolation marker in one file.
//!
//! Run: `cargo run --example macro_features`

use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::lang::kotlin::Kotlin;
use sigil_stitch::prelude::*;

fn render_ts(block: &CodeBlock) -> String {
    FileSpec::builder("demo.ts")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(60)
        .unwrap()
}

fn main() {
    type_reference();
    name_identifier();
    string_literal();
    literal_value();
    nested_code_block();
    soft_line_break();
    join_separator();
    meta_for_separator();
    dollar_escape();
    comment_directive();
    indent_dedent();
    line_continuation();
    open_override();
    else_if_chain();
}

/// `$T(expr)` — type reference with automatic import tracking.
fn type_reference() {
    println!("--- $T: Type Reference ---\n");
    let user = TypeName::importable_type("./models", "User");
    let block = sigil_quote!(TypeScript {
        const user: $T(user) = await getUser();
    })
    .unwrap();
    println!("{}", render_ts(&block));
}

/// `$N(expr)` — identifier name with keyword escaping.
fn name_identifier() {
    println!("--- $N: Name Identifier ---\n");

    // Reserved word escaping: "type" → "type_" in TypeScript
    let field_name = "type";
    let block = sigil_quote!(TypeScript {
        const $N(field_name) = getValue();
    })
    .unwrap();
    println!("Keyword escaping ({field_name}):");
    println!("{}", render_ts(&block));

    // Dynamic name construction: build getter names from field names
    let field = "userName";
    let getter = format!(
        "get{}",
        field.chars().next().unwrap().to_uppercase().to_string() + &field[1..]
    );
    let block = sigil_quote!(TypeScript {
        $N(getter)(): string;
    })
    .unwrap();
    println!("Dynamic name ({getter}):");
    println!("{}", render_ts(&block));

    // Member access: this.$N(name) for dynamic property access
    let prop = "email";
    let block = sigil_quote!(TypeScript {
        this.$N(prop) = value;
    })
    .unwrap();
    println!("Member access (this.{prop}):");
    println!("{}", render_ts(&block));
}

/// `$S(expr)` — language-aware string literal.
fn string_literal() {
    println!("--- $S: String Literal ---\n");
    let block = sigil_quote!(TypeScript {
        console.log($S("hello world"));
    })
    .unwrap();
    println!("{}", render_ts(&block));
}

/// `$L(expr)` — raw literal value, no transformation.
fn literal_value() {
    println!("--- $L: Literal Value ---\n");
    let default_port = 8080;
    let block = sigil_quote!(TypeScript {
        const port = $L(default_port.to_string());
    })
    .unwrap();
    println!("{}", render_ts(&block));
}

/// `$C(expr)` — splice a nested CodeBlock inline.
fn nested_code_block() {
    println!("--- $C: Nested Code Block ---\n");
    let greeting = CodeBlock::of("'hello'", ()).unwrap();
    let block = sigil_quote!(TypeScript {
        console.log($C(greeting));
    })
    .unwrap();
    println!("{}", render_ts(&block));
}

/// `$W` — soft line-break point for the pretty printer.
fn soft_line_break() {
    println!("--- $W: Soft Line Break ---\n");
    let block = sigil_quote!(TypeScript {
        createUser($W firstName, $W lastName, $W email, $W role);
    })
    .unwrap();
    println!("At width 40:");
    let narrow = FileSpec::builder("demo.ts")
        .add_code(block.clone())
        .build()
        .unwrap()
        .render(40)
        .unwrap();
    println!("{narrow}");
    println!("At width 120:");
    let wide = FileSpec::builder("demo.ts")
        .add_code(block)
        .build()
        .unwrap()
        .render(120)
        .unwrap();
    println!("{wide}");
}

/// `$join(sep, iter)` — join items with a separator.
fn join_separator() {
    println!("--- $join: Separator Join ---\n");
    let fields = vec!["name", "age", "email"];
    let block = sigil_quote!(TypeScript {
        const keys = [$join(", ", fields)];
    })
    .unwrap();
    println!("{}", render_ts(&block));
}

/// `$for(...; separator = ...)` — emit structured bodies with separators.
fn meta_for_separator() {
    println!("--- $for: Structured Separator Loop ---\n");

    let handlers = vec!["createUser", "updateUser", "deleteUser"];

    let block = sigil_quote!(TypeScript {
        $for(handler in &handlers; separator = "\n") {
            export function $N(*handler)() {
                return runHandler($S(*handler));
            }
        }
    })
    .unwrap();

    println!("Blank-line separated generated functions:");
    println!("{}", render_ts(&block));

    let members = vec![
        TypeName::primitive("Cat"),
        TypeName::primitive("Dog"),
        TypeName::primitive("null"),
    ];
    let block = sigil_quote!(TypeScript {
        export type Pet =
          $for(member in &members; separator = "\n| ") { $T((*member).clone()) };
    })
    .unwrap();

    println!("Inline union members with continuation separators:");
    println!("{}", render_ts(&block));
}

/// `$$` — literal dollar sign in output.
fn dollar_escape() {
    println!("--- $$: Dollar Escape ---\n");
    let block = sigil_quote!(TypeScript {
        const price = $$ + amount;
    })
    .unwrap();
    println!("{}", render_ts(&block));
}

/// `$comment("text")` — language-appropriate comment.
fn comment_directive() {
    println!("--- $comment: Comment ---\n");
    let block = sigil_quote!(TypeScript {
        $comment("Initialize the connection pool");
        const pool = createPool();
    })
    .unwrap();
    println!("{}", render_ts(&block));
}

/// `$>` / `$<` — manual indent and dedent.
fn indent_dedent() {
    println!("--- $> / $<: Manual Indent ---\n");
    let block = sigil_quote!(TypeScript {
        namespace Foo {
        $>
        const x = 1;
        const y = 2;
        $<
        }
    })
    .unwrap();
    println!("{}", render_ts(&block));
}

/// `$+` — line continuation (merge next source line into current statement).
fn line_continuation() {
    println!("--- $+: Line Continuation ---\n");
    let block = sigil_quote!(Kotlin {
        val result = someFunction( $+
            arg1, arg2)
    })
    .unwrap();
    let output = FileSpec::builder_with("demo.kt", Kotlin::new())
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    println!("{output}");
}

/// Context-aware block delimiters — Haskell `class` uses `where` via `block_open_for`.
fn open_override() {
    println!("--- Context-Aware Block Opener ---\n");
    let block = sigil_quote!(Haskell {
        class Functor f {
            fmap :: (a -> b) -> f a -> f b;
        }
    })
    .unwrap();
    let output = FileSpec::builder_with("demo.hs", Haskell::new())
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap();
    println!("{output}");
}

/// `$else_if` / `$else` — multi-branch conditional code generation.
fn else_if_chain() {
    println!("--- $else_if: Conditional Chain ---\n");

    let env = "staging";
    let block = sigil_quote!(TypeScript {
        $if(env == "production") {
            const apiUrl = $S("https://api.prod.example.com");
        } $else_if(env == "staging") {
            const apiUrl = $S("https://api.staging.example.com");
        } $else {
            const apiUrl = $S("http://localhost:3000");
        }
    })
    .unwrap();

    println!("With env = \"staging\":");
    println!("{}", render_ts(&block));

    // ── $T_join — type join with import tracking ──────────
    println!();
    println!("--- $T_join: Type Union with Import Tracking ---");
    println!();

    let variants = vec![
        TypeName::importable_type("./events", "UserCreated"),
        TypeName::importable_type("./events", "UserDeleted"),
        TypeName::primitive("null"),
    ];

    let block = sigil_quote!(TypeScript {
        export type UserEvent = $T_join(" | ", &variants);
    })
    .unwrap();

    println!("{}", render_ts(&block));
}
