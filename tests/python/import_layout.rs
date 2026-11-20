use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::python::Python;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::import_spec::ImportSpec;

#[test]
fn python_from_imports_wrap_at_file_render_width() {
    let output = FileSpec::builder_with("wide.py", Python::new())
        .add_import(ImportSpec::named(
            "package.submodule.with_a_long_name",
            "AlphaTypeNameWithLongPublicName",
        ))
        .add_import(ImportSpec::named(
            "package.submodule.with_a_long_name",
            "BetaTypeNameWithLongPublicName",
        ))
        .add_import(ImportSpec::named(
            "package.submodule.with_a_long_name",
            "GammaTypeNameWithLongPublicName",
        ))
        .add_function(
            FunSpec::builder("run")
                .body(CodeBlock::of("pass", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap()
        .render(100)
        .unwrap();

    assert_eq!(
        output,
        "from package.submodule.with_a_long_name import (\n    AlphaTypeNameWithLongPublicName,\n    BetaTypeNameWithLongPublicName,\n    GammaTypeNameWithLongPublicName,\n)\n\n\ndef run():\n    pass\n"
    );
}

#[test]
fn python_same_name_alias_remains_an_explicit_public_reexport() {
    let output = FileSpec::builder_with("reexports.py", Python::new())
        .add_import(ImportSpec::named_as(
            ".models",
            "PublicModel",
            "PublicModel",
        ))
        .build()
        .unwrap()
        .render(100)
        .unwrap();

    assert_eq!(output, "from .models import PublicModel as PublicModel\n");
}

#[test]
fn python_import_only_file_has_one_final_newline() {
    let output = FileSpec::builder_with("imports.py", Python::new())
        .add_import(ImportSpec::named("json", "dumps"))
        .build()
        .unwrap()
        .render(100)
        .unwrap();

    assert_eq!(output, "from json import dumps\n");
}
