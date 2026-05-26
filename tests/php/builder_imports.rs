use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::php::Php;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_imports() {
    let logger = TypeName::importable("Psr\\Log", "LoggerInterface");

    let mut b = CodeBlock::builder();
    b.add("function createLogger(): %T {", (logger.clone(),));
    b.add_line();
    b.add("%>", ());
    b.add_statement("return new %T()", (logger,));
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("logger.php", Php::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("php/function_with_imports.php", &output);
}

#[test]
fn test_import_with_alias() {
    let user_model = TypeName::importable("App\\Models", "User").with_alias("UserModel");

    let mut b = CodeBlock::builder();
    b.add_statement("$user = new %T()", (user_model,));
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("user.php", Php::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("php/import_with_alias.php", &output);
}
