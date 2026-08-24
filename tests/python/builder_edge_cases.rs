use sigil_stitch::code_block::{CodeBlock, NameArg};
use sigil_stitch::error::SigilStitchError;
use sigil_stitch::lang::capability::TypeCapability;
use sigil_stitch::lang::python::Python;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_abstract_method() {
    let abc = TypeName::importable("abc", "ABC");
    let abstractmethod = TypeName::importable("abc", "abstractmethod");

    let handle = FunSpec::builder("handle_request")
        .annotation(CodeBlock::of("@%T", (abstractmethod,)).unwrap())
        .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
        .add_param(ParameterSpec::new("req", TypeName::primitive("Request")).unwrap())
        .returns(TypeName::primitive("Response"));
    // No body — should emit `...`

    let tb = TypeSpec::builder("BaseController", TypeKind::Class)
        .extends(abc)
        .add_method(handle.build().unwrap())
        .add_method(
            FunSpec::builder("log")
                .add_param(ParameterSpec::new("self", TypeName::primitive("")).unwrap())
                .returns(TypeName::primitive("None"))
                .body(CodeBlock::of("print('handled')", ()).unwrap())
                .build()
                .unwrap(),
        );

    let file = FileSpec::builder_with("controller.py", Python::new())
        .add_type(tb.build().unwrap())
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("python/abstract_class.py", &output);
}

#[test]
fn test_decorated_function() {
    let file = FileSpec::builder_with("views.py", Python::new())
        .add_function(
            FunSpec::builder("my_view")
                .annotation(CodeBlock::of("@app.route('/hello')", ()).unwrap())
                .returns(TypeName::primitive("str"))
                .body(CodeBlock::of("return 'Hello, World!'", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("python/decorated_function.py", &output);
}

// ── %N keyword escaping ─────────────────────────────────

#[test]
fn test_name_escapes_python_keywords() {
    let keywords = [
        "class", "def", "import", "from", "return", "lambda", "global", "yield",
    ];
    for kw in keywords {
        let block = CodeBlock::of("%N = None", NameArg(kw.into())).unwrap();
        let file = FileSpec::builder_with("test.py", Python::new())
            .add_code(block)
            .build()
            .unwrap();
        let output = file.render(80).unwrap();
        assert!(
            output.contains(&format!("{kw}_")),
            "Expected '{kw}_' for reserved word '{kw}', got: {output}"
        );
    }
}

#[test]
fn test_name_no_escape_python_non_keywords() {
    let names = ["user", "my_class", "data_from", "imports"];
    for name in names {
        let block = CodeBlock::of("%N = None", NameArg(name.into())).unwrap();
        let file = FileSpec::builder_with("test.py", Python::new())
            .add_code(block)
            .build()
            .unwrap();
        let output = file.render(80).unwrap();
        assert!(
            output.contains(&format!("{name} = None")),
            "Expected '{name} = None' in output, got: {output}"
        );
    }
}

#[test]
fn test_name_escape_in_assignment() {
    let mut cb = CodeBlock::builder();
    cb.add_statement(
        "self.%N = %N",
        (NameArg("class".into()), NameArg("class".into())),
    );
    let block = cb.build().unwrap();

    let file = FileSpec::builder_with("test.py", Python::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(output.contains("self.class_ = class_"));
}

// ── Embedded fields in Python ───────────────────────────

#[test]
fn test_embedded_types_in_python_class_fail_closed() {
    let type_ = TypeSpec::builder("AdminUser", TypeKind::Class)
        .add_embedded(TypeName::primitive("User"))
        .add_embedded(TypeName::primitive("Admin"))
        .add_field(
            FieldSpec::builder("role", TypeName::primitive("str"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let error = type_.emit(&Python::new()).unwrap_err();
    assert!(matches!(
        error,
        SigilStitchError::UnsupportedTypeCapabilities {
            ref capabilities,
            ..
        } if capabilities == &[TypeCapability::StructuralEmbedding]
    ));
}
