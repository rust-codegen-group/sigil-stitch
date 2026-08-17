use crate::ir::Statement;

use super::MacroLang;

/// Post-parse rewrite pass that injects Indent/Dedent around language-specific
/// structural patterns (guards, let-body continuations).
pub(super) fn rewrite_statements(stmts: Vec<Statement>, lang: MacroLang) -> Vec<Statement> {
    match lang {
        MacroLang::Haskell => rewrite_haskell(stmts),
        MacroLang::OCaml => rewrite_ocaml(stmts),
        _ => stmts,
    }
}

fn is_guard_format(format: &str) -> bool {
    format.starts_with("| ") || format == "|"
}

fn is_guard(stmt: &Statement) -> bool {
    match stmt {
        Statement::Line(formatted) | Statement::Terminated(formatted) => {
            is_guard_format(formatted.format())
        }
        _ => false,
    }
}

/// Haskell: indent guard lines (`| cond = expr`) under the preceding function header.
fn rewrite_haskell(stmts: Vec<Statement>) -> Vec<Statement> {
    let mut out = Vec::with_capacity(stmts.len() + 4);
    let mut iter = stmts.into_iter().peekable();

    while let Some(stmt) = iter.next() {
        if !is_guard(&stmt) {
            out.push(stmt);
            if iter.peek().is_some_and(is_guard) {
                out.push(Statement::Indent);
                while iter.peek().is_some_and(is_guard) {
                    out.push(iter.next().unwrap());
                }
                out.push(Statement::Dedent);
            }
        } else {
            // Guard at start of sequence (no header) — still indent
            out.push(Statement::Indent);
            out.push(stmt);
            while iter.peek().is_some_and(is_guard) {
                out.push(iter.next().unwrap());
            }
            out.push(Statement::Dedent);
        }
    }

    out
}

fn is_let_opener(stmt: &Statement) -> bool {
    match stmt {
        Statement::Line(formatted) => formatted.format().ends_with(" ="),
        _ => false,
    }
}

fn is_content_stmt(stmt: &Statement) -> bool {
    matches!(stmt, Statement::Line(_) | Statement::Terminated(_))
}

/// OCaml: indent continuation lines under a `let ... =` opener.
fn rewrite_ocaml(stmts: Vec<Statement>) -> Vec<Statement> {
    let mut out = Vec::with_capacity(stmts.len() + 4);
    let mut iter = stmts.into_iter().peekable();

    while let Some(stmt) = iter.next() {
        out.push(stmt);
        if is_let_opener(out.last().unwrap()) && iter.peek().is_some_and(is_content_stmt) {
            out.push(Statement::Indent);
            while iter.peek().is_some_and(is_content_stmt) {
                out.push(iter.next().unwrap());
            }
            out.push(Statement::Dedent);
        }
    }

    out
}
