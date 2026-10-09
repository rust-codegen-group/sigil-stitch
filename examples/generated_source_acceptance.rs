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
            .returns(vec![result])
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
        .returns(vec![array(sum)])
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
        vec![TypeName::primitive("void")],
    );
    let mut use_sites = CodeBlock::builder();
    use_sites.add(
        "declare const handler: Handler;\nhandler('ok');\nhandler('ok', 2);\nconst completion: Promise<void> = complete();\n",
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
        .add_function(
            FunSpec::builder("complete")
                .is_async()
                .returns(vec![])
                .body(CodeBlock::of("return;", ())?)
                .build()?,
        )
        .add_type(
            TypeSpec::builder("Handler", TypeKind::TypeAlias)
                .extends(callable)
                .build()?,
        )
        .add_code(use_sites.build()?)
        .build()?
        .render(120)?;
    assert!(output.contains("text: string, count?: number"));
    assert!(output.contains("async function complete(): Promise<void>"));
    Ok(output)
}

fn go(negative: bool) -> Result<String, SigilStitchError> {
    let results = || {
        vec![
            TypeName::pointer(TypeName::importable("net/http", "Response")),
            TypeName::importable("time", "Time"),
        ]
    };
    let fetch_body = CodeBlock::of("return nil, %T{}", TypeName::importable("time", "Time"))?;
    let receiver = FunSpec::builder("Fetch")
        .returns(results())
        .receiver(ParameterSpec::new("self", TypeName::primitive("Client"))?)
        .body(fetch_body.clone())
        .build()?;
    let mut uses = CodeBlock::builder();
    uses.add(
        "var emptyCallback %T = Empty\n",
        TypeName::callable(vec![], vec![]),
    );
    uses.add(
        "var singleCallback %T = Single\n",
        TypeName::callable(vec![], vec![TypeName::primitive("int")]),
    );
    uses.add(
        "var pairCallback %T = Pair\n",
        TypeName::callable(
            vec![],
            vec![TypeName::primitive("int"), TypeName::primitive("bool")],
        ),
    );
    uses.add(
        "var fetchCallback %T = Fetch\n",
        TypeName::callable(vec![], results()),
    );
    uses.add("var _ Fetcher = Client{}\nfunc use() {\nemptyCallback()\n_ = singleCallback()\nvalue, ok := pairCallback()\n_, _ = value, ok\nresponse, timestamp := fetchCallback()\n_, _ = response, timestamp\n", ());
    if negative {
        uses.add(
            "var scalar int = Pair() // acceptance-failure\n_ = scalar\n",
            (),
        );
    }
    uses.add("}\n", ());
    let output = FileSpec::builder("returns.go")
        .header(CodeBlock::of("package acceptance\n", ())?)
        .add_type(TypeSpec::builder("Client", TypeKind::Struct).build()?)
        .add_type(
            TypeSpec::builder("Fetcher", TypeKind::Interface)
                .add_method(FunSpec::builder("Fetch").returns(results()).build()?)
                .build()?,
        )
        .add_function(
            FunSpec::builder("Empty")
                .returns(vec![])
                .body(CodeBlock::of("return", ())?)
                .build()?,
        )
        .add_function(
            FunSpec::builder("Single")
                .returns(vec![TypeName::primitive("int")])
                .body(CodeBlock::of("return 1", ())?)
                .build()?,
        )
        .add_function(
            FunSpec::builder("Pair")
                .returns(vec![
                    TypeName::primitive("int"),
                    TypeName::primitive("bool"),
                ])
                .body(CodeBlock::of("return 1, true", ())?)
                .build()?,
        )
        .add_function(
            FunSpec::builder("Fetch")
                .returns(results())
                .body(fetch_body)
                .build()?,
        )
        .add_function(receiver)
        .add_code(uses.build()?)
        .build()?
        .render(120)?;
    for expected in [
        "func Empty() {",
        "func Single() int {",
        "func Pair() (int, bool) {",
        "func Fetch() (*http.Response, time.Time) {",
        "func (self Client) Fetch() (*http.Response, time.Time) {",
        "Fetch() (*http.Response, time.Time)",
        "var fetchCallback func() (*http.Response, time.Time) = Fetch",
        "\"net/http\"",
        "\"time\"",
    ] {
        assert!(output.contains(expected), "missing {expected}:\n{output}");
    }
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
        .returns(vec![TypeName::parameter("A")])
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
        ("go_positive.go", go(false)?),
        ("go_single_value.go", go(true)?),
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
