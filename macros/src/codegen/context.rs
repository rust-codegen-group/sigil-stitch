use proc_macro2::{Ident, Span};

pub(super) struct GenerateContext {
    next_ident: usize,
}

impl GenerateContext {
    pub(super) fn new() -> Self {
        Self { next_ident: 0 }
    }

    pub(super) fn ident(&mut self, role: &str) -> Ident {
        let next = self.next_ident;
        self.next_ident += 1;
        Ident::new(&format!("__sigil_{role}_{next}"), Span::mixed_site())
    }
}
