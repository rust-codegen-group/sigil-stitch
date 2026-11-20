//! Width-aware layout for the Python expressions that span more than one
//! operand. Operate on quote tokens, not rendered Python source.

use proc_macro2::{Spacing, TokenTree};

use super::{
    spacing::{MacroSpacingPolicy, comprehension_for_start},
    util::is_ident,
};
use crate::ir::{
    DelimitedLayout, FormattedCode, LayoutEdgesPlan, LayoutForceBreak, LayoutPlan, NoArgMarker,
    TrailingCommaPolicy,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Context {
    Statement,
    CallArguments,
    Parenthesized,
    Comprehension,
    Other,
}

pub(super) fn format_expression(
    tokens: &[TokenTree],
    policy: MacroSpacingPolicy,
    mut format: impl FnMut(&[TokenTree]) -> syn::Result<FormattedCode>,
) -> Option<syn::Result<FormattedCode>> {
    match policy.python_expression_context()? {
        Context::Statement => {
            let first = tokens.first()?;
            let header = ["if", "elif", "while"]
                .iter()
                .any(|word| is_ident(first, word));
            if header {
                let condition = &tokens[1..];
                let mut operators = positions(condition, &["and", "or"]);
                // With more than one Boolean operator, break the chain as a
                // unit. Leave a single `test and all(...)` implicit. For an
                // identity test, prefer a comparison break to one just before
                // the inner call's values.
                if operators.is_empty() {
                    operators = positions(condition, &["is"]);
                } else if operators.len() == 1 {
                    operators.clear();
                }
                if !operators.is_empty()
                    && positions(condition, &["if", "else", "for", "lambda"]).is_empty()
                    && !condition.iter().any(|token| punct(token, ','))
                {
                    return Some((|| {
                        let mut out = format(&tokens[..1])?;
                        out.format_mut().push(' ');
                        let child = break_before(condition, &operators, &mut format)?;
                        out.push_layout(child, expr_layout(true));
                        Ok(out)
                    })());
                }
                return None;
            }
            format_conditional_statement(tokens, first, &mut format)
        }
        Context::CallArguments => {
            // A conditional expression groups independently of the argument
            // list. Breaking the call must not split every conditional.
            if !positions(tokens, &["lambda"]).is_empty() {
                return None;
            }
            let args = tokens.split(|token| punct(token, ',')).collect::<Vec<_>>();
            if !args.iter().any(|arg| conditional_branches(arg).is_some()) {
                return None;
            }
            Some((|| {
                let mut out = FormattedCode::new();
                for (index, argument) in args.iter().enumerate() {
                    if index != 0 {
                        out.format_mut().push(',');
                        if !argument.is_empty() {
                            out.push_marker(NoArgMarker::SoftBreak);
                        }
                    }
                    if let Some(branches) = conditional_branches(argument) {
                        let child = break_before(argument, &branches, &mut format)?;
                        out.push_layout(child, expr_layout(false));
                    } else {
                        out.append(format(argument)?);
                    }
                }
                Ok(out)
            })())
        }
        Context::Parenthesized => {
            let breaks =
                conditional_branches(tokens).unwrap_or_else(|| positions(tokens, &["and", "or"]));
            (!breaks.is_empty()).then(|| break_before(tokens, &breaks, &mut format))
        }
        Context::Comprehension => {
            let for_pos = positions(tokens, &["for"]).first().copied()?;
            let clause_start = comprehension_for_start(tokens, for_pos);
            let element = &tokens[..clause_start];
            let breaks =
                conditional_branches(element).unwrap_or_else(|| positions(element, &["and", "or"]));
            if breaks.is_empty() {
                return None;
            }
            Some((|| {
                let mut out = break_before(element, &breaks, &mut format)?;
                out.append(format(&tokens[clause_start..])?);
                Ok(out)
            })())
        }
        Context::Other => None,
    }
}

fn format_conditional_statement(
    tokens: &[TokenTree],
    first: &TokenTree,
    format: &mut impl FnMut(&[TokenTree]) -> syn::Result<FormattedCode>,
) -> Option<syn::Result<FormattedCode>> {
    // A raise cause is a separate expression outside the exception's parentheses.
    let from = is_ident(first, "raise")
        .then(|| positions(tokens, &["from"]).into_iter().next())
        .flatten();
    let (statement, cause) = match from {
        Some(index) => (&tokens[..index], Some(&tokens[index + 1..])),
        None => (tokens, None),
    };
    let mut breaks = conditional_branches(statement);
    let cause_breaks = cause.and_then(conditional_branches);
    if breaks.is_none() && cause_breaks.is_none() {
        return None;
    }

    // `await` is an expression operator: it belongs to its branch, not to the statement prefix.
    let start = if is_ident(first, "yield")
        && statement
            .get(1)
            .is_some_and(|token| is_ident(token, "from"))
    {
        2
    } else if ["assert", "raise", "return", "yield"]
        .iter()
        .any(|keyword| is_ident(first, keyword))
    {
        1
    } else {
        (0..breaks.as_ref()?[0])
            .rposition(|index| is_assignment(statement, index))
            .map_or(0, |index| index + 1)
    };
    if let Some(offsets) = &mut breaks {
        if offsets[0] <= start {
            return None;
        }
        for offset in offsets {
            *offset -= start;
        }
    }

    Some((|| {
        let mut out = format(&statement[..start])?;
        if start == 0 {
            // Preserve the statement column before a layout whose first token may be `(`.
            out.format_mut().push_str("%[");
        } else {
            out.format_mut().push(' ');
        }
        append_conditional_operand(&mut out, &statement[start..], breaks.as_deref(), format)?;
        if let Some(cause) = cause {
            out.format_mut().push_str(" from ");
            append_conditional_operand(&mut out, cause, cause_breaks.as_deref(), format)?;
        }
        Ok(out)
    })())
}

fn append_conditional_operand(
    out: &mut FormattedCode,
    tokens: &[TokenTree],
    offsets: Option<&[usize]>,
    format: &mut impl FnMut(&[TokenTree]) -> syn::Result<FormattedCode>,
) -> syn::Result<()> {
    if let Some(offsets) = offsets {
        out.push_layout(break_before(tokens, offsets, format)?, expr_layout(true));
    } else {
        out.append(format(tokens)?);
    }
    Ok(())
}

fn conditional_branches(tokens: &[TokenTree]) -> Option<Vec<usize>> {
    if !positions(tokens, &["for", "lambda"]).is_empty()
        || tokens.iter().any(|token| punct(token, ','))
    {
        return None;
    }
    let branches = positions(tokens, &["if", "else"]);
    if branches.len() < 2 || !branches.len().is_multiple_of(2) || branches[0] == 0 {
        return None;
    }
    if !branches.iter().enumerate().all(|(index, &position)| {
        let expected = if index % 2 == 0 { "if" } else { "else" };
        is_ident(&tokens[position], expected)
    }) {
        return None;
    }
    Some(branches)
}

fn positions(tokens: &[TokenTree], words: &[&str]) -> Vec<usize> {
    tokens
        .iter()
        .enumerate()
        .filter_map(|(i, token)| {
            if i != 0 && punct(&tokens[i - 1], '$') {
                return None;
            }
            words.iter().any(|word| is_ident(token, word)).then_some(i)
        })
        .collect()
}

fn punct(token: &TokenTree, value: char) -> bool {
    matches!(token, TokenTree::Punct(p) if p.as_char() == value)
}

fn is_assignment(tokens: &[TokenTree], index: usize) -> bool {
    let Some(TokenTree::Punct(equals)) = tokens.get(index) else {
        return false;
    };
    if index == 0
        || equals.as_char() != '='
        || (equals.spacing() == Spacing::Joint
            && tokens.get(index + 1).is_some_and(|next| punct(next, '=')))
    {
        return false;
    }

    let mut prefix = tokens[..index]
        .iter()
        .rev()
        .map_while(|token| match token {
            TokenTree::Punct(p) if p.spacing() == Spacing::Joint => Some(p.as_char()),
            _ => None,
        })
        .fuse();
    match (prefix.next(), prefix.next(), prefix.next()) {
        (None | Some('+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '@'), None, None) => true,
        (Some(operator @ ('<' | '>' | '*' | '/')), Some(previous), None) => operator == previous,
        _ => false,
    }
}

fn break_before(
    tokens: &[TokenTree],
    offsets: &[usize],
    format: &mut impl FnMut(&[TokenTree]) -> syn::Result<FormattedCode>,
) -> syn::Result<FormattedCode> {
    let mut out = FormattedCode::new();
    let mut start = 0;
    for &end in offsets {
        out.append(format(&tokens[start..end])?);
        out.push_marker(NoArgMarker::SoftBreak);
        start = end;
    }
    out.append(format(&tokens[start..])?);
    Ok(out)
}

fn expr_layout(continuation_parens: bool) -> LayoutPlan {
    if !continuation_parens {
        LayoutPlan::Inline
    } else {
        LayoutPlan::Delimited(DelimitedLayout {
            edges: LayoutEdgesPlan::WithBreaks,
            trailing_comma: TrailingCommaPolicy::Never,
            force_break: LayoutForceBreak::Automatic,
            delimited_loop: None,
            flat_prefix: String::new(),
            flat_suffix: String::new(),
            broken_prefix: "(".into(),
            broken_suffix: ")".into(),
        })
    }
}
