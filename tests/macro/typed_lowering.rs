use std::cell::{Cell, RefCell};
use std::fmt;

use super::helpers::*;

struct LoggedItem<'a> {
    value: &'a str,
    events: &'a RefCell<Vec<&'static str>>,
}

impl fmt::Display for LoggedItem<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.events.borrow_mut().push("item");
        formatter.write_str(self.value)
    }
}

#[test]
fn direct_raw_literal_interpolation_uses_typed_rust_expressions() {
    let name = "Ada";
    let block = sigil_quote!(TypeScript {
        const greeting = $V(r#"Hello @{name.to_uppercase()}"#);
    })
    .unwrap();

    assert!(render_ts(&block).contains("Hello ADA"));
}

#[test]
fn literal_interpolation_ignores_braces_inside_rust_syntax() {
    let value = 7;
    let block = sigil_quote!(TypeScript {
        const rendered = $V(r#"@{format!("}} {}", { let nested = value; nested })}"#);
    })
    .unwrap();

    assert!(render_ts(&block).contains("} 7"));
}

#[test]
fn dynamic_string_expressions_are_not_reparsed() {
    let dynamic = String::from("literal @{not_rust}");
    let block = sigil_quote!(TypeScript {
        const value = $V(dynamic);
    })
    .unwrap();

    assert!(render_ts(&block).contains("literal @{not_rust}"));
}

#[test]
fn mixed_type_join_preserves_a_borrowed_temporary() {
    let types = [TypeName::primitive("A"), TypeName::primitive("B")];
    let block = sigil_quote!(TypeScript {
        type Value = $L(String::from("Prefix").as_str()) | $T_join(" | ", &types);
    })
    .unwrap();

    assert!(render_ts(&block).contains("Prefix | A | B"));
}

#[test]
fn mixed_parsed_splice_preserves_a_borrowed_temporary() {
    let block = sigil_quote!(TypeScript {
        const value = $L(String::from("prefix").as_str()) $if(true) { + suffix };
    })
    .unwrap();

    assert!(render_ts(&block).contains("prefix + suffix"));
}

#[test]
fn mixed_fallible_arguments_drop_temporaries_before_later_statements() {
    let text = RefCell::new(String::from("Prefix"));
    let types = [TypeName::primitive("A"), TypeName::primitive("B")];
    let block = sigil_quote!(TypeScript {
        type Value = $L(text.borrow().as_str()) | $T_join(" | ", &types);
        const after = $L({
            text.borrow_mut().push_str(" updated");
            "done"
        });
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("Prefix | A | B"), "{output}");
    assert!(output.contains("const after = done"), "{output}");
    assert_eq!(&*text.borrow(), "Prefix updated");
}

#[test]
fn nested_helper_error_is_returned_and_stops_later_expressions() {
    let later_ran = Cell::new(false);
    let result = sigil_quote!(TypeScript {
        const invalid = {
            $let(_local = 1);
            $>
        };
        $let(later = {
            later_ran.set(true);
            "later"
        });
        const later = $L(later);
    });

    assert!(result.is_err());
    assert!(!later_ran.get());
}

#[test]
fn let_binding_after_a_successful_helper_stays_in_scope() {
    let types = [TypeName::primitive("A"), TypeName::primitive("B")];
    let block = sigil_quote!(TypeScript {
        type Value = $T_join(" | ", &types);
        $let(suffix = "kept");
        const after = $L(suffix);
    })
    .unwrap();

    let output = render_ts(&block);
    assert!(output.contains("type Value = A | B"), "{output}");
    assert!(output.contains("const after = kept"), "{output}");
}

#[test]
fn guarded_binding_depth_limit_compiles() {
    macro_rules! quote_guarded_bindings {
        ($dollar:tt, $types:ident; $($binding:ident),+ $(,)?) => {
            sigil_quote!(TypeScript {
                $(
                    type Value = $dollar T_join(" | ", &$types);
                    $dollar let($binding = stringify!($binding));
                    const binding = $dollar L($binding);
                )+
            })
        };
    }

    let types = [TypeName::primitive("A"), TypeName::primitive("B")];
    let block = quote_guarded_bindings!(
        $, types;
        guard_0, guard_1, guard_2, guard_3, guard_4, guard_5, guard_6, guard_7, guard_8, guard_9, guard_10, guard_11, guard_12, guard_13, guard_14, guard_15, guard_16, guard_17, guard_18, guard_19, guard_20, guard_21, guard_22, guard_23, guard_24, guard_25, guard_26, guard_27, guard_28, guard_29, guard_30, guard_31, guard_32, guard_33, guard_34, guard_35, guard_36, guard_37, guard_38, guard_39, guard_40, guard_41, guard_42, guard_43, guard_44, guard_45, guard_46, guard_47, guard_48, guard_49, guard_50, guard_51, guard_52, guard_53, guard_54, guard_55, guard_56, guard_57, guard_58, guard_59, guard_60, guard_61, guard_62, guard_63, guard_64, guard_65, guard_66, guard_67, guard_68, guard_69, guard_70, guard_71, guard_72, guard_73, guard_74, guard_75, guard_76, guard_77, guard_78, guard_79, guard_80, guard_81, guard_82, guard_83, guard_84, guard_85, guard_86, guard_87, guard_88, guard_89, guard_90, guard_91, guard_92, guard_93, guard_94, guard_95, guard_96, guard_97, guard_98, guard_99, guard_100, guard_101, guard_102, guard_103, guard_104, guard_105, guard_106, guard_107, guard_108, guard_109, guard_110, guard_111, guard_112, guard_113, guard_114, guard_115, guard_116, guard_117, guard_118, guard_119, guard_120, guard_121, guard_122, guard_123, guard_124, guard_125, guard_126, guard_127
    )
    .unwrap();

    let output = render_ts(&block);
    assert_eq!(output.matches("A | B").count(), 128, "{output}");
}

#[test]
fn let_binding_stays_in_scope_across_a_successful_helper() {
    let result = sigil_quote!(TypeScript {
        $let(prefix = "kept");
        const nested = {
            $let(inner = prefix);
            value: $L(inner);
        };
        const after = $L(prefix);
    });

    let output = render_ts(&result.unwrap());
    assert!(output.contains("value: kept"), "{output}");
    assert!(output.contains("const after = kept"), "{output}");
}

#[test]
fn join_preserves_iterator_item_and_separator_order() {
    let events = RefCell::new(Vec::new());
    let block = sigil_quote!(TypeScript {
        const values = [$join(
            { events.borrow_mut().push("separator"); ", " },
            {
                events.borrow_mut().push("iterator");
                vec![
                    LoggedItem { value: "a", events: &events },
                    LoggedItem { value: "b", events: &events },
                ]
            }
        )];
    })
    .unwrap();

    assert_eq!(
        events.into_inner(),
        ["iterator", "item", "item", "separator"]
    );
    assert!(render_ts(&block).contains("a, b"));
}

#[test]
fn join_evaluates_separator_once_for_empty_input() {
    let events = RefCell::new(Vec::new());
    let block = sigil_quote!(TypeScript {
        const values = [$join(
            { events.borrow_mut().push("separator"); ", " },
            {
                events.borrow_mut().push("iterator");
                Vec::<String>::new()
            }
        )];
    })
    .unwrap();

    assert_eq!(events.into_inner(), ["iterator", "separator"]);
    assert!(render_ts(&block).contains("[]"));
}

#[test]
fn join_evaluates_separator_once_for_single_input() {
    let events = RefCell::new(Vec::new());
    let block = sigil_quote!(TypeScript {
        const values = [$join(
            { events.borrow_mut().push("separator"); ", " },
            {
                events.borrow_mut().push("iterator");
                [LoggedItem { value: "a", events: &events }]
            }
        )];
    })
    .unwrap();

    assert_eq!(events.into_inner(), ["iterator", "item", "separator"]);
    assert!(render_ts(&block).contains("[a]"));
}

#[test]
fn type_join_evaluates_separator_only_between_items() {
    let events = RefCell::new(Vec::new());
    let block = sigil_quote!(TypeScript {
        type Value = $T_join(
            { events.borrow_mut().push("separator"); " | " },
            {
                events.borrow_mut().push("iterator");
                vec![TypeName::primitive("A"), TypeName::primitive("B")]
            }
        );
    })
    .unwrap();

    assert_eq!(events.into_inner(), ["iterator", "separator"]);
    assert!(render_ts(&block).contains("A | B"));
}

#[test]
fn type_join_skips_separator_for_empty_and_single_inputs() {
    for types in [Vec::new(), vec![TypeName::primitive("A")]] {
        let separator_calls = Cell::new(0);
        let block = sigil_quote!(TypeScript {
            type Value = $T_join(
                { separator_calls.set(separator_calls.get() + 1); " | " },
                types
            );
        })
        .unwrap();

        assert_eq!(separator_calls.get(), 0);
        let _ = render_ts(&block);
    }
}

#[test]
fn statement_for_preserves_separator_and_trailing_order() {
    let events = RefCell::new(Vec::new());
    let items = [1, 2];
    let block = sigil_quote!(TypeScript {
        $for(
            item in { events.borrow_mut().push("iterator"); &items };
            separator = { events.borrow_mut().push("separator"); "," },
            trailing = { events.borrow_mut().push("trailing"); true }
        ) {
            const value = $L({ events.borrow_mut().push("item"); item.to_string() });
        }
    })
    .unwrap();

    assert_eq!(
        events.into_inner(),
        [
            "iterator",
            "item",
            "separator",
            "item",
            "trailing",
            "separator"
        ]
    );
    assert!(render_ts(&block).contains("const value = 1"));
}

#[test]
fn empty_statement_for_skips_body_separator_and_trailing() {
    let events = RefCell::new(Vec::new());
    let items: [i32; 0] = [];
    let block = sigil_quote!(TypeScript {
        $for(
            item in { events.borrow_mut().push("iterator"); &items };
            separator = { events.borrow_mut().push("separator"); "," },
            trailing = { events.borrow_mut().push("trailing"); true }
        ) {
            const value = $L({ events.borrow_mut().push("item"); item.to_string() });
        }
    })
    .unwrap();

    assert_eq!(events.into_inner(), ["iterator"]);
    assert!(!render_ts(&block).contains("const value"));
}

#[test]
fn inline_for_preserves_separator_and_trailing_order() {
    let events = RefCell::new(Vec::new());
    let items = [1, 2];
    let block = sigil_quote!(TypeScript {
        const values = [$for(
            item in { events.borrow_mut().push("iterator"); &items };
            separator = { events.borrow_mut().push("separator"); "," },
            trailing = { events.borrow_mut().push("trailing"); true }
        ) { $L({ events.borrow_mut().push("item"); item.to_string() }) }];
    })
    .unwrap();

    assert_eq!(
        events.into_inner(),
        [
            "iterator",
            "item",
            "separator",
            "item",
            "trailing",
            "separator"
        ]
    );
    let output = render_ts(&block);
    assert!(output.contains("[1,2,]"), "{output}");
}

#[test]
fn single_inline_for_checks_trailing_without_emitting_separator() {
    let events = RefCell::new(Vec::new());
    let items = [1];
    let block = sigil_quote!(TypeScript {
        const values = [$for(
            item in { events.borrow_mut().push("iterator"); &items };
            separator = { events.borrow_mut().push("separator"); "," },
            trailing = { events.borrow_mut().push("trailing"); false }
        ) { $L({ events.borrow_mut().push("item"); item.to_string() }) }];
    })
    .unwrap();

    assert_eq!(events.into_inner(), ["iterator", "item", "trailing"]);
    assert!(render_ts(&block).contains("[1]"));
}

#[test]
fn failing_helper_stops_a_generated_statement_loop() {
    let visits = Cell::new(0);
    let later_visits = Cell::new(0);
    let separator_visits = Cell::new(0);
    let trailing_visits = Cell::new(0);
    let result = sigil_quote!(TypeScript {
        $for(
            item in ["a", "b"];
            separator = { separator_visits.set(separator_visits.get() + 1); "," },
            trailing = { trailing_visits.set(trailing_visits.get() + 1); true }
        ) {
            const invalid = {
                $let(_seen = { visits.set(visits.get() + 1); item });
                $>
            };
            const later = $L({ later_visits.set(later_visits.get() + 1); item });
        }
    });

    assert!(result.is_err());
    assert_eq!(visits.get(), 1);
    assert_eq!(later_visits.get(), 0);
    assert_eq!(separator_visits.get(), 0);
    assert_eq!(trailing_visits.get(), 0);
}

#[test]
fn failing_helper_stops_a_generated_inline_loop() {
    let visits = Cell::new(0);
    let result = sigil_quote!(TypeScript {
        const values = [$for(item in ["a", "b"]) {
            $if(true) {
                const invalid = {
                    $let(_seen = { visits.set(visits.get() + 1); item });
                    $>
                };
            }
        }];
    });

    assert!(result.is_err());
    assert_eq!(visits.get(), 1);
}

fn caller_return_after_helper(early: bool) -> &'static str {
    let _block = sigil_quote!(TypeScript {
        const nested = {
            $let(inner = "ready");
            value: $L(inner);
        };
        const current = $L(if early { return "returned" } else { "kept" });
    })
    .unwrap();

    "completed"
}

#[test]
fn caller_return_keeps_its_target_after_a_helper() {
    assert_eq!(caller_return_after_helper(true), "returned");
    assert_eq!(caller_return_after_helper(false), "completed");
}

#[test]
fn caller_break_keeps_its_target_after_a_helper() {
    let mut seen = Vec::new();
    for value in [1, 2, 3] {
        let block = sigil_quote!(TypeScript {
            const nested = {
                $let(inner = "ready");
                value: $L(inner);
            };
            const current = $L(if value == 2 { break } else { value.to_string() });
        })
        .unwrap();
        seen.push(render_ts(&block));
    }

    assert_eq!(seen.len(), 1);
    assert!(seen[0].contains("current = 1"));
}

#[test]
fn caller_continue_keeps_its_target_after_a_helper() {
    let mut seen = Vec::new();
    for value in [1, 2, 3] {
        let block = sigil_quote!(TypeScript {
            const nested = {
                $let(inner = value.to_string());
                value: $L(inner);
            };
            const current = $L(if value == 2 { continue } else { value.to_string() });
        })
        .unwrap();
        seen.push(render_ts(&block));
    }

    assert_eq!(seen.len(), 2);
    assert!(seen[0].contains("current = 1"));
    assert!(seen[1].contains("current = 3"));
}

fn caller_question_mark_after_helper(value: Option<&str>) -> Option<String> {
    let block = sigil_quote!(TypeScript {
        const nested = {
            $let(inner = "ready");
            value: $L(inner);
        };
        const current = $L(value?);
    })
    .ok()?;

    Some(render_ts(&block))
}

#[test]
fn caller_question_mark_keeps_its_target_after_a_helper() {
    assert!(
        caller_question_mark_after_helper(Some("ready"))
            .unwrap()
            .contains("current = ready")
    );
    assert_eq!(caller_question_mark_after_helper(None), None);
}

#[test]
fn generated_names_do_not_capture_caller_bindings() {
    let __sigil_builder_0 = "builder";
    let __sigil_helper_error_1 = "error";
    let __sigil_arg_2 = "argument";
    let __sigil_for_emitted_3 = "loop";
    let block = sigil_quote!(TypeScript {
        const values = [$join(
            ",",
            [
                __sigil_builder_0,
                __sigil_helper_error_1,
                __sigil_arg_2,
                __sigil_for_emitted_3,
            ]
        )];
    })
    .unwrap();

    assert!(
        render_ts(&block).contains("builder,error,argument,loop"),
        "generated identifiers captured a caller binding"
    );
}
