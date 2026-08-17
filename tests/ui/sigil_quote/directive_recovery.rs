use sigil_stitch::prelude::*;

fn main() {
    let _ = sigil_quote!(TypeScript {
        $if(let) { value; } $for(item + 1 in items) {
            const nested = $N(struct);
        }
        const inline = [$for(entry + 1 in entries) { $N(enum) }];
        const inline_if = [$if(let) { value } $N(struct)];
    });
}
