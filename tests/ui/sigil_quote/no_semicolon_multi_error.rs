use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        const first = $L(let)
        const second = $N(struct)
    });
}
