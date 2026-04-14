mod golden;

use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::swift::Swift;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::type_name::TypeName;

#[test]
fn test_swift_function_with_imports() {
    let url = TypeName::<Swift>::importable("Foundation", "URL");
    let user = TypeName::<Swift>::importable("MyModule", "User");

    let mut b = CodeBlock::<Swift>::builder();
    b.add_statement("let endpoint: %T = getEndpoint()", (url,));
    b.add_statement("let user: %T = fetchUser(endpoint)", (user,));
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("App.swift", Swift::new());
    fb.add_code(block);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/function_with_imports.swift", &output);
}

#[test]
fn test_swift_import_grouping() {
    let url = TypeName::<Swift>::importable("Foundation", "URL");
    let view = TypeName::<Swift>::importable("SwiftUI", "View");
    let vc = TypeName::<Swift>::importable("UIKit", "UIViewController");
    let combine = TypeName::<Swift>::importable("Combine", "Publisher");
    let alamofire = TypeName::<Swift>::importable("Alamofire", "Session");
    let my_type = TypeName::<Swift>::importable("MyModule", "MyType");

    let mut b = CodeBlock::<Swift>::builder();
    b.add(
        "// %T %T %T %T %T %T",
        (url, view, vc, combine, alamofire, my_type),
    );
    b.add_line();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("Imports.swift", Swift::new());
    fb.add_code(block);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/import_grouping.swift", &output);
}

#[test]
fn test_swift_control_flow() {
    let mut b = CodeBlock::<Swift>::builder();
    b.begin_control_flow("if x > 0", ());
    b.add_statement("return 1", ());
    b.next_control_flow("else if x < 0", ());
    b.add_statement("return -1", ());
    b.next_control_flow("else", ());
    b.add_statement("return 0", ());
    b.end_control_flow();
    let block = b.build().unwrap();

    let mut fb = FileSpec::builder_with("Flow.swift", Swift::new());
    fb.add_code(block);
    let file = fb.build();
    let output = file.render(80).unwrap();

    golden::assert_golden("swift/control_flow.swift", &output);
}
