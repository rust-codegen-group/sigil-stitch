use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        $if(let) {
            const first = $N(struct);
        } $else_if true {
            value;
        }
    });
}
