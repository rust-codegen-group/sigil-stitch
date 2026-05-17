use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::csharp::CSharp;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::{FunSpec, TypeParamSpec};
use sigil_stitch::spec::modifiers::{TypeKind, Visibility};
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_class_with_methods() {
    let ctor_body = CodeBlock::of("this.repo = repo;\nthis.logger = logger;", ()).unwrap();
    let find_body = CodeBlock::of("return this.repo.FindById(id);", ()).unwrap();

    let ts = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .doc("Service for managing users.")
        .add_field(
            FieldSpec::builder("repo", TypeName::primitive("UserRepository"))
                .visibility(Visibility::Private)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("logger", TypeName::primitive("ILogger"))
                .visibility(Visibility::Private)
                .is_readonly()
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("UserService")
                .visibility(Visibility::Public)
                .add_param(
                    ParameterSpec::new("repo", TypeName::primitive("UserRepository")).unwrap(),
                )
                .add_param(ParameterSpec::new("logger", TypeName::primitive("ILogger")).unwrap())
                .body(ctor_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("FindUser")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("User"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .body(find_body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("UserService.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/class_with_methods.cs", &output);
}

#[test]
fn test_interface() {
    let tp = TypeParamSpec::new("T");

    let ts = TypeSpec::builder("IRepository", TypeKind::Interface)
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .doc("Generic data repository.")
        .add_method(
            FunSpec::builder("FindById")
                .returns(TypeName::primitive("T"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Save")
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("IRepository.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/interface.cs", &output);
}

#[test]
fn test_class_extends_implements() {
    let ts = TypeSpec::builder("UserService", TypeKind::Class)
        .visibility(Visibility::Public)
        .extends(TypeName::primitive("BaseService"))
        .extends(TypeName::primitive("IUserService"))
        .extends(TypeName::primitive("IDisposable"))
        .add_method(
            FunSpec::builder("Dispose")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("void"))
                .body(CodeBlock::of("// cleanup", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("UserService.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/class_extends_implements.cs", &output);
}

#[test]
fn test_generic_class() {
    let tp = TypeParamSpec::new("T");

    let ts = TypeSpec::builder("SortedList", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_type_param(tp)
        .add_where_constraint(
            TypeName::primitive("T"),
            vec![TypeName::primitive("IComparable")],
        )
        .add_field(
            FieldSpec::builder("items", TypeName::primitive("List<T>"))
                .visibility(Visibility::Private)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("Add")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("void"))
                .add_param(ParameterSpec::new("item", TypeName::primitive("T")).unwrap())
                .body(CodeBlock::of("items.Add(item);\nitems.Sort();", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("SortedList.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/generic_class.cs", &output);
}

#[test]
fn test_struct() {
    let ts = TypeSpec::builder("Point", TypeKind::Struct)
        .visibility(Visibility::Public)
        .add_field(
            FieldSpec::builder("X", TypeName::primitive("double"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("Y", TypeName::primitive("double"))
                .visibility(Visibility::Public)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Point.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/struct.cs", &output);
}

#[test]
fn test_enum() {
    let ts = TypeSpec::builder("Direction", TypeKind::Enum)
        .visibility(Visibility::Public)
        .add_variant(EnumVariantSpec::new("North").unwrap())
        .add_variant(
            EnumVariantSpec::builder("South")
                .value(CodeBlock::of("1", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("East")
                .value(CodeBlock::of("2", ()).unwrap())
                .build()
                .unwrap(),
        )
        .add_variant(
            EnumVariantSpec::builder("West")
                .value(CodeBlock::of("3", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Direction.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/enum.cs", &output);
}

#[test]
fn test_annotation_bracket_syntax() {
    use sigil_stitch::spec::annotation_spec::AnnotationSpec;

    let ts = TypeSpec::builder("Entity", TypeKind::Class)
        .visibility(Visibility::Public)
        .annotate(AnnotationSpec::new("Serializable"))
        .add_field(
            FieldSpec::builder("id", TypeName::primitive("int"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    let file = FileSpec::builder_with("Entity.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();
    assert!(
        output.contains("[Serializable]"),
        "C# annotations should use bracket syntax, got:\n{output}"
    );
    assert!(
        !output.contains("@Serializable"),
        "should NOT use @-prefix annotation style, got:\n{output}"
    );
}

#[test]
fn test_where_clause_multiple() {
    let ts = TypeSpec::builder("Mapper", TypeKind::Class)
        .visibility(Visibility::Public)
        .add_type_param(TypeParamSpec::new("TIn"))
        .add_type_param(TypeParamSpec::new("TOut"))
        .add_where_constraint(
            TypeName::primitive("TIn"),
            vec![TypeName::primitive("IConvertible")],
        )
        .add_where_constraint(
            TypeName::primitive("TOut"),
            vec![
                TypeName::primitive("IConvertible"),
                TypeName::primitive("new()"),
            ],
        )
        .add_method(
            FunSpec::builder("Map")
                .visibility(Visibility::Public)
                .returns(TypeName::primitive("TOut"))
                .add_param(ParameterSpec::new("input", TypeName::primitive("TIn")).unwrap())
                .body(CodeBlock::of("throw new NotImplementedException();", ()).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Mapper.cs", CSharp::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("csharp/where_clause_multiple.cs", &output);
}
