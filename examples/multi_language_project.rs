//! Showcase: generate a multi-language project from a single schema definition.
//!
//! Produces a TypeScript API client, a Go server handler, a Python test file,
//! and a C# model class from the same field definitions — demonstrating
//! `ProjectSpec` and how the same concepts render differently across languages.
//!
//! Run: `cargo run --example multi_language_project`

use sigil_stitch::lang::csharp::CSharp;
use sigil_stitch::lang::go::Go;
use sigil_stitch::lang::python::Python;
use sigil_stitch::prelude::*;

struct SchemaField {
    name: &'static str,
    ts_type: &'static str,
    go_type: &'static str,
    py_type: &'static str,
    cs_type: &'static str,
}

fn main() {
    let schema = vec![
        SchemaField {
            name: "id",
            ts_type: "number",
            go_type: "int64",
            py_type: "int",
            cs_type: "int",
        },
        SchemaField {
            name: "name",
            ts_type: "string",
            go_type: "string",
            py_type: "str",
            cs_type: "string",
        },
        SchemaField {
            name: "email",
            ts_type: "string",
            go_type: "string",
            py_type: "str",
            cs_type: "string",
        },
        SchemaField {
            name: "active",
            ts_type: "boolean",
            go_type: "bool",
            py_type: "bool",
            cs_type: "bool",
        },
    ];

    let ts_file = build_typescript_client(&schema);
    let go_file = build_go_handler(&schema);
    let py_file = build_python_test(&schema);
    let cs_file = build_csharp_model(&schema);

    let project = ProjectSpec::builder()
        .add_file(ts_file)
        .add_file(go_file)
        .add_file(py_file)
        .add_file(cs_file)
        .build()
        .unwrap();

    let rendered = project.render(80).unwrap();
    for file in &rendered {
        println!("===== {} =====\n", file.path);
        println!("{}", file.content);
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + chars.as_str(),
    }
}

fn build_typescript_client(schema: &[SchemaField]) -> FileSpec {
    let axios = TypeName::importable("axios", "AxiosInstance");

    let mut iface = TypeSpec::builder("User", TypeKind::Interface).visibility(Visibility::Public);
    for field in schema {
        iface = iface.add_field(
            FieldSpec::builder(field.name, TypeName::primitive(field.ts_type))
                .build()
                .unwrap(),
        );
    }

    let get_body = sigil_quote!(TypeScript {
        const response = await this.client.get($S("/users/") + String(id));
        return response.data;
    })
    .unwrap();

    let create_body = sigil_quote!(TypeScript {
        const response = await this.client.post($S("/users"), data);
        return response.data;
    })
    .unwrap();

    let cls = TypeSpec::builder("UserClient", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("HTTP client for User CRUD operations.")
        .add_field(
            FieldSpec::builder("client", TypeName::primitive("AxiosInstance"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("getUser")
                .is_async()
                .add_param(ParameterSpec::new("id", TypeName::primitive("number")).unwrap())
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("User")],
                ))
                .body(get_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("createUser")
                .is_async()
                .add_param(
                    ParameterSpec::new("data", TypeName::primitive("Omit<User, 'id'>")).unwrap(),
                )
                .returns(TypeName::generic(
                    TypeName::primitive("Promise"),
                    vec![TypeName::primitive("User")],
                ))
                .body(create_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let trigger = CodeBlock::of("// %T", (axios,)).unwrap();

    FileSpec::builder("client.ts")
        .add_code(trigger)
        .add_type(iface.build().unwrap())
        .add_type(cls)
        .build()
        .unwrap()
}

fn build_go_handler(schema: &[SchemaField]) -> FileSpec {
    let json_decoder = TypeName::importable("encoding/json", "NewDecoder");
    let http = TypeName::importable("net/http", "HandlerFunc");

    let mut ts = TypeSpec::builder("User", TypeKind::Struct).doc("User represents a user entity.");
    for field in schema {
        ts = ts.add_field(
            FieldSpec::builder(&capitalize(field.name), TypeName::primitive(field.go_type))
                .tag(&format!("json:\"{}\"", field.name))
                .build()
                .unwrap(),
        );
    }

    let mut handler_body = CodeBlock::builder();
    handler_body.add("var user User", ());
    handler_body.add_line();
    handler_body.begin_control_flow(
        "if err := %T(r.Body).Decode(&user); err != nil",
        (json_decoder,),
    );
    handler_body.add_statement("http.Error(w, err.Error(), http.StatusBadRequest)", ());
    handler_body.add("return", ());
    handler_body.add_line();
    handler_body.end_control_flow();
    handler_body.add_statement("w.WriteHeader(http.StatusCreated)", ());

    let handler_fn = FunSpec::builder("CreateUserHandler")
        .doc("CreateUserHandler handles POST /users.")
        .add_param(ParameterSpec::new("w", TypeName::primitive("http.ResponseWriter")).unwrap())
        .add_param(
            ParameterSpec::new("r", TypeName::pointer(TypeName::primitive("http.Request")))
                .unwrap(),
        )
        .body(handler_body.build().unwrap())
        .build()
        .unwrap();

    let trigger = CodeBlock::of("// %T", (http,)).unwrap();

    FileSpec::builder_with("handler.go", Go::new())
        .header(CodeBlock::of("package api", ()).unwrap())
        .add_code(trigger)
        .add_type(ts.build().unwrap())
        .add_function(handler_fn)
        .build()
        .unwrap()
}

fn build_python_test(schema: &[SchemaField]) -> FileSpec {
    let dataclass = TypeName::importable("dataclasses", "dataclass");

    let mut cls = TypeSpec::builder("User", TypeKind::Class)
        .annotation(CodeBlock::of("@%T", (dataclass,)).unwrap());
    for field in schema {
        cls = cls.add_field(
            FieldSpec::builder(field.name, TypeName::primitive(field.py_type))
                .build()
                .unwrap(),
        );
    }

    let test_body = CodeBlock::of(
        "user = User(id=1, name=%S, email=%S, active=True)\nassert user.name == %S\nassert user.active is True",
        (
            StringLitArg("Alice".into()),
            StringLitArg("alice@example.com".into()),
            StringLitArg("Alice".into()),
        ),
    )
    .unwrap();

    let test_fn = FunSpec::builder("test_user_creation")
        .doc("Verify User can be instantiated from schema fields.")
        .body(test_body)
        .build()
        .unwrap();

    FileSpec::builder_with("test_user.py", Python::new())
        .add_type(cls.build().unwrap())
        .add_function(test_fn)
        .build()
        .unwrap()
}

fn build_csharp_model(schema: &[SchemaField]) -> FileSpec {
    let mut cls = TypeSpec::builder("User", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("User entity generated from schema.");
    for field in schema {
        cls = cls.add_field(
            FieldSpec::builder(&capitalize(field.name), TypeName::primitive(field.cs_type))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        );
    }

    let mut to_string_body = CodeBlock::builder();
    to_string_body.add_statement("return $\"User {{ Name={Name}, Email={Email} }}\"", ());

    cls = cls.add_method(
        FunSpec::builder("ToString")
            .visibility(Visibility::Public)
            .is_override()
            .returns(TypeName::primitive("string"))
            .body(to_string_body.build().unwrap())
            .build()
            .unwrap(),
    );

    FileSpec::builder_with("User.cs", CSharp::new())
        .add_type(cls.build().unwrap())
        .build()
        .unwrap()
}
