use proc_macro2::TokenTree;

pub(super) fn is_semicolon(tt: &TokenTree) -> bool {
    matches!(tt, TokenTree::Punct(p) if p.as_char() == ';')
}

pub(super) fn is_ident(tt: &TokenTree, name: &str) -> bool {
    matches!(tt, TokenTree::Ident(id) if *id == name)
}

pub(super) fn unescape_string(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('0') => out.push('\0'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => {
                    return Err(format!("unknown escape sequence: \\{other}"));
                }
                None => {
                    return Err("unexpected end of string after \\".to_string());
                }
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}
