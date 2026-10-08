//! Structured type interpolation uses the same lowering and import pipeline as builders.

use sigil_stitch::lang::haskell::Haskell;
use sigil_stitch::prelude::*;

fn render(filename: &str, block: CodeBlock, width: usize) -> String {
    FileSpec::builder(filename)
        .add_code(block)
        .build()
        .unwrap()
        .render(width)
        .unwrap()
}

fn boxed_parameter() -> TypeName {
    TypeName::application(
        TypeName::importable_type("./models", "Box"),
        vec![TypeArgument::Single(TypeName::parameter("T"))],
    )
}

#[test]
fn type_interpolation_preserves_complete_callable_and_nested_imports() {
    let callable = TypeName::callable(
        vec![
            CallableParam::Single {
                name: Some("value".into()),
                type_name: boxed_parameter(),
                presence: CallableParamPresence::Optional,
            },
            CallableParam::Repeated {
                name: Some("rest".into()),
                element_type: TypeName::importable_type("./models", "Item"),
            },
        ],
        TypeName::importable_type("./results", "Result"),
    );
    let quoted = sigil_quote!(TypeScript { type Handler = $T(callable.clone()); }).unwrap();
    let mut builder = CodeBlock::builder();
    builder.add_statement("type Handler = %T", (callable,));
    let built = builder.build().unwrap();
    for width in [8, 120] {
        let output = render("handler.ts", quoted.clone(), width);
        assert_eq!(output, render("handler.ts", built.clone(), width));
        assert!(
            output
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains("(value?: Box<T>, ...rest: Item[]) => Result"),
            "{output}"
        );
        assert!(
            output.contains("import type { Box, Item } from './models'"),
            "{output}"
        );
        assert!(
            output.contains("import type { Result } from './results'"),
            "{output}"
        );
    }
}

#[test]
fn type_join_preserves_compound_values_and_resolves_peer_imports() {
    let types = [
        boxed_parameter(),
        TypeName::application(
            TypeName::importable_type("./legacy", "Box"),
            vec![TypeArgument::Single(TypeName::parameter("U"))],
        ),
    ];
    let quoted = sigil_quote!(TypeScript {
        type Items = [$T_join(", ", &types)];
    })
    .unwrap();
    for width in [8, 120] {
        let output = render("items.ts", quoted.clone(), width);
        assert!(
            output.contains("type Items = [Box<T>, LegacyBox<U>]"),
            "{output}"
        );
        assert!(
            output.contains("import type { Box as LegacyBox } from './legacy'"),
            "{output}"
        );
        assert!(
            output.contains("import type { Box } from './models'"),
            "{output}"
        );
    }
}

#[test]
fn cpp_interpolation_keeps_complete_pack_expansion_patterns() {
    let pair = TypeName::application(
        TypeName::primitive("Pair"),
        vec![
            TypeArgument::Single(TypeName::parameter("Ts")),
            TypeArgument::Single(TypeName::parameter("Us")),
        ],
    );
    let tuple = TypeName::application(
        TypeName::primitive("Tuple"),
        vec![TypeArgument::Expansion { pattern: pair }],
    );
    let block = sigil_quote!(Cpp { using Items = $T(tuple); }).unwrap();
    assert!(render("items.cpp", block, 80).contains("Tuple<Pair<Ts, Us>...>"));
}

#[test]
fn haskell_interpolation_collects_and_qualifies_type_operator_imports() {
    let operator = |module| {
        TypeName::application(
            TypeName::importable_type(module, "+"),
            vec![
                TypeArgument::Single(TypeName::parameter("n")),
                TypeArgument::Single(TypeName::parameter("m")),
            ],
        )
    };
    let block = sigil_quote!(Haskell {
        left :: $T(operator("GHC.TypeNats"))
        right :: $T(operator("GHC.TypeLits"))
    })
    .unwrap();
    let output = render("indexed.hs", block, 80);
    assert!(
        output.contains("import GHC.TypeNats (type (+))"),
        "{output}"
    );
    assert!(
        output.contains("import qualified GHC.TypeLits (type (+))"),
        "{output}"
    );
    assert!(output.contains("left :: (+) n m"), "{output}");
    assert!(output.contains("right :: (GHC.TypeLits.+) n m"), "{output}");
}

#[test]
fn unsupported_type_in_join_fails_during_complete_file_rendering() {
    let types = vec![TypeName::application(
        TypeName::primitive("Box"),
        vec![TypeArgument::Expansion {
            pattern: TypeName::parameter("Ts"),
        }],
    )];
    let block = sigil_quote!(TypeScript { type Bad = $T_join(", ", types); }).unwrap();
    let error = FileSpec::builder("bad.ts")
        .add_code(block)
        .build()
        .unwrap()
        .render(80)
        .unwrap_err();
    assert!(error.to_string().contains("expansion"), "{error}");
}

#[test]
fn declaration_splices_preserve_generic_bindings_and_closed_sum_types() {
    let sum = ClosedSumSpec::builder("Outcome")
        .add_generic_param(GenericParamSpec::single("a").unwrap())
        .add_case(ClosedSumCaseSpec::positional("Value", vec![TypeName::parameter("a")]).unwrap())
        .build()
        .unwrap();
    let blocks = sum.emit(&Haskell::new()).unwrap();
    let quoted = sigil_quote!(Haskell { $C_each(blocks.clone()) }).unwrap();
    let mut builder = CodeBlock::builder();
    for block in blocks {
        builder.add_code(block);
    }
    builder.add_line();
    assert_eq!(
        render("outcome.hs", quoted, 80),
        render("outcome.hs", builder.build().unwrap(), 80)
    );
}

#[test]
fn code_and_literal_splices_keep_compound_type_references() {
    let fragment = CodeFragment::of("type Value = %T", (boxed_parameter(),)).unwrap();
    let block = CodeBlock::of("type Value = %T", (boxed_parameter(),)).unwrap();
    let literal = sigil_quote!(TypeScript { $L(fragment); }).unwrap();
    let code = sigil_quote!(TypeScript { $C(block); }).unwrap();
    for width in [8, 120] {
        let output = render("value.ts", literal.clone(), width);
        assert_eq!(output, render("value.ts", code.clone(), width));
        assert!(output.contains("type Value = Box<T>;"), "{output}");
        assert!(
            output.contains("import type { Box } from './models'"),
            "{output}"
        );
    }
}
