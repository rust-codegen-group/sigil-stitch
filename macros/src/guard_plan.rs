use crate::ir::{FormattedCode, ParsedInput, QuoteArg, Statement};

pub(crate) const MAX_GUARDED_BINDING_DEPTH: usize = 128;

pub(crate) fn validate(input: &ParsedInput) -> syn::Result<()> {
    validate_sequence(&input.statements, 0)
}

fn validate_sequence(statements: &[Statement], enclosing_depth: usize) -> syn::Result<()> {
    let mut depth = enclosing_depth;
    let mut guarded = false;

    for statement in statements {
        if guarded && let Statement::MetaLet { marker_span, .. } = statement {
            depth += 1;
            if depth > MAX_GUARDED_BINDING_DEPTH {
                return Err(syn::Error::new(
                    *marker_span,
                    format!(
                        "sigil_quote! supports at most {MAX_GUARDED_BINDING_DEPTH} guarded \
                         $let continuations; split this macro into smaller sigil_quote! blocks"
                    ),
                ));
            }
            guarded = false;
        }

        validate_statement(statement, depth)?;
        guarded |= statement_is_fallible(statement);
    }

    Ok(())
}

fn validate_statement(statement: &Statement, enclosing_depth: usize) -> syn::Result<()> {
    match statement {
        Statement::Terminated(formatted) | Statement::Line(formatted) => {
            validate_formatted(formatted, enclosing_depth)
        }
        Statement::ControlFlow { branches, .. } => {
            for branch in branches {
                validate_formatted(&branch.condition, enclosing_depth)?;
                validate_sequence(&branch.body, enclosing_depth)?;
            }
            Ok(())
        }
        Statement::MetaIf(meta_if) => {
            validate_sequence(&meta_if.first.body, enclosing_depth)?;
            for branch in &meta_if.else_if {
                validate_sequence(&branch.body, enclosing_depth)?;
            }
            if let Some(body) = &meta_if.otherwise {
                validate_sequence(body, enclosing_depth)?;
            }
            Ok(())
        }
        Statement::MetaFor { body, .. } => validate_sequence(body, enclosing_depth),
        Statement::InlineFor { body, .. } => validate_formatted(body, enclosing_depth),
        Statement::ParenBlock { header, body } => {
            validate_formatted(header, enclosing_depth)?;
            validate_sequence(body, enclosing_depth)
        }
        Statement::BlankLine
        | Statement::Comment(_)
        | Statement::Attr(_)
        | Statement::Indent
        | Statement::Dedent
        | Statement::SpliceEach { .. }
        | Statement::MetaLet { .. } => Ok(()),
    }
}

fn validate_formatted(formatted: &FormattedCode, enclosing_depth: usize) -> syn::Result<()> {
    for arg in formatted.args() {
        if let QuoteArg::ParsedBlock(statements) | QuoteArg::ParsedSplice(statements) = arg {
            validate_sequence(statements, enclosing_depth)?;
        }
    }
    Ok(())
}

pub(crate) fn argument_is_fallible(arg: &QuoteArg) -> bool {
    matches!(
        arg,
        QuoteArg::TypeJoin { .. } | QuoteArg::ParsedBlock(_) | QuoteArg::ParsedSplice(_)
    )
}

pub(crate) fn statement_is_fallible(statement: &Statement) -> bool {
    match statement {
        Statement::Terminated(formatted) | Statement::Line(formatted) => {
            formatted.args().iter().any(argument_is_fallible)
        }
        Statement::ControlFlow { branches, .. } => branches.iter().any(|branch| {
            branch.condition.args().iter().any(argument_is_fallible)
                || branch.body.iter().any(statement_is_fallible)
        }),
        Statement::MetaIf(meta_if) => {
            meta_if.first.body.iter().any(statement_is_fallible)
                || meta_if
                    .else_if
                    .iter()
                    .any(|branch| branch.body.iter().any(statement_is_fallible))
                || meta_if
                    .otherwise
                    .as_ref()
                    .is_some_and(|body| body.iter().any(statement_is_fallible))
        }
        Statement::MetaFor { body, .. } => body.iter().any(statement_is_fallible),
        Statement::InlineFor { body, .. } => body.args().iter().any(argument_is_fallible),
        Statement::ParenBlock { header, body } => {
            header.args().iter().any(argument_is_fallible) || body.iter().any(statement_is_fallible)
        }
        Statement::BlankLine
        | Statement::Comment(_)
        | Statement::Attr(_)
        | Statement::Indent
        | Statement::Dedent
        | Statement::SpliceEach { .. }
        | Statement::MetaLet { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;

    use super::*;
    use crate::ir::FormattedCode;

    fn fallible_statement() -> Statement {
        let mut formatted = FormattedCode::new();
        formatted.push_argument(QuoteArg::TypeJoin {
            separator: syn::parse_quote!(" | "),
            iter: syn::parse_quote!(::std::iter::empty::<&str>()),
        });
        Statement::Line(formatted)
    }

    fn binding() -> Statement {
        let block: syn::Block = syn::parse_quote!({
            let value = "value";
        });
        let syn::Stmt::Local(local) = block.stmts.into_iter().next().unwrap() else {
            unreachable!("the test fixture contains one local binding");
        };
        Statement::MetaLet {
            local,
            marker_span: Span::call_site(),
        }
    }

    fn alternating_sequence(depth: usize) -> ParsedInput {
        let mut statements = Vec::with_capacity(depth * 2);
        for _ in 0..depth {
            statements.push(fallible_statement());
            statements.push(binding());
        }
        ParsedInput { statements }
    }

    #[test]
    fn accepts_the_guarded_binding_depth_limit() {
        validate(&alternating_sequence(MAX_GUARDED_BINDING_DEPTH)).unwrap();
    }

    #[test]
    fn rejects_guarded_binding_depth_above_the_limit() {
        let error = validate(&alternating_sequence(MAX_GUARDED_BINDING_DEPTH + 1)).unwrap_err();

        assert!(error.to_string().contains("at most 128 guarded $let"));
    }

    #[test]
    fn counts_guarded_bindings_across_nested_generated_blocks() {
        let mut nested = alternating_sequence(MAX_GUARDED_BINDING_DEPTH).statements;
        nested.push(Statement::Line({
            let mut formatted = FormattedCode::new();
            formatted.push_argument(QuoteArg::ParsedBlock(vec![fallible_statement(), binding()]));
            formatted
        }));

        let error = validate(&ParsedInput { statements: nested }).unwrap_err();
        assert!(error.to_string().contains("at most 128 guarded $let"));
    }
}
