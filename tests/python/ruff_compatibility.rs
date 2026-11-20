use std::process::{Command, Stdio};
use std::sync::Once;

use sigil_stitch::lang::python::Python;
use sigil_stitch::prelude::*;

const WIDTHS: [usize; 4] = [40, 80, 88, 100];

fn quoted_file(filename: &str, block: CodeBlock) -> FileSpec {
    FileSpec::builder(filename).add_code(block).build().unwrap()
}

fn assert_ruff_file(file: &FileSpec, widths: &[usize]) {
    for &width in widths {
        assert_ruff_fixed_point(&file.render(width).unwrap(), file.filename(), width);
    }
}

fn ruff_command() -> Command {
    if std::env::var_os("SIGIL_STITCH_RUFF_UVX").is_some() {
        let mut command = Command::new("uvx");
        command.args(["--from", "ruff==0.17.5", "ruff"]);
        command
    } else {
        Command::new("ruff")
    }
}

fn assert_python_syntax(source: &str, filename: &str, width: usize) {
    let mut child = Command::new("python3")
        .args([
            "-c",
            "import sys; compile(sys.stdin.buffer.read(), sys.argv[1], 'exec')",
            filename,
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 syntax oracle must be installed");
    let mut stdin = child
        .stdin
        .take()
        .expect("python3 stdin should be available");
    std::io::Write::write_all(&mut stdin, source.as_bytes()).expect("write source to python3");
    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("wait for python3 syntax check");
    assert!(
        output.status.success(),
        "Python rejected {filename} at width {width}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_ruff_fixed_point(source: &str, filename: &str, width: usize) {
    assert_python_syntax(source, filename, width);

    static CHECK_VERSION: Once = Once::new();
    CHECK_VERSION.call_once(|| {
        let version = ruff_command()
            .arg("--version")
            .output()
            .expect("the pinned Ruff oracle must be installed");
        assert!(
            version.status.success(),
            "Ruff --version failed: {version:?}"
        );
        let version = String::from_utf8_lossy(&version.stdout);
        assert_eq!(
            version.trim(),
            "ruff 0.17.5",
            "unexpected Ruff oracle version"
        );
    });

    let mut command = ruff_command();
    let mut child = command
        .args([
            "format",
            "--check",
            "--diff",
            "--isolated",
            "--line-length",
            &width.to_string(),
            "--stdin-filename",
            filename,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the pinned Ruff oracle must be runnable");
    let mut stdin = child
        .stdin
        .take()
        .expect("Ruff stdin should be available for the fixed-point check");
    std::io::Write::write_all(&mut stdin, source.as_bytes()).expect("write source to Ruff");
    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("wait for Ruff fixed-point check");
    assert!(
        output.status.success(),
        "Ruff changed {filename} at width {width}:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn generated_python_is_ruff_fixed_point() {
    let generator = sigil_quote!(Python {
        if (_support.is_dict(value) and all((_support.number(_item, True)
            or isinstance(_item, str)) for _item in _support.as_dict(value).values())) {
            return value;
        }
    })
    .unwrap();
    let generator_function = FunSpec::builder("generated_generator")
        .body(generator)
        .build()
        .unwrap();
    let generator_file = FileSpec::builder_with("generator.py", Python::new())
        .add_function(generator_function)
        .build()
        .unwrap()
        .render(88)
        .unwrap();
    assert_ruff_fixed_point(&generator_file, "generator.py", 88);

    let async_comprehension = sigil_quote!(Python {
        async def collect(values): {
            return (item async for item in values)
        }
    })
    .unwrap();
    assert_ruff_file(
        &quoted_file("async_comprehension.py", async_comprehension),
        &[32, 40, 88],
    );

    let async_filtered_items = sigil_quote!(Python {
        async def collect(values): {
            return [item.is_eligible and item.has_matching_metadata async for item in values if item.enabled]
        }
    })
    .unwrap();
    assert_ruff_file(
        &quoted_file("async_filtered_items.py", async_filtered_items),
        &[40, 120],
    );

    let local_import = sigil_quote!(Python {
        def load(): {
            from . import sibling
            return sibling
        }
    })
    .unwrap();
    assert_ruff_file(&quoted_file("local_import.py", local_import), &[88]);

    let comprehension = sigil_quote!(Python {
        return {key: convert(item) for key, item in values.items()}
    })
    .unwrap();
    let comprehension_function = FunSpec::builder("generated_comprehension")
        .body(comprehension)
        .build()
        .unwrap();
    for width in [40, 80, 88, 100] {
        let comprehension_file = FileSpec::builder_with("comprehension.py", Python::new())
            .add_function(comprehension_function.clone())
            .build()
            .unwrap()
            .render(width)
            .unwrap();
        assert_ruff_fixed_point(&comprehension_file, "comprehension.py", width);
    }

    let initializer = sigil_quote!(Python {
        additional_properties: _dict[_str, dict[str, Choice] | str] = _dataclass_field(default_factory=lambda: _dict[_str, dict[str, Choice] | str](), kw_only=True, metadata={"openapi_additional_properties": True, "openapi_dictionary": True})
    })
    .unwrap();
    assert_ruff_file(&quoted_file("initializer.py", initializer), &[88]);

    let semantic_body = sigil_quote!(Python {
        return {key: convert(item) for key, item in values.items()}
    })
    .unwrap();
    let semantic_function = FunSpec::builder("convert_values")
        .add_param(ParameterSpec::new("values", TypeName::primitive("object")).unwrap())
        .returns(vec![TypeName::primitive("object")])
        .body(semantic_body)
        .build()
        .unwrap();
    let semantic_file = FileSpec::builder_with("semantic.py", Python::new())
        .add_function(semantic_function)
        .build()
        .unwrap()
        .render(88)
        .unwrap();
    assert_ruff_fixed_point(&semantic_file, "semantic.py", 88);

    let first = sigil_quote!(Python {
        def first(): {
            pass
        }
    })
    .unwrap();
    let second = sigil_quote!(Python {
        def second(): {
            pass
        }
    })
    .unwrap();
    let file = FileSpec::builder("declarations.py")
        .add_code(first)
        .add_code(second)
        .build()
        .unwrap()
        .render(88)
        .unwrap();
    assert_ruff_fixed_point(&file, "declarations.py", 88);

    let expression_cases = sigil_quote!(Python {
        assert first_choice if condition else second_choice
        value += first_choice if condition else second_choice
        first = second = first_choice if condition else second_choice
        value = first if second else third if fourth else fifth
        call(first_argument, second_argument,)
        tuple_value = (first_argument, second_argument,)
    })
    .unwrap();
    let expression_function = FunSpec::builder("expression_cases")
        .body(expression_cases)
        .build()
        .unwrap();
    for width in [40, 80, 88, 100] {
        let expression_file = FileSpec::builder_with("expressions.py", Python::new())
            .add_function(expression_function.clone())
            .build()
            .unwrap()
            .render(width)
            .unwrap();
        assert_ruff_fixed_point(&expression_file, "expressions.py", width);
    }

    let pragma = sigil_quote!(Python {
        value = decode(payload, encoding) $comment("type: ignore[assignment]  # démо")
    })
    .unwrap();
    let pragma_function = FunSpec::builder("pragma_case")
        .body(pragma)
        .build()
        .unwrap();
    let pragma_file = FileSpec::builder_with("pragma.pyi", Python::new().with_extension("pyi"))
        .add_function(pragma_function)
        .build()
        .unwrap()
        .render(36)
        .unwrap();
    assert_ruff_fixed_point(&pragma_file, "pragma.pyi", 36);
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn class_field_initializers_are_ruff_fixed_points() {
    let annotation = TypeName::primitive("VeryLongAnnotatedTypeName");
    let call = FieldSpec::builder("value", annotation.clone())
        .initializer(sigil_quote!(Python { make_value(first_arg, second_arg, third_arg) }).unwrap())
        .build()
        .unwrap();
    let pragma = FieldSpec::builder("other", annotation)
        .initializer(
            sigil_quote!(Python {
                make_value() $comment("type: ignore[assignment, attr-defined]")
            })
            .unwrap(),
        )
        .build()
        .unwrap();
    let model = TypeSpec::builder("Model", TypeKind::Class)
        .add_field(call)
        .add_field(pragma)
        .build()
        .unwrap();
    let file = FileSpec::builder("fields.py")
        .add_type(model)
        .build()
        .unwrap();
    assert_ruff_file(&file, &WIDTHS);
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn wrapped_imports_are_ruff_fixed_points() {
    let module = "package.submodule.with_a_long_name";
    let file = FileSpec::builder("wrapped_imports.py")
        .add_import(ImportSpec::named(module, "AlphaTypeNameWithLongPublicName"))
        .add_import(ImportSpec::named(module, "BetaTypeNameWithLongPublicName"))
        .add_import(ImportSpec::named(module, "GammaTypeNameWithLongPublicName"))
        .add_function(
            FunSpec::builder("run")
                .body(sigil_quote!(Python { pass }).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    assert_ruff_file(&file, &WIDTHS);
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn import_only_files_are_ruff_fixed_points() {
    let file = FileSpec::builder("imports_only.py")
        .add_import(ImportSpec::named("json", "dumps"))
        .build()
        .unwrap();
    assert_ruff_file(&file, &[88]);

    let file = FileSpec::builder("reexports.py")
        .add_import(ImportSpec::named_as(
            ".models",
            "PublicModel",
            "PublicModel",
        ))
        .build()
        .unwrap();
    assert!(
        file.render(88)
            .unwrap()
            .contains("PublicModel as PublicModel")
    );
    assert_ruff_file(&file, &WIDTHS);
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn long_structured_signatures_are_ruff_fixed_points() {
    let single = FunSpec::builder("constrained_inline_additional_property_item_from_dict")
        .add_param(ParameterSpec::of("value", TypeName::primitive("object")))
        .returns(vec![TypeName::primitive(
            "ConstrainedInlineAdditionalPropertyItem",
        )])
        .body(sigil_quote!(Python { return value }).unwrap())
        .build()
        .unwrap();
    let multi = FunSpec::builder("fetch_resource")
        .add_param(ParameterSpec::of(
            "source",
            TypeName::primitive("SourceWithANameThatTakesUpMoreThanHalfOfOneLine"),
        ))
        .add_param(ParameterSpec::of(
            "sink",
            TypeName::primitive("SinkWithANameThatTakesUpMoreThanHalfOfOneLine"),
        ))
        .returns(vec![TypeName::application(
            TypeName::importable(".runtime", "ApiResponse"),
            vec![TypeArgument::Single(TypeName::primitive(
                "ExtremelyLongReturnClassNameWhichUsesMostOfAnEntireLineItselfEvenWithoutTheApiResponseContainer",
            ))],
        )])
        .body(sigil_quote!(Python { return source }).unwrap())
        .build()
        .unwrap();
    let file = FileSpec::builder("signatures.py")
        .add_function(single)
        .add_function(multi)
        .build()
        .unwrap();
    assert_ruff_file(&file, &WIDTHS);
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn conditional_statement_operands_are_ruff_fixed_points() {
    let generator = sigil_quote!(Python {
        def iterate(): {
            yield from first_iterable if use_first else second_iterable
        }
    })
    .unwrap();
    let raises = sigil_quote!(Python {
        def rethrow(): {
            raise first_error if should_use_first_error else second_error from original_cause
        }
    })
    .unwrap();
    let cause = sigil_quote!(Python {
        def raise_for_cause(): {
            raise first_error from primary_cause if should_use_primary_cause else fallback_cause
        }
    })
    .unwrap();
    let awaits = sigil_quote!(Python {
        async def evaluate(): {
            await first_awaitable_candidate if use_it else fallback_value
        }
    })
    .unwrap();
    let comparisons = sigil_quote!(Python {
        def compare(): {
            lower = first_operand <= second_operand if use_first else fallback_value
            upper = first_operand >= second_operand if use_first else fallback_value
            value <<= first_choice if use_first else fallback_value
        }
    })
    .unwrap();
    let file = FileSpec::builder("conditionals.py")
        .add_code(generator)
        .add_code(raises)
        .add_code(cause)
        .add_code(awaits)
        .add_code(comparisons)
        .build()
        .unwrap();
    assert_ruff_file(&file, &WIDTHS);
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn literal_separator_loops_are_ruff_fixed_points() {
    let cases: [(&str, &[&str]); 3] = [
        ("loop_empty.py", &[]),
        ("loop_single_item.py", &["first_argument"]),
        (
            "loop_many_items.py",
            &["first_argument", "second_argument", "third_argument"],
        ),
    ];
    for (filename, names) in cases {
        let block = sigil_quote!(Python {
            values = [$for(name in names; separator = ", ") { $N(*name) }]
            result = function($for(name in names; separator = ", ") { $N(*name) })
        })
        .unwrap();
        assert_ruff_file(&quoted_file(filename, block), &WIDTHS);
    }
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn long_assignment_with_an_empty_loop_is_a_ruff_fixed_point() {
    let names: [&str; 0] = [];
    let block = sigil_quote!(Python {
        an_assignment_target_that_is_too_long_to_fit_at_width_40 = [$for(name in names; separator = ", ") { $N(name) }]
    })
    .unwrap();
    assert_ruff_file(&quoted_file("overlong_empty_loop.py", block), &[40]);
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn explicit_loop_trailing_commas_are_ruff_fixed_points() {
    let block = sigil_quote!(Python {
        value = ($for(name in ["single"]; separator = ", ", trailing = true) { $N(name) })
        result = f($for(name in ["first", "second"]; separator = ", ", trailing = true) { $N(name) })
    })
    .unwrap();
    let file = quoted_file("explicit_loop_trailing.py", block);
    for width in WIDTHS {
        let source = file.render(width).unwrap();
        assert_eq!(
            source,
            "value = (single,)\nresult = f(\n    first,\n    second,\n)\n"
        );
        assert_ruff_fixed_point(&source, file.filename(), width);
    }
}

#[test]
#[ignore = "requires the pinned Ruff 0.17.5 oracle; run scripts/check-ruff-python.sh"]
fn literal_separator_loop_subscripts_are_ruff_fixed_points() {
    let scalar_key = "a_lookup_key_that_cannot_fit_on_the_continuation_line";
    let generic_key = "another_lookup_key_that_cannot_fit_on_the_continuation_line";
    let block = sigil_quote!(Python {
        scalar = values[$for(name in [scalar_key]; separator = ", ") { $N(name) }]
        type_like = Values[$for(name in [generic_key]; separator = ", ") { $N(name) }]
        explicit_tuple = values[$for(name in ["only_key"]; separator = ", ", trailing = true) { $N(name) }]
        implicit_tuple = values[$for(name in ["first_long_lookup_key", "second_long_lookup_key"]; separator = ", ") { $N(name) }]
    })
    .unwrap();
    let file = quoted_file("subscript_loops.py", block);
    for width in [32, 40, 88] {
        let source = file.render(width).unwrap();
        assert!(!source.contains(&format!("{scalar_key},")), "{source}");
        assert!(!source.contains(&format!("{generic_key},")), "{source}");
        assert!(source.contains("only_key,"), "{source}");
        assert_ruff_fixed_point(&source, file.filename(), width);
    }
}
