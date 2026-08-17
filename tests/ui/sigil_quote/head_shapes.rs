use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!({ const value = 1; });
    let _ = sigil_quote!(TypeScript);
    let _ = sigil_quote!(TypeScript + JavaScript {});
}
