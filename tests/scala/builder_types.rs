use sigil_stitch::code_block::CodeBlock;
use sigil_stitch::lang::scala::Scala;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::spec::where_spec::{TypeParamKind, TypeParamSpec};
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_case_class() {
    let ts = TypeSpec::builder("User", TypeKind::Struct)
        .doc("A user case class.")
        .add_primary_constructor_param(
            ParameterSpec::new("name", TypeName::primitive("String")).unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::new("age", TypeName::primitive("Int")).unwrap(),
        )
        .add_primary_constructor_param(
            ParameterSpec::new("email", TypeName::primitive("String")).unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("User.scala", Scala::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/case_class.scala", &output);
}

#[test]
fn test_trait_with_type_param() {
    let tp = TypeParamSpec::new("T");

    let ts = TypeSpec::builder("Repository", TypeKind::Trait)
        .add_type_param(tp)
        .doc("Generic data repository.")
        .add_method(
            FunSpec::builder("findById")
                .returns(TypeName::primitive("Option[T]"))
                .add_param(ParameterSpec::new("id", TypeName::primitive("String")).unwrap())
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("save")
                .add_param(ParameterSpec::new("entity", TypeName::primitive("T")).unwrap())
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Repository.scala", Scala::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/trait_with_type_param.scala", &output);
}

#[test]
fn test_class_extends() {
    let base = TypeName::importable("com.example.base", "BaseService");
    let serial = TypeName::importable("com.example.serial", "Serializable");

    let body = CodeBlock::of("true", ()).unwrap();
    let ts = TypeSpec::builder("AdminService", TypeKind::Class)
        .extends(base)
        .extends(serial)
        .add_method(
            FunSpec::builder("isAdmin")
                .returns(TypeName::primitive("Boolean"))
                .body(body)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("AdminService.scala", Scala::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/class_extends.scala", &output);
}

#[test]
fn test_class_extends_multiple() {
    let ts = TypeSpec::builder("Worker", TypeKind::Class)
        .extends(TypeName::primitive("Actor"))
        .extends(TypeName::primitive("Logging"))
        .extends(TypeName::primitive("Serializable"))
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Worker.scala", Scala::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/class_extends_multiple.scala", &output);
}

#[test]
fn test_enum() {
    let ts = TypeSpec::builder("Color", TypeKind::Enum)
        .doc("Supported colors.")
        .add_variant(EnumVariantSpec::new("Red").unwrap())
        .add_variant(EnumVariantSpec::new("Green").unwrap())
        .add_variant(EnumVariantSpec::new("Blue").unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Color.scala", Scala::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/enum.scala", &output);
}

#[test]
fn test_hkt_type_param() {
    let tp_f = TypeParamSpec::new("F").with_kind(TypeParamKind::Constructor1);
    let tp_a = TypeParamSpec::new("A");

    let body = CodeBlock::of("???", ()).unwrap();
    let fun = FunSpec::builder("traverse")
        .add_type_param(tp_f)
        .add_type_param(tp_a)
        .returns(TypeName::primitive("F[List[A]]"))
        .add_param(ParameterSpec::new("list", TypeName::primitive("List[A]")).unwrap())
        .add_param(ParameterSpec::new("f", TypeName::primitive("A => F[A]")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Traverse.scala", Scala::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/hkt_type_param.scala", &output);
}

#[test]
fn test_bounded_type_param() {
    let tp = TypeParamSpec::new("T").with_bound(TypeName::primitive("Comparable[T]"));

    let body = CodeBlock::of("if (a.compareTo(b) >= 0) a else b", ()).unwrap();
    let fun = FunSpec::builder("max")
        .add_type_param(tp)
        .returns(TypeName::primitive("T"))
        .add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("T")).unwrap())
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Max.scala", Scala::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/bounded_type_param.scala", &output);
}

#[test]
fn test_abstract_class() {
    let desc_body = CodeBlock::of("getClass.getSimpleName", ()).unwrap();

    let ts = TypeSpec::builder("Shape", TypeKind::Class)
        .doc("Abstract shape.")
        .is_abstract()
        .add_method(
            FunSpec::builder("describe")
                .returns(TypeName::primitive("String"))
                .body(desc_body)
                .build()
                .unwrap(),
        )
        .add_method(
            FunSpec::builder("area")
                .is_abstract()
                .returns(TypeName::primitive("Double"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Shape.scala", Scala::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/abstract_class.scala", &output);
}

#[test]
fn test_context_bound() {
    let body = CodeBlock::of("implicitly[Ordering[T]].compare(a, b)", ()).unwrap();
    let fun = FunSpec::builder("sortedPair")
        .add_type_param(TypeParamSpec::new("T").with_context_bound(TypeName::primitive("Ordering")))
        .add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("T")).unwrap())
        .returns(TypeName::generic(
            TypeName::primitive("Tuple2"),
            vec![TypeName::primitive("T"), TypeName::primitive("T")],
        ))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("sorted.scala", Scala::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/context_bound.scala", &output);
}

#[test]
fn test_newtype() {
    let ts = TypeSpec::builder("Meters", TypeKind::Newtype)
        .extends(TypeName::primitive("Double"))
        .build()
        .unwrap();

    let file = FileSpec::builder_with("meters.scala", Scala::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/newtype.scala", &output);
}

#[test]
fn test_multiple_context_bounds() {
    let body = CodeBlock::of("implicitly[Ordering[T]].compare(a, b)", ()).unwrap();
    let fun = FunSpec::builder("compare")
        .add_type_param(
            TypeParamSpec::new("T")
                .with_context_bound(TypeName::primitive("Ordering"))
                .with_context_bound(TypeName::primitive("Numeric")),
        )
        .add_param(ParameterSpec::new("a", TypeName::primitive("T")).unwrap())
        .add_param(ParameterSpec::new("b", TypeName::primitive("T")).unwrap())
        .returns(TypeName::primitive("Int"))
        .body(body)
        .build()
        .unwrap();

    let file = FileSpec::builder_with("compare.scala", Scala::new())
        .add_function(fun)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("scala/multiple_context_bounds.scala", &output);
}
