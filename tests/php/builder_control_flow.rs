use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::php::Php;
use sigil_stitch::spec::file_spec::FileSpec;

use super::golden;

#[test]
fn test_control_flow() {
    let mut b = CodeBlock::builder();
    b.add("function classify(int $x): string {", ());
    b.add_line();
    b.add("%>", ());
    b.begin_control_flow("if ($x > 0)", ());
    b.add_statement("return \"positive\"", ());
    b.next_control_flow("elseif ($x < 0)", ());
    b.add_statement("return \"negative\"", ());
    b.next_control_flow("else", ());
    b.add_statement("return \"zero\"", ());
    b.end_control_flow();
    b.add("%<", ());
    b.add("}", ());
    b.add_line();
    let block = b.build().unwrap();

    let file = FileSpec::builder_with("classify.php", Php::new())
        .add_code(block)
        .build()
        .unwrap();

    let output = file.render(80).unwrap();
    golden::assert_golden("php/control_flow.php", &output);
}
