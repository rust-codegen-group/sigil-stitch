use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        $if(let) {
            enabled;
        }
        $for(item + 1 in items) {
            value;
        }
        $for(item in let) {
            value;
        }
        $let(name + 1);
    });
}
