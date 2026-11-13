use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        const direct = $L(missing_direct);
        const embedded = $V("prefix @{missing_embedded}");
        const nested = $L(r#"@{Some(missing_nested).unwrap()}"#);
    });
}
