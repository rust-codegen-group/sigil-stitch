use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::rust::Rust;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_control_flow() {
    let mut b = CodeBlock::builder();
    b.add("pub fn classify(x: i32) -> &'static str {", ());
    b.add_line();
    b.add("%>", ());
    b.begin_control_flow("if x > 0", ());
    b.add("\"positive\"", ());
    b.add_line();
    b.next_control_flow("else if x < 0", ());
    b.add("\"negative\"", ());
    b.add_line();
    b.next_control_flow("else", ());
    b.add("\"zero\"", ());
    b.add_line();
    b.end_control_flow();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("classify.rs", Rust::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("rust/control_flow.rs", &output);
}
