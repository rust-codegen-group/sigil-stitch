//! Compare target-specific ways to describe a response's named fields.
//!
//! Run: `cargo run --example response_types`
//! Write the generated files: `cargo run --example response_types -- <directory>`
//!
//! These are related examples, not one portable anonymous-record type. TypeScript
//! composes existing TypeName values; Go, Dart and Swift use target-associated
//! source fragments with structured type references; Python, Rust and Kotlin use
//! declaration specs. No new TypeName variant or automatic translation is needed.
//!
//! Only the TypeScript and Python examples constrain status to exactly 200. Their
//! numeric leaf uses the existing Raw escape hatch, not a numeric singleton API.
//! Optional keys are not modeled as TypeName::Optional nullable/optional values.

use std::error::Error;
use std::path::Path;

use sigil_stitch::lang::{
    dart::Dart, go::Go, kotlin::Kotlin, python::Python, rust::Rust, swift::Swift,
};
use sigil_stitch::prelude::*;

fn main() -> Result<(), Box<dyn Error>> {
    let project = response_types()?;
    match std::env::args_os().nth(1) {
        Some(directory) => {
            for path in project.write_to(Path::new(&directory), 100)? {
                println!("{}", path.display());
            }
        }
        None => {
            for file in project.render(100)? {
                println!("===== {} =====\n\n{}", file.path, file.content);
            }
        }
    }
    Ok(())
}

fn response_types() -> Result<ProjectSpec, SigilStitchError> {
    ProjectSpec::builder()
        .add_file(typescript_models()?)
        .add_file(typescript_response()?)
        .add_file(go_response()?)
        .add_file(dart_response()?)
        .add_file(swift_response()?)
        .add_file(python_response()?)
        .add_file(rust_response()?)
        .add_file(kotlin_response()?)
        .build()
}

fn typescript_models() -> Result<FileSpec, SigilStitchError> {
    FileSpec::builder("typescript/models.ts")
        .add_type(
            TypeSpec::builder("Body", TypeKind::TypeAlias)
                .visibility(Visibility::Public)
                .extends(TypeName::primitive("string"))
                .build()?,
        )
        .build()
}

fn typescript_response() -> Result<FileSpec, SigilStitchError> {
    // A singleton key makes Record describe a required named property here.
    // This TypeScript-specific use does not give Map record semantics elsewhere.
    let status = TypeName::map(TypeName::string_literal("status"), TypeName::raw("200"));
    let body = TypeName::map(
        TypeName::string_literal("body"),
        TypeName::importable_type("./models", "Body").with_alias("ResponseBody"),
    );
    // Partial permits an absent key; TypeName::optional would permit null instead.
    let optional_body = TypeName::application(
        TypeName::primitive("Partial"),
        vec![TypeArgument::Single(body.clone())],
    );

    FileSpec::builder("typescript/response.ts")
        .header(CodeBlock::of(
            "// Required and optional keys, both with status fixed to 200.\n",
            (),
        )?)
        .add_type(
            TypeSpec::builder("Response", TypeKind::TypeAlias)
                .visibility(Visibility::Public)
                .extends(TypeName::intersection(vec![status.clone(), body]))
                .build()?,
        )
        .add_type(
            TypeSpec::builder("OptionalResponse", TypeKind::TypeAlias)
                .visibility(Visibility::Public)
                .extends(TypeName::intersection(vec![status, optional_body]))
                .build()?,
        )
        .add_code(CodeBlock::of(
            "const complete: Response = { status: 200, body: 'ok' };\nconst omitted: OptionalResponse = { status: 200 };\n",
            (),
        )?)
        .build()
}

fn go_response() -> Result<FileSpec, SigilStitchError> {
    // The anonymous struct is target source, not a Raw TypeName containing types.
    FileSpec::builder_with("go/response.go", Go::new())
        .header(CodeBlock::of("package response\n", ())?)
        .add_code(CodeBlock::of(
            "// An alias for an anonymous struct; Status can hold any int.\ntype Response = struct {\n%>Status %T\nBody %T\n%<}\n\nvar example = Response{Status: 200, Body: \"ok\"}\n",
            (TypeName::primitive("int"), TypeName::primitive("string")),
        )?)
        .build()
}

fn dart_response() -> Result<FileSpec, SigilStitchError> {
    // TypeName::Tuple is positional; named record syntax stays local to Dart.
    FileSpec::builder_with("dart/response.dart", Dart::new())
        .add_code(CodeBlock::of(
            "// Dart 3 named record; status can hold any int.\ntypedef Response = ({%T status, %T body});\n\nconst Response example = (status: 200, body: 'ok');\n",
            (TypeName::primitive("int"), TypeName::primitive("String")),
        )?)
        .build()
}

fn swift_response() -> Result<FileSpec, SigilStitchError> {
    // Use two elements: Swift has no single-element tuple type.
    FileSpec::builder_with("swift/response.swift", Swift::new())
        .add_code(CodeBlock::of(
            "// A labelled tuple, not an object type; status can hold any Int.\ntypealias Response = (status: %T, body: %T)\n\nlet example: Response = (status: 200, body: \"ok\")\n",
            (TypeName::primitive("Int"), TypeName::primitive("String")),
        )?)
        .build()
}

fn python_response() -> Result<FileSpec, SigilStitchError> {
    let status = TypeName::application(
        TypeName::importable("typing", "Literal"),
        vec![TypeArgument::Single(TypeName::raw("200"))],
    );
    let optional_body = TypeName::application(
        TypeName::importable("typing", "NotRequired"),
        vec![TypeArgument::Single(TypeName::primitive("str"))],
    );

    // TypedDict/NotRequired are explicit Python names, not a special shared kind
    // or a nullable-value fallback for optional presence. They need a type checker;
    // the annotations do not validate dictionary contents at runtime.
    FileSpec::builder_with("python/response.py", Python::new())
        .header(CodeBlock::of(
            "# Python 3.11+ named TypedDicts; NotRequired permits a missing key.\n",
            (),
        )?)
        .add_type(
            TypeSpec::builder("Response", TypeKind::Class)
                .extends(TypeName::importable("typing", "TypedDict"))
                .add_field(FieldSpec::new("status", status.clone())?)
                .add_field(FieldSpec::new("body", TypeName::primitive("str"))?)
                .build()?,
        )
        .add_type(
            TypeSpec::builder("OptionalResponse", TypeKind::Class)
                .extends(TypeName::importable("typing", "TypedDict"))
                .add_field(FieldSpec::new("status", status)?)
                .add_field(FieldSpec::new("body", optional_body)?)
                .build()?,
        )
        .add_code(CodeBlock::of(
            "complete: Response = {\"status\": 200, \"body\": \"ok\"}\nomitted: OptionalResponse = {\"status\": 200}\n",
            (),
        )?)
        .build()
}

fn rust_response() -> Result<FileSpec, SigilStitchError> {
    FileSpec::builder_with("rust/response.rs", Rust::new())
        .add_type(
            TypeSpec::builder("Response", TypeKind::Struct)
                .visibility(Visibility::Public)
                .doc("A named struct; status can hold any u16, not only 200.")
                .add_field(
                    FieldSpec::builder("status", TypeName::primitive("u16"))
                        .visibility(Visibility::Public)
                        .build()?,
                )
                .add_field(
                    FieldSpec::builder("body", TypeName::primitive("String"))
                        .visibility(Visibility::Public)
                        .build()?,
                )
                .build()?,
        )
        .build()
}

fn kotlin_response() -> Result<FileSpec, SigilStitchError> {
    FileSpec::builder_with("kotlin/Response.kt", Kotlin::new())
        .add_type(
            TypeSpec::builder("Response", TypeKind::Struct)
                .doc("A named data class; status can hold any Int, not only 200.")
                .add_primary_constructor_param(
                    ParameterSpec::builder("status", TypeName::primitive("Int"))
                        .is_property()
                        .build()?,
                )
                .add_primary_constructor_param(
                    ParameterSpec::builder("body", TypeName::primitive("String"))
                        .is_property()
                        .build()?,
                )
                .build()?,
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typescript_composition_preserves_import_aliases_and_optional_keys() {
        for width in [100, 32] {
            let output = typescript_response().unwrap().render(width).unwrap();
            assert!(output.contains("import type { Body as ResponseBody } from './models';"));
            let compact: String = output.split_whitespace().collect();
            assert!(compact.contains("Response=Record<'status',200>&Record<'body',ResponseBody>"));
            assert!(compact.contains(
                "OptionalResponse=Record<'status',200>&Partial<Record<'body',ResponseBody>>"
            ));
            assert!(!compact.contains("null"));
        }
    }

    #[test]
    fn native_fragments_preserve_their_distinct_type_syntax() {
        let go = go_response().unwrap().render(100).unwrap();
        assert!(go.contains("type Response = struct {"));
        assert!(go.contains("Status int\n"));
        assert!(go.contains("Body string\n"));

        let dart = dart_response().unwrap().render(100).unwrap();
        assert!(dart.contains("typedef Response = ({int status, String body});"));

        let swift = swift_response().unwrap().render(100).unwrap();
        assert!(swift.contains("typealias Response = (status: Int, body: String)"));
    }

    #[test]
    fn named_declarations_keep_target_owned_field_semantics() {
        let python = python_response().unwrap().render(100).unwrap();
        assert!(python.contains("from typing import Literal, NotRequired, TypedDict"));
        assert!(python.contains("class Response(TypedDict):"));
        assert!(python.contains("status: Literal[200]"));
        assert!(python.contains("body: NotRequired[str]"));

        let rust = rust_response().unwrap().render(100).unwrap();
        assert!(rust.contains("pub struct Response {"));
        assert!(rust.contains("pub status: u16,"));
        assert!(rust.contains("pub body: String,"));

        let kotlin = kotlin_response().unwrap().render(100).unwrap();
        assert!(kotlin.contains("data class Response(val status: Int, val body: String)"));
    }
}
