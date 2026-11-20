use sigil_stitch::lang::python::Python;
use sigil_stitch::prelude::*;

const WIDTH: usize = 100;

fn api_response(payload: TypeName) -> TypeName {
    TypeName::application(
        TypeName::importable(".runtime", "ApiResponse"),
        vec![TypeArgument::Single(payload)],
    )
}

#[test]
fn method_wraps_parameters_before_a_fitting_generic_return() {
    let method = FunSpec::builder("upload_asset_with_http_info")
        .add_param(ParameterSpec::of("self", TypeName::primitive("")))
        .add_param(ParameterSpec::of("*", TypeName::primitive("")))
        .add_param(ParameterSpec::of(
            "body",
            TypeName::primitive("UploadAssetMultipartRequestBody"),
        ))
        .returns(vec![api_response(TypeName::primitive("UploadMetadata"))])
        .body(sigil_quote!(Python { return body }).unwrap())
        .build()
        .unwrap();
    let class = TypeSpec::builder("TransferApi", TypeKind::Class)
        .add_method(method)
        .build()
        .unwrap();
    let file = FileSpec::builder_with("transfer_api.py", Python::new())
        .add_type(class)
        .build()
        .unwrap();

    assert_eq!(
        file.render(WIDTH).unwrap(),
        concat!(
            "from .runtime import ApiResponse\n\n\n",
            "class TransferApi:\n",
            "    def upload_asset_with_http_info(\n",
            "        self, *, body: UploadAssetMultipartRequestBody\n",
            "    ) -> ApiResponse[UploadMetadata]:\n",
            "        return body\n",
        ),
    );
}

#[test]
fn broken_single_parameter_has_a_trailing_comma() {
    let function = FunSpec::builder("constrained_inline_additional_property_item_from_dict")
        .add_param(ParameterSpec::of("value", TypeName::primitive("object")))
        .returns(vec![TypeName::primitive(
            "ConstrainedInlineAdditionalPropertyItem",
        )])
        .body(sigil_quote!(Python { return value }).unwrap())
        .build()
        .unwrap();
    let file = FileSpec::builder_with("constrained_inline.py", Python::new())
        .add_function(function)
        .build()
        .unwrap();

    assert_eq!(
        file.render(WIDTH).unwrap(),
        concat!(
            "def constrained_inline_additional_property_item_from_dict(\n",
            "    value: object,\n",
            ") -> ConstrainedInlineAdditionalPropertyItem:\n",
            "    return value\n",
        ),
    );
}

#[test]
fn wrapped_signature_packs_keyword_separator_and_preserves_default_spacing() {
    let none = sigil_quote!(Python { None }).unwrap();
    let function = FunSpec::builder(
        "map_detail_with_an_explicit_default_and_a_long_name_to_exercise_wrapped_signatures",
    )
    .is_async()
    .add_param(ParameterSpec::of("*", TypeName::primitive("")))
    .add_param(
        ParameterSpec::builder("raw", TypeName::primitive(""))
            .default_value(none.clone())
            .build()
            .unwrap(),
    )
    .add_param(
        ParameterSpec::builder("payload", TypeName::optional(TypeName::primitive("str")))
            .default_value(none)
            .build()
            .unwrap(),
    )
    .returns(vec![api_response(TypeName::primitive("str"))])
    .body(sigil_quote!(Python { return payload }).unwrap())
    .build()
    .unwrap();
    let file = FileSpec::builder_with("defaults.py", Python::new())
        .add_function(function)
        .build()
        .unwrap();

    assert_eq!(
        file.render(WIDTH).unwrap(),
        concat!(
            "from .runtime import ApiResponse\n\n\n",
            "async def map_detail_with_an_explicit_default_and_a_long_name_to_exercise_wrapped_signatures(\n",
            "    *, raw=None, payload: str | None = None\n",
            ") -> ApiResponse[str]:\n",
            "    return payload\n",
        ),
    );
}

#[test]
fn unbroken_single_parameter_does_not_gain_a_trailing_comma() {
    let function = FunSpec::builder("short")
        .add_param(
            ParameterSpec::builder("value", TypeName::primitive("str"))
                .default_value(sigil_quote!(Python { "fallback" }).unwrap())
                .build()
                .unwrap(),
        )
        .returns(vec![api_response(TypeName::primitive("str"))])
        .build()
        .unwrap();
    let file = FileSpec::builder_with("short.py", Python::new())
        .add_function(function)
        .build()
        .unwrap();

    assert_eq!(
        file.render(WIDTH).unwrap(),
        "from .runtime import ApiResponse\n\n\ndef short(value: str = \"fallback\") -> ApiResponse[str]: ...\n",
    );
}

#[test]
fn long_parameters_and_return_annotation_can_both_break() {
    let function = FunSpec::builder("fetch_resource")
        .add_param(ParameterSpec::of(
            "source",
            TypeName::primitive("SourceWithANameThatTakesUpMoreThanHalfOfOneLine"),
        ))
        .add_param(ParameterSpec::of(
            "sink",
            TypeName::primitive("SinkWithANameThatTakesUpMoreThanHalfOfOneLine"),
        ))
        .returns(vec![api_response(TypeName::primitive(
            "ExtremelyLongReturnClassNameWhichUsesMostOfAnEntireLineItselfEvenWithoutTheApiResponseContainer",
        ))])
        .body(sigil_quote!(Python { return source }).unwrap())
        .build()
        .unwrap();
    let file = FileSpec::builder_with("fetch_resource.py", Python::new())
        .add_function(function)
        .build()
        .unwrap();

    assert_eq!(
        file.render(WIDTH).unwrap(),
        concat!(
            "from .runtime import ApiResponse\n\n\n",
            "def fetch_resource(\n",
            "    source: SourceWithANameThatTakesUpMoreThanHalfOfOneLine,\n",
            "    sink: SinkWithANameThatTakesUpMoreThanHalfOfOneLine,\n",
            ") -> ApiResponse[\n",
            "    ExtremelyLongReturnClassNameWhichUsesMostOfAnEntireLineItselfEvenWithoutTheApiResponseContainer\n",
            "]:\n",
            "    return source\n",
        ),
    );
}

#[test]
fn nested_suite_closers_do_not_accumulate_blank_lines() {
    let guard = sigil_quote!(Python {
        if present {
            if invalid {
                return None
            }
        }
    })
    .unwrap();
    let method = FunSpec::builder("check")
        .add_param(ParameterSpec::of("self", TypeName::primitive("")))
        .body(
            sigil_quote!(Python {
                $C(guard)
                return value
            })
            .unwrap(),
        )
        .build()
        .unwrap();
    let file = FileSpec::builder("check.py")
        .add_function(method)
        .build()
        .unwrap();
    assert_eq!(
        file.render(WIDTH).unwrap(),
        concat!(
            "def check(self):\n",
            "    if present:\n",
            "        if invalid:\n",
            "            return None\n",
            "\n",
            "    return value\n",
        )
    );
}

#[test]
fn no_newline_nested_control_flow_keeps_following_statement_separate() {
    let mut guard_builder = CodeBlock::builder();
    guard_builder.begin_control_flow("if present", ());
    guard_builder.add("pass", ());
    guard_builder.end_control_flow_no_newline();
    let guard = guard_builder.build().unwrap();

    let mut body_builder = CodeBlock::builder();
    body_builder.add_code(guard);
    body_builder.add_line();
    body_builder.add_statement("return value", ());
    let body = body_builder.build().unwrap();

    let function = FunSpec::builder("check").body(body).build().unwrap();
    let output = FileSpec::builder("check.py")
        .add_function(function)
        .build()
        .unwrap()
        .render(WIDTH)
        .unwrap();

    assert_eq!(
        output,
        "def check():\n    if present:\n        pass\n    return value\n"
    );
}
