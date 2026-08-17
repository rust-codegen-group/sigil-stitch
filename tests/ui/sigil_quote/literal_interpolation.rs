use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        const value = $V(r#"@{let} and @{also let}"#);
        const empty = $L("prefix @{}");
        const unclosed = $V(r#"prefix @{value"#);
        const raw = $L(r#"@{struct}"#);
    });
}
