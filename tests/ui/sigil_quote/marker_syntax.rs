use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        const unknown = $Bogus(value);
        const missing = $L value;
        $else_if(true) {
            value;
        }
        $else {
            value;
        }
    });
}
