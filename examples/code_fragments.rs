//! Compose parsed fragments without losing indentation state.
//!
//! Run: `cargo run --example code_fragments`

use sigil_stitch::lang::python::Python;
use sigil_stitch::prelude::*;

fn main() {
    let early_return = CodeFragment::of("if enabled:\n%>return value%<", ()).unwrap();

    let block = sigil_quote!(Python {
        def choose(enabled: bool, value: str) -> str: {
            $L(early_return)
            return "fallback"
        }
    })
    .unwrap();

    let output = FileSpec::builder_with("demo.py", Python::new())
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    println!("{output}");
}
