use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::spec::enum_variant_spec::EnumVariantSpec;
use sigil_stitch::spec::field_spec::FieldSpec;
use sigil_stitch::spec::file_spec::FileSpec;
use sigil_stitch::spec::fun_spec::FunSpec;
use sigil_stitch::spec::modifiers::TypeKind;
use sigil_stitch::spec::parameter_spec::ParameterSpec;
use sigil_stitch::spec::type_spec::TypeSpec;
use sigil_stitch::type_name::TypeName;

use super::golden;

#[test]
fn test_data_type_with_record() {
    let ts = TypeSpec::builder("Person", TypeKind::Struct)
        .doc("A person record type.")
        .add_field(
            FieldSpec::builder("personName", TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("personAge", TypeName::primitive("Int"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("personEmail", TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Person.hs", Haskell::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/data_type_record.hs", &output);
}

#[test]
fn test_enum_type() {
    let ts = TypeSpec::builder("Color", TypeKind::Enum)
        .doc("Supported colors.")
        .add_variant(EnumVariantSpec::new("Red").unwrap())
        .add_variant(EnumVariantSpec::new("Green").unwrap())
        .add_variant(EnumVariantSpec::new("Blue").unwrap())
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Color.hs", Haskell::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/enum_type.hs", &output);
}

#[test]
fn test_type_alias() {
    let ts = TypeSpec::builder("Name", TypeKind::TypeAlias)
        .extends(TypeName::primitive("String"))
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Name.hs", Haskell::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/type_alias.hs", &output);
}

#[test]
fn test_data_with_deriving() {
    let ts = TypeSpec::builder("Color", TypeKind::Enum)
        .add_variant(EnumVariantSpec::new("Red").unwrap())
        .add_variant(EnumVariantSpec::new("Green").unwrap())
        .add_variant(EnumVariantSpec::new("Blue").unwrap())
        .implements(TypeName::primitive("Show"))
        .implements(TypeName::primitive("Eq"))
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Color.hs", Haskell::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/data_with_deriving.hs", &output);
}

#[test]
fn test_newtype() {
    let ts = TypeSpec::builder("Meters", TypeKind::Newtype)
        .extends(TypeName::primitive("Int"))
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Meters.hs", Haskell::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/newtype.hs", &output);
}

#[test]
fn test_type_class_via_type_spec() {
    let ts = TypeSpec::builder("Printable", TypeKind::Trait)
        .doc("Things that can be printed.")
        .add_method(
            FunSpec::builder("prettyPrint")
                .add_param(ParameterSpec::new("x", TypeName::primitive("a")).unwrap())
                .returns(TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Printable.hs", Haskell::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/type_class_spec.hs", &output);
}

#[test]
fn test_data_with_deriving_record() {
    let ts = TypeSpec::builder("Person", TypeKind::Struct)
        .add_field(
            FieldSpec::builder("personName", TypeName::primitive("String"))
                .build()
                .unwrap(),
        )
        .add_field(
            FieldSpec::builder("personAge", TypeName::primitive("Int"))
                .build()
                .unwrap(),
        )
        .implements(TypeName::primitive("Show"))
        .implements(TypeName::primitive("Eq"))
        .build()
        .unwrap();

    let file = FileSpec::builder_with("Person.hs", Haskell::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/data_deriving_record.hs", &output);
}

#[test]
fn test_newtype_deriving() {
    let ts = TypeSpec::builder("UserId", TypeKind::Newtype)
        .extends(TypeName::primitive("Int"))
        .implements(TypeName::primitive("Show"))
        .implements(TypeName::primitive("Eq"))
        .implements(TypeName::primitive("Ord"))
        .build()
        .unwrap();

    let file = FileSpec::builder_with("UserId.hs", Haskell::new())
        .add_type(ts)
        .build()
        .unwrap();
    let output = file.render(80).unwrap();

    golden::assert_golden("haskell/builder_newtype_deriving.hs", &output);
}
