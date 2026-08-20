use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::capability::FunctionCapability;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_params() {
    let body = CodeBlock::of("x + y", ()).unwrap();
    let fun = FunSpec::builder("add")
        .returns(TypeName::primitive("Int"))
        .add_param(ParameterSpec::new("x", TypeName::primitive("Int")).unwrap())
        .add_param(ParameterSpec::new("y", TypeName::primitive("Int")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Add.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_with_params.hs", &output);
}

#[test]
fn test_function_with_import() {
    let map_type = TypeName::importable("Data.Map", "Map");

    let body = CodeBlock::of("Data.Map.empty", ()).unwrap();
    let fun = FunSpec::builder("emptyMap")
        .returns(map_type)
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("EmptyMap.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    assert!(output.contains("import Data.Map (Map)"), "{output}");
    golden::assert_golden("haskell/function_with_import.hs", &output);
}

#[test]
fn test_split_signature_preserves_compound_param_and_return_types() {
    let text = TypeName::importable("Data.Text", "Text");
    let user = TypeName::importable("Domain.User", "User");
    let map = TypeName::importable("Data.Map", "Map");
    let return_type = TypeName::generic(map, vec![text.clone(), TypeName::optional(user)]);
    let fun = FunSpec::builder("transform")
        .add_param(ParameterSpec::new("value", TypeName::optional(text)).unwrap())
        .returns(return_type)
        .body(CodeBlock::of("undefined", ()).unwrap())
        .build()
        .unwrap();

    let output = FileSpec::builder_with("Transform.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import Data.Map (Map)"), "{output}");
    assert!(output.contains("import Data.Text (Text)"), "{output}");
    assert!(output.contains("import Domain.User (User)"), "{output}");
    assert!(
        output.contains("transform :: Maybe Text -> Map Text (Maybe User)"),
        "{output}"
    );
}

#[test]
fn test_split_signature_rejects_parameter_types_without_return_type() {
    let consume = FunSpec::builder("consume")
        .add_param(
            ParameterSpec::new("value", TypeName::importable("Domain.Input", "Input")).unwrap(),
        )
        .body(CodeBlock::of("undefined", ()).unwrap())
        .build()
        .unwrap();

    let error = FileSpec::builder_with("Consumer.hs", Haskell::new())
        .add_function(consume)
        .build()
        .unwrap()
        .render(80)
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::FileSpecValidation { errors, .. }
            if matches!(errors.as_slice(), [SigilStitchError::MissingRequiredFunctionCapabilities {
                capabilities,
                ..
            }] if capabilities == &vec![FunctionCapability::ExplicitReturnType])
    ));
}

#[test]
fn test_split_signature_rejects_untyped_parameters_with_return_type() {
    let consume = FunSpec::builder("consume")
        .add_param(ParameterSpec::new("value", TypeName::primitive("")).unwrap())
        .returns(TypeName::primitive("Int"))
        .body(CodeBlock::of("0", ()).unwrap())
        .build()
        .unwrap();

    let error = FileSpec::builder_with("Consumer.hs", Haskell::new())
        .add_function(consume)
        .build()
        .unwrap()
        .render(80)
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::FileSpecValidation { errors, .. }
            if matches!(errors.as_slice(), [SigilStitchError::MissingRequiredFunctionCapabilities {
                capabilities,
                ..
            }] if capabilities == &vec![FunctionCapability::TypedParameters])
    ));
}

#[test]
fn test_split_signature_qualifies_conflicting_import_names() {
    let fun = FunSpec::builder("convert")
        .add_param(
            ParameterSpec::new("value", TypeName::importable("Domain.Input", "Value")).unwrap(),
        )
        .returns(TypeName::importable("Domain.Output", "Value"))
        .body(CodeBlock::of("undefined", ()).unwrap())
        .build()
        .unwrap();

    let output = FileSpec::builder_with("Convert.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(output.contains("import Domain.Input (Value)"), "{output}");
    assert!(
        output.contains("import qualified Domain.Output (Value)"),
        "{output}"
    );
    assert!(
        output.contains("convert :: Value -> Domain.Output.Value"),
        "{output}"
    );
}

#[test]
fn test_split_signature_preserves_imported_context_bounds() {
    let fun = FunSpec::builder("forceDisplay")
        .add_type_param(
            sigil_stitch::spec::where_spec::TypeParamSpec::new("a")
                .with_bound(TypeName::importable("Control.DeepSeq", "NFData"))
                .with_bound(
                    TypeName::importable("Domain.DeepSeq", "NFData").with_alias("DomainNFData"),
                ),
        )
        .add_param(ParameterSpec::new("value", TypeName::primitive("a")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(CodeBlock::of("show value", ()).unwrap())
        .build()
        .unwrap();

    let output = FileSpec::builder_with("ForceDisplay.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap()
        .render(80)
        .unwrap();

    assert!(
        output.contains("import Control.DeepSeq (NFData)"),
        "{output}"
    );
    assert!(
        output.contains("import qualified Domain.DeepSeq (NFData)"),
        "{output}"
    );
    assert!(
        output.contains("(NFData a, Domain.DeepSeq.NFData a) => "),
        "{output}"
    );
}

#[test]
fn test_function_with_context() {
    let body = CodeBlock::of("show x", ()).unwrap();
    let fun = FunSpec::builder("display")
        .add_type_param(
            sigil_stitch::spec::where_spec::TypeParamSpec::new("a")
                .with_bound(TypeName::primitive("Show")),
        )
        .add_param(ParameterSpec::new("x", TypeName::primitive("a")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Display.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_with_context.hs", &output);
}

#[test]
fn test_function_no_body() {
    let fun = FunSpec::builder("add")
        .returns(TypeName::primitive("Int"))
        .add_param(ParameterSpec::new("x", TypeName::primitive("Int")).unwrap())
        .add_param(ParameterSpec::new("y", TypeName::primitive("Int")).unwrap())
        .build()
        .unwrap();

    let error = FileSpec::builder_with("Add.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap()
        .render(80)
        .unwrap_err();

    assert!(matches!(
        error,
        SigilStitchError::FileSpecValidation { errors, .. }
            if matches!(errors.as_slice(), [SigilStitchError::FunctionBodyRequired { .. }])
    ));
}

#[test]
fn test_function_with_doc() {
    let body = CodeBlock::of("putStrLn (\"Hello, \" ++ name)", ()).unwrap();
    let fun = FunSpec::builder("greet")
        .doc("Greet the user by name.")
        .add_param(ParameterSpec::new("name", TypeName::primitive("String")).unwrap())
        .returns(TypeName::primitive("IO ()"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("greet.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_with_doc.hs", &output);
}

#[test]
fn test_multi_constraint_context() {
    let body = CodeBlock::of("show x", ()).unwrap();
    let fun = FunSpec::builder("display")
        .add_type_param(
            sigil_stitch::spec::where_spec::TypeParamSpec::new("a")
                .with_bound(TypeName::primitive("Show"))
                .with_bound(TypeName::primitive("Eq")),
        )
        .add_param(ParameterSpec::new("x", TypeName::primitive("a")).unwrap())
        .returns(TypeName::primitive("String"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Display.hs", Haskell::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_multi_context.hs", &output);
}
