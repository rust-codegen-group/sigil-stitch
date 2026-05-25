use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::java::Java;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_control_flow() {
    let mut b = CodeBlock::builder();
    b.begin_control_flow("if (x > 0)", ());
    b.add_statement("return 1", ());
    b.next_control_flow("else if (x < 0)", ());
    b.add_statement("return -1", ());
    b.next_control_flow("else", ());
    b.add_statement("return 0", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("Flow.java", Java::new())
        .add_code(block)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("java/control_flow.java", &output);
}
