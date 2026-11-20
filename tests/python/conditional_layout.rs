use sigil_stitch::prelude::*;

fn render(block: CodeBlock, width: usize) -> String {
    FileSpec::builder("conditional.py")
        .add_code(block)
        .build()
        .unwrap()
        .render(width)
        .unwrap()
}

#[test]
fn yield_from_stays_outside_conditional_parentheses() {
    let block = sigil_quote!(Python {
        def iterate(): {
            yield from first_iterable if use_first else second_iterable
        }
    })
    .unwrap();
    assert_eq!(
        render(block, 40),
        concat!(
            "def iterate():\n",
            "    yield from (\n",
            "        first_iterable\n",
            "        if use_first\n",
            "        else second_iterable\n",
            "    )\n",
        ),
    );
}

#[test]
fn raise_keeps_the_cause_outside_the_exception_expression() {
    let block = sigil_quote!(Python {
        def rethrow(): {
            raise first_error if should_use_first_error else second_error from original_cause
        }
    })
    .unwrap();
    assert_eq!(
        render(block, 40),
        concat!(
            "def rethrow():\n",
            "    raise (\n",
            "        first_error\n",
            "        if should_use_first_error\n",
            "        else second_error\n",
            "    ) from original_cause\n",
        ),
    );
}

#[test]
fn raise_formats_a_conditional_cause_separately() {
    let block = sigil_quote!(Python {
        def rethrow(): {
            raise first_error from primary_cause if should_use_primary_cause else fallback_cause
        }
    })
    .unwrap();
    assert_eq!(
        render(block, 40),
        concat!(
            "def rethrow():\n",
            "    raise first_error from (\n",
            "        primary_cause\n",
            "        if should_use_primary_cause\n",
            "        else fallback_cause\n",
            "    )\n",
        ),
    );
}

#[test]
fn await_stays_on_the_selected_branch() {
    let block = sigil_quote!(Python {
        async def evaluate(): {
            await first_awaitable_candidate if use_it else fallback_value
        }
    })
    .unwrap();
    assert_eq!(
        render(block, 40),
        concat!(
            "async def evaluate():\n",
            "    (\n",
            "        await first_awaitable_candidate\n",
            "        if use_it\n",
            "        else fallback_value\n",
            "    )\n",
        ),
    );
}

#[test]
fn conditional_preserves_comparisons_and_shift_assignment() {
    let block = sigil_quote!(Python {
        def compare(): {
            lower = first_operand <= second_operand if use_first else fallback_value
            upper = first_operand >= second_operand if use_first else fallback_value
            value <<= first_choice if use_first else fallback_value
        }
    })
    .unwrap();
    assert_eq!(
        render(block, 40),
        concat!(
            "def compare():\n",
            "    lower = (\n",
            "        first_operand <= second_operand\n",
            "        if use_first\n",
            "        else fallback_value\n",
            "    )\n",
            "    upper = (\n",
            "        first_operand >= second_operand\n",
            "        if use_first\n",
            "        else fallback_value\n",
            "    )\n",
            "    value <<= (\n",
            "        first_choice\n",
            "        if use_first\n",
            "        else fallback_value\n",
            "    )\n",
        ),
    );
}
