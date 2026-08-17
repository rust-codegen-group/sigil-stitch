use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        const value = $Bogus(foo) $N(struct);
    });
}
