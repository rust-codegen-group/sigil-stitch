use sigil_stitch::prelude::*;

struct NotDisplay;

mod values {
    pub struct PrivateConversion;

    impl PrivateConversion {
        #[allow(dead_code)]
        fn to_string(&self) -> String {
            String::new()
        }
    }
}

use values::PrivateConversion;

fn main() {
    let _ = sigil_quote!(TypeScript {
        const quoted = $S(PrivateConversion);
        const $N(PrivateConversion) = 1;
        const verbatim = $V(PrivateConversion);
        const embedded = $V("prefix @{NotDisplay}");
    });
}
