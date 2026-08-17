use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        $for(item in items; separator = ",", separator = ";", unknown = true) {
            value;
        }
        $for(item in items; trailing = true) {
            value;
        }
        $for(item in items; separator = let, trailing = match) {
            value;
        }
        $for(item in items; separator = let, separator = ",") {
            value;
        }
        $for(item in items; unknown = let) {
            value;
        }
    });
}
