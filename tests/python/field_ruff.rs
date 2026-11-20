use sigil_stitch::lang::python::Python;
use sigil_stitch::prelude::*;
use sigil_stitch::spec::modifiers::DeclarationContext;

fn render_field_with_lang(
    lang: &Python,
    annotation: TypeName,
    initializer: CodeBlock,
    width: usize,
) -> String {
    FieldSpec::builder("value", annotation)
        .initializer(initializer)
        .build()
        .unwrap()
        .emit(lang, DeclarationContext::Member)
        .unwrap()
        .render_standalone(lang, width)
        .unwrap()
}

fn render_field(annotation: TypeName, initializer: CodeBlock, width: usize) -> String {
    let lang = Python::new();
    render_field_with_lang(&lang, annotation, initializer, width)
}

#[test]
fn annotated_initializer_ignores_pragma_width_but_wraps_when_code_does_not_fit() {
    let mut initializer = CodeBlock::builder();
    initializer
        .add("make_value() ", ())
        .add_comment("type: ignore[assignment, attr-defined]");
    let initializer = initializer.build().unwrap();
    let annotation = TypeName::primitive("VeryLongAnnotatedTypeName");

    assert_eq!(
        render_field(annotation.clone(), initializer.clone(), 60),
        "value: VeryLongAnnotatedTypeName = make_value()  # type: ignore[assignment, attr-defined]\n"
    );
    assert_eq!(
        render_field(annotation, initializer, 40),
        "value: VeryLongAnnotatedTypeName = (\n    make_value()\n)  # type: ignore[assignment, attr-defined]\n"
    );
}

#[test]
fn annotated_initializer_uses_inner_breaks_when_the_call_prefix_fits() {
    let initializer = sigil_quote!(Python {
        make_value(first_arg, second_arg, third_arg)
    })
    .unwrap();
    let annotation = TypeName::primitive("VeryLongAnnotatedTypeName");

    assert_eq!(
        render_field(annotation.clone(), initializer.clone(), 46),
        "value: VeryLongAnnotatedTypeName = make_value(\n    first_arg, second_arg, third_arg\n)\n"
    );
    assert_eq!(
        render_field(annotation, initializer, 40),
        "value: VeryLongAnnotatedTypeName = (\n    make_value(\n        first_arg, second_arg, third_arg\n    )\n)\n"
    );
}

#[test]
fn annotated_initializer_trims_the_quoted_statement_newline() {
    let initializer = sigil_quote!(Python { None }).unwrap();
    assert_eq!(
        render_field(
            TypeName::optional(TypeName::primitive("str")),
            initializer,
            88
        ),
        "value: str | None = None\n"
    );
}

#[test]
fn wrapped_field_initializer_closes_at_class_member_indent() {
    let field = FieldSpec::builder("value", TypeName::primitive("VeryLongAnnotatedTypeName"))
        .initializer(sigil_quote!(Python { make_value() }).unwrap())
        .build()
        .unwrap();
    let class = TypeSpec::builder("Model", TypeKind::Class)
        .add_field(field)
        .build()
        .unwrap();
    let file = FileSpec::builder_with("model.py", Python::new())
        .add_type(class)
        .build()
        .unwrap();
    assert_eq!(
        file.render(44).unwrap(),
        "class Model:\n    value: VeryLongAnnotatedTypeName = (\n        make_value()\n    )\n",
    );
}

#[test]
fn opaque_multiline_initializer_is_not_reindented() {
    let initializer = sigil_quote!(Python {
        $L("\"\"\"first line\nsecond line\"\"\"")
    })
    .unwrap();
    assert_eq!(
        render_field(TypeName::primitive("str"), initializer, 40),
        "value: str = \"\"\"first line\nsecond line\"\"\"\n"
    );
}

#[test]
fn preferred_initializer_preserves_tab_indentation_for_outer_and_inner_breaks() {
    let lang = Python::new().with_indent("\t");
    let initializer = sigil_quote!(Python {
        make_value(first_arg, second_arg, third_arg)
    })
    .unwrap();

    assert_eq!(
        render_field_with_lang(
            &lang,
            TypeName::primitive("VeryLongAnnotatedTypeName"),
            initializer,
            40,
        ),
        "value: VeryLongAnnotatedTypeName = (\n\tmake_value(\n\t\tfirst_arg, second_arg, third_arg\n\t)\n)\n"
    );
}
