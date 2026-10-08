//! Emit complete compiler-acceptance fixtures through public generation APIs.
//!
//! Run with the output directory as the sole argument. Compiler invocation and
//! positive/negative pairing are owned by examples/source_acceptance.rs.

use sigil_stitch::prelude::*;
use std::error::Error;
use std::path::Path;

fn application(base: TypeName, arguments: Vec<TypeName>) -> TypeName {
    TypeName::application(
        base,
        arguments.into_iter().map(TypeArgument::Single).collect(),
    )
}

fn tuple_pack(name: &str) -> TypeName {
    TypeName::application(
        TypeName::primitive("std::tuple"),
        vec![TypeArgument::Expansion {
            pattern: TypeName::parameter(name),
        }],
    )
}

fn cpp(negative: bool) -> Result<String, SigilStitchError> {
    let pair = application(
        TypeName::primitive("Pair"),
        vec![TypeName::parameter("As"), TypeName::parameter("Bs")],
    );
    let pairs = TypeName::application(
        TypeName::primitive("std::tuple"),
        vec![TypeArgument::Expansion { pattern: pair }],
    );
    let function = |name: &str, result: TypeName, body: &str| {
        FunSpec::builder(name)
            .add_generic_param(GenericParamSpec::pack("As")?)
            .add_generic_param(GenericParamSpec::pack("Bs")?)
            .add_param(ParameterSpec::new("left", tuple_pack("As"))?)
            .add_param(ParameterSpec::new("right", tuple_pack("Bs"))?)
            .returns(result)
            .body(CodeBlock::of(body, ())?)
            .build()
    };
    let mut use_sites = CodeBlock::builder();
    use_sites.add("int main() {\n", ());
    use_sites.add(
        "combine(std::tuple<int, double>{}, std::tuple<char>{});\n",
        (),
    );
    use_sites.add("combine(std::tuple<>{}, std::tuple<>{});\n", ());
    use_sites.add("auto empty = pairs(std::tuple<>{}, std::tuple<>{});\n", ());
    use_sites.add(
        "%T bundle;\n",
        (TypeName::application(TypeName::primitive("Bundle"), vec![]),),
    );
    if negative {
        use_sites.add("auto invalid = pairs(std::tuple<int, double>{}, std::tuple<char>{}); // acceptance-failure\n", ());
    } else {
        use_sites.add(
            "auto valid = pairs(std::tuple<int, double>{}, std::tuple<char, bool>{});\n",
            (),
        );
    }
    use_sites.add("}\n", ());
    let output = FileSpec::builder("packs.cpp")
        .header(CodeBlock::of(
            "#include <tuple>\ntemplate<class A, class B> struct Pair {};\n",
            (),
        )?)
        .add_type(
            TypeSpec::builder("Bundle", TypeKind::Class)
                .add_generic_param(GenericParamSpec::pack("Ts")?)
                .build()?,
        )
        .add_function(function("combine", TypeName::primitive("void"), "return;")?)
        .add_function(function("pairs", pairs, "return {};")?)
        .add_code(use_sites.build()?)
        .build()?
        .render(120)?;
    assert!(output.contains("template<class... As, class... Bs>"));
    assert!(output.contains("Pair<As, Bs>..."));
    assert!(output.contains("Bundle<>"));
    Ok(output)
}

fn haskell(negative: bool) -> Result<String, SigilStitchError> {
    let natural = |name| {
        GenericParamSpec::new(
            name,
            GenericParamDomain::Single {
                kind: Some(KindExpr::Named(TypeName::importable("GHC.TypeNats", "Nat"))),
            },
        )
    };
    let array = |index| {
        application(
            TypeName::primitive("Array"),
            vec![index, TypeName::parameter("a")],
        )
    };
    let sum = application(
        TypeName::importable_type("GHC.TypeNats", "+"),
        vec![TypeName::parameter("n"), TypeName::parameter("m")],
    );
    let function = FunSpec::builder("append")
        .add_generic_param(natural("n")?)
        .add_generic_param(natural("m")?)
        .add_generic_param(GenericParamSpec::new(
            "a",
            GenericParamDomain::Single {
                kind: Some(KindExpr::Type),
            },
        )?)
        .add_param(ParameterSpec::new("left", array(TypeName::parameter("n")))?)
        .add_param(ParameterSpec::new(
            "right",
            array(TypeName::parameter("m")),
        )?)
        .returns(array(sum))
        .body(CodeBlock::of("undefined", ())?)
        .build()?;
    let use_sites = if negative {
        "data Array (n :: Nat) a = Array\ninvalid :: Array 6 Int\ninvalid = append (undefined :: Array 2 Int) (undefined :: Array 3 Int) -- acceptance-failure\n"
    } else {
        "data Array (n :: Nat) a = Array\nvalid :: Array 5 Int\nvalid = append (undefined :: Array 2 Int) (undefined :: Array 3 Int)\n"
    };
    let output = FileSpec::builder("Indexed.hs")
        .header(CodeBlock::of("module Indexed where\n", ())?)
        .add_function(function)
        .add_code(CodeBlock::of(use_sites, ())?)
        .build()?
        .render(120)?;
    assert!(output.contains("Array ((+) n m) a"));
    assert!(output.contains("import GHC.TypeNats (Nat, type (+))"));
    Ok(output)
}

fn typescript(failure: Option<&str>) -> Result<String, SigilStitchError> {
    let callable = TypeName::callable(
        vec![
            CallableParam::Single {
                name: Some("text".into()),
                type_name: TypeName::primitive("string"),
                presence: CallableParamPresence::Required,
            },
            CallableParam::Single {
                name: Some("count".into()),
                type_name: TypeName::primitive("number"),
                presence: CallableParamPresence::Optional,
            },
        ],
        TypeName::primitive("void"),
    );
    let mut use_sites = CodeBlock::builder();
    use_sites.add(
        "declare const handler: Handler;\nhandler('ok');\nhandler('ok', 2);\n",
        (),
    );
    match failure {
        Some("missing") => {
            use_sites.add("handler(); // acceptance-failure\n", ());
        }
        Some("type") => {
            use_sites.add("handler(17); // acceptance-failure\n", ());
        }
        None => {}
        Some(_) => unreachable!(),
    }
    let output = FileSpec::builder("callable.ts")
        .add_type(
            TypeSpec::builder("Handler", TypeKind::TypeAlias)
                .extends(callable)
                .build()?,
        )
        .add_code(use_sites.build()?)
        .build()?
        .render(120)?;
    assert!(output.contains("text: string, count?: number"));
    Ok(output)
}

fn scala(negative: bool) -> Result<String, SigilStitchError> {
    let function = FunSpec::builder("wrap")
        .add_generic_param(GenericParamSpec::new(
            "F",
            GenericParamDomain::Single {
                kind: Some(KindExpr::Constructor {
                    parameters: vec![KindExpr::Type],
                    result: Box::new(KindExpr::Type),
                }),
            },
        )?)
        .add_generic_param(GenericParamSpec::single("A")?)
        .add_param(ParameterSpec::new(
            "value",
            application(TypeName::parameter("F"), vec![TypeName::parameter("A")]),
        )?)
        .returns(TypeName::parameter("A"))
        .body(CodeBlock::of("???", ())?)
        .build()?;
    let use_sites = if negative {
        "val invalid = wrap[Int, Int](???) // acceptance-failure\n"
    } else {
        "val valid: Int = wrap[List, Int](List(1))\n"
    };
    let output = FileSpec::builder("Kinds.scala")
        .add_function(function)
        .add_code(CodeBlock::of(use_sites, ())?)
        .build()?
        .render(120)?;
    assert!(output.contains("F[_]"));
    Ok(output)
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .ok_or("an output directory is required")?;
    let directory = Path::new(&directory);
    std::fs::create_dir_all(directory)?;
    for (name, source) in [
        ("cpp_positive.cpp", cpp(false)?),
        ("cpp_mismatched.cpp", cpp(true)?),
        ("haskell_positive.hs", haskell(false)?),
        ("haskell_wrong_index.hs", haskell(true)?),
        ("typescript_positive.ts", typescript(None)?),
        ("typescript_missing.ts", typescript(Some("missing"))?),
        ("typescript_wrong_type.ts", typescript(Some("type"))?),
        ("scala_positive.scala", scala(false)?),
        ("scala_wrong_kind.scala", scala(true)?),
    ] {
        std::fs::write(directory.join(name), source)?;
    }
    Ok(())
}
