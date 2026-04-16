//! Generate a TypeScript file using structural specs.
//!
//! Run with: `cargo run --example typescript_hello_world`

use sigil_stitch::code_block::{CodeBlock, StringLitArg};
use sigil_stitch::lang::typescript::TypeScript;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

fn main() {
    // Define types that need imports.
    let user_type = TypeName::<TypeScript>::importable_type("./models", "User");
    let not_found = TypeName::<TypeScript>::importable_type("./errors", "NotFoundError");

    // Build the class using TypeSpec.
    let mut tb = TypeSpec::<TypeScript>::builder("UserService", TypeKind::Class);
    tb.visibility(Visibility::Public);
    tb.doc("Service for managing users.");

    // Private field.
    let mut field_b = FieldSpec::builder("userRepo", TypeName::primitive("UserRepository"));
    field_b.visibility(Visibility::Private);
    field_b.is_readonly();
    tb.add_field(field_b.build().unwrap());

    // Async method with control flow body.
    let mut body = CodeBlock::<TypeScript>::builder();
    body.add_statement(
        "const user = await this.userRepo.findById(%S)",
        (StringLitArg("id".to_string()),),
    );
    body.begin_control_flow("if (!user)", ());
    body.add_statement("throw new %T('User not found')", (not_found,));
    body.end_control_flow();
    body.add_statement("return user", ());
    let body_block = body.build().unwrap();

    let mut fb = FunSpec::builder("getUser");
    fb.is_async();
    fb.add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap());
    fb.returns(TypeName::generic(
        TypeName::primitive("Promise"),
        vec![user_type],
    ));
    fb.body(body_block);
    tb.add_method(fb.build().unwrap());

    // Build the file.
    let mut file = FileSpec::<TypeScript>::builder("UserService.ts");
    file.add_type(tb.build().unwrap());
    let spec = file.build().unwrap();

    // Render at 80 columns.
    let output = spec.render(80).unwrap();
    println!("{output}");
}
