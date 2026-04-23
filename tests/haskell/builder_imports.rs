use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_function_with_imports() {
    let map_type = TypeName::<Haskell>::importable("Data.Map", "Map");
    let text_type = TypeName::<Haskell>::importable("Data.Text", "Text");

    let mut b = CodeBlock::<Haskell>::builder();
    b.add_statement(
        "let users = %T.fromList [(%T.pack \"alice\", 1)]",
        (map_type, text_type),
    );
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("App.hs", Haskell::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/function_with_imports.hs", &output);
}

#[test]
fn test_import_grouping() {
    let put_str_ln = TypeName::<Haskell>::importable("Prelude", "putStrLn");
    let map_type = TypeName::<Haskell>::importable("Data.Map", "Map");
    let from_list = TypeName::<Haskell>::importable("Data.Map", "fromList");
    let when_fn = TypeName::<Haskell>::importable("Control.Monad", "when");
    let user = TypeName::<Haskell>::importable("MyApp.Types", "User");

    let mut b = CodeBlock::<Haskell>::builder();
    b.add(
        "-- %T %T %T %T %T",
        (put_str_ln, map_type, from_list, when_fn, user),
    );
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("Imports.hs", Haskell::new());
    fb.add_code(block);
    let file = fb.build().unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/import_grouping.hs", &output);
}
