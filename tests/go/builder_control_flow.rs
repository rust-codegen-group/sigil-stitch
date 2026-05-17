use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::go_lang::GoLang;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_control_flow() {
    let mut b = CodeBlock::builder();
    b.add("func classify(x int) string {", ());
    b.add_line();
    b.add("%>", ());
    b.begin_control_flow("if x > 0", ());
    b.add_statement("return \"positive\"", ());
    b.end_control_flow();
    b.begin_control_flow("if x < 0", ());
    b.add_statement("return \"negative\"", ());
    b.end_control_flow();
    b.add_statement("return \"zero\"", ());
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("classify.go", GoLang::new())
        .header(CodeBlock::of("package main", ()).unwrap())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/control_flow.go", &output);
}

#[test]
fn test_if_init_semicolon() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if err := doStuff(); err != nil", ());
    b.add_statement("return err", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("test.go", GoLang::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("go/builder_if_init.go", &output);
}
