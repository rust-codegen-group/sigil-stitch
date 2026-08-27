use sigil_stitch::code_block::{CodeBlock, CodeFragment};
#[expect(deprecated, reason = "0.6.8 quote compatibility assertion")]
use sigil_stitch::lang::config::QuoteStyle;
use sigil_stitch::lang::python::Python;
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::prelude::*;
use sigil_stitch::{assert_quote, assert_rendered};

#[test]
fn assert_rendered_matches_manual_codeblock() {
    let block = CodeBlock::of("const x = 1", ()).unwrap();

    assert_rendered!(TypeScript::new(), block, "const x = 1\n");
}

#[test]
fn assert_quote_matches_inline_quote() {
    assert_quote!(TypeScript, {
        const x = 1;
    }, "const x = 1;\n");
}

#[test]
#[expect(deprecated, reason = "0.6.8 quote compatibility assertion")]
fn assert_rendered_uses_configured_language() {
    let block = sigil_quote!(Python {
        print($S("hi"))
    })
    .unwrap();

    assert_rendered!(
        Python::new().with_quote_style(QuoteStyle::Double),
        block,
        "print(\"hi\")\n",
    );
}

#[test]
fn assert_rendered_includes_imports() {
    let user = TypeName::importable_type("./models", "User");
    let block = CodeBlock::of("const user: %T = getUser()", (user,)).unwrap();

    assert_rendered!(
        TypeScript::new(),
        block,
        "import type { User } from './models';\n\nconst user: User = getUser()\n",
    );
}

#[test]
fn assert_quote_accepts_width() {
    assert_quote!(TypeScript, width = 12, {
        call($W first, $W second, $W third);
    }, "call( first,\nsecond,\nthird);\n");
}

#[test]
fn assert_rendered_handles_code_fragment_composition() {
    let fragment = CodeFragment::of("if enabled:\n%>return value%<", ()).unwrap();
    let block = sigil_quote!(Python {
        def choose(enabled: bool, value: str) -> str: {
            $L(fragment)
            return "fallback"
        }
    })
    .unwrap();

    assert_rendered!(
        Python::new(),
        block,
        "def choose(enabled: bool, value: str) -> str:\n    if enabled:\n        return value\n    return \"fallback\"\n\n",
    );
}
