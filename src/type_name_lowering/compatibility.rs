use crate::code_block::CodeBlock;
use crate::error::SigilStitchError;
use crate::lang::RendererLang;
#[expect(deprecated, reason = "frozen 0.6.8 compatibility interpreter")]
use crate::type_name::{AssociatedTypeStyle, GenericApplicationStyle, TypeName, TypePresentation};

use super::structure::{
    concat, delimited_soft, join, join_soft, literal, name, qualified, surround, terminal,
};

#[expect(deprecated, reason = "frozen 0.6.8 compatibility interpreter")]
pub(crate) fn lower<L: RendererLang + ?Sized>(
    lang: &L,
    type_name: &TypeName,
) -> Result<CodeBlock, SigilStitchError> {
    let config = lang.type_presentation();
    let generics = lang.generic_syntax();

    fn presentation(
        value: &TypePresentation<'_>,
        items: Vec<CodeBlock>,
        generic_open: &str,
        generic_close: &str,
    ) -> CodeBlock {
        match value {
            TypePresentation::GenericWrap { name: wrapper } => {
                let open = if generic_open.is_empty() && !wrapper.is_empty() {
                    " "
                } else {
                    generic_open
                };
                concat([
                    literal(*wrapper),
                    delimited_soft(open, items, ",", generic_close),
                ])
            }
            TypePresentation::Prefix { prefix } => {
                concat([literal(*prefix), items.into_iter().next().unwrap()])
            }
            TypePresentation::Postfix { suffix } => {
                concat([items.into_iter().next().unwrap(), literal(*suffix)])
            }
            TypePresentation::Surround { prefix, suffix } => concat([
                literal(*prefix),
                items.into_iter().next().unwrap(),
                literal(*suffix),
            ]),
            TypePresentation::Delimited { open, sep, close } => {
                surround(open, join(items, sep), close)
            }
            TypePresentation::Infix { sep } => join_soft(items, sep.trim_start()),
        }
    }

    let recurse = |value: &TypeName| lower(lang, value);
    let lowered = match type_name {
        TypeName::Importable {
            module,
            name: imported_name,
            qualified: true,
            ..
        } => match lang.module_separator() {
            Some(separator) => qualified(module, separator, imported_name),
            None => name(imported_name.clone()),
        },
        TypeName::Importable { .. } | TypeName::Primitive(_) | TypeName::Raw(_) => {
            terminal(type_name)
        }
        TypeName::StringLiteral(_) => {
            return Err(SigilStitchError::UnsupportedTypeName {
                language: lang.file_extension().to_string(),
                context: "root".to_string(),
                reason: "the 0.6.8 compatibility lowerer does not support string singleton types"
                    .to_string(),
            });
        }
        TypeName::Generic { base, params } => {
            let base = recurse(base)?;
            let params = params.iter().map(recurse).collect::<Result<Vec<_>, _>>()?;
            match generics.application_style {
                GenericApplicationStyle::Delimited => concat([
                    base,
                    delimited_soft(generics.open, params, ",", generics.close),
                ]),
                GenericApplicationStyle::PrefixJuxtaposition => {
                    let mut parts = vec![base];
                    for (parameter, original) in params.into_iter().zip(params_type(type_name)) {
                        parts.push(literal(" "));
                        if crate::type_name_render::is_compound_type(original) {
                            parts.push(surround("(", parameter, ")"));
                        } else {
                            parts.push(parameter);
                        }
                    }
                    concat(parts)
                }
                GenericApplicationStyle::PostfixJuxtaposition if params.len() == 1 => {
                    concat([params.into_iter().next().unwrap(), literal(" "), base])
                }
                GenericApplicationStyle::PostfixJuxtaposition => {
                    concat([delimited_soft("(", params, ",", ")"), literal(" "), base])
                }
            }
        }
        TypeName::Array(inner) => presentation(
            &config.array,
            vec![recurse(inner)?],
            generics.open,
            generics.close,
        ),
        TypeName::ReadonlyArray(inner) => match config.readonly_array {
            Some(value) => {
                presentation(&value, vec![recurse(inner)?], generics.open, generics.close)
            }
            None => concat([
                literal("readonly "),
                presentation(
                    &config.array,
                    vec![recurse(inner)?],
                    generics.open,
                    generics.close,
                ),
            ]),
        },
        TypeName::Union(members) => presentation(
            &config.union,
            members.iter().map(recurse).collect::<Result<_, _>>()?,
            generics.open,
            generics.close,
        ),
        TypeName::Intersection(members) => presentation(
            &config.intersection,
            members.iter().map(recurse).collect::<Result<_, _>>()?,
            generics.open,
            generics.close,
        ),
        TypeName::Pointer(inner) => presentation(
            &config.pointer,
            vec![recurse(inner)?],
            generics.open,
            generics.close,
        ),
        TypeName::Slice(inner) => presentation(
            &config.slice,
            vec![recurse(inner)?],
            generics.open,
            generics.close,
        ),
        TypeName::Map { key, value } => presentation(
            &config.map,
            vec![recurse(key)?, recurse(value)?],
            generics.open,
            generics.close,
        ),
        TypeName::Optional(inner) => match config.optional {
            TypePresentation::Infix { .. } => presentation(
                &config.optional,
                vec![recurse(inner)?, literal(config.optional_absent_literal)],
                generics.open,
                generics.close,
            ),
            _ => presentation(
                &config.optional,
                vec![recurse(inner)?],
                generics.open,
                generics.close,
            ),
        },
        TypeName::Tuple(elements) => presentation(
            &config.tuple,
            elements.iter().map(recurse).collect::<Result<_, _>>()?,
            generics.open,
            generics.close,
        ),
        TypeName::Reference {
            inner,
            mutable,
            lifetime,
        } => {
            let inner = recurse(inner)?;
            if let Some(lifetime) = lifetime {
                concat([
                    literal(format!(
                        "&{lifetime} {}",
                        if *mutable { "mut " } else { "" }
                    )),
                    inner,
                ])
            } else {
                presentation(
                    if *mutable {
                        &config.reference_mut
                    } else {
                        &config.reference
                    },
                    vec![inner],
                    generics.open,
                    generics.close,
                )
            }
        }
        TypeName::Function {
            params,
            return_type,
        } => {
            let function = config.function;
            let mut params = params.iter().map(recurse).collect::<Result<Vec<_>, _>>()?;
            let return_type = recurse(return_type)?;
            if function.curried {
                params.push(return_type);
                join(params, function.arrow)
            } else {
                let params = surround(
                    function.params_open,
                    join(params, function.params_sep),
                    function.params_close,
                );
                let signature = if function.return_first {
                    concat([return_type, literal(function.keyword), params])
                } else {
                    concat([
                        literal(function.keyword),
                        params,
                        literal(function.arrow),
                        return_type,
                    ])
                };
                surround(function.wrapper_open, signature, function.wrapper_close)
            }
        }
        TypeName::AssociatedType {
            base,
            qualifier,
            member,
        } => {
            let base = recurse(base)?;
            match config.associated_type {
                AssociatedTypeStyle::QualifiedPath {
                    open,
                    as_kw,
                    close_sep,
                    simple_sep,
                } => match qualifier {
                    Some(qualifier) => concat([
                        literal(open),
                        base,
                        literal(as_kw),
                        recurse(qualifier)?,
                        literal(close_sep),
                        name(member.clone()),
                    ]),
                    None => concat([base, literal(simple_sep), name(member.clone())]),
                },
                AssociatedTypeStyle::DotAccess => {
                    concat([base, literal("."), name(member.clone())])
                }
                AssociatedTypeStyle::IndexAccess { open, close } => {
                    concat([base, literal(open), name(member.clone()), literal(close)])
                }
            }
        }
        TypeName::ImplTrait { bounds } => concat([
            literal(config.impl_trait.keyword),
            join(
                bounds.iter().map(recurse).collect::<Result<_, _>>()?,
                config.impl_trait.separator,
            ),
        ]),
        TypeName::DynTrait { bounds } => concat([
            literal(config.dyn_trait.keyword),
            join(
                bounds.iter().map(recurse).collect::<Result<_, _>>()?,
                config.dyn_trait.separator,
            ),
        ]),
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => match (upper_bound, lower_bound) {
            (Some(bound), None) => {
                concat([literal(config.wildcard.upper_keyword), recurse(bound)?])
            }
            (None, Some(bound)) => {
                concat([literal(config.wildcard.lower_keyword), recurse(bound)?])
            }
            (None, None) => literal(config.wildcard.unbounded),
            (Some(_), Some(_)) => unreachable!("intrinsic validation rejects dual bounds"),
        },
    };
    Ok(lowered)
}

fn params_type(type_name: &TypeName) -> &[TypeName] {
    match type_name {
        TypeName::Generic { params, .. } => params,
        _ => &[],
    }
}

#[cfg(test)]
#[expect(
    deprecated,
    reason = "tests freeze the 0.6.8 compatibility interpreter"
)]
mod tests {
    use super::*;
    use crate::lang::CodeLang;
    use crate::lang::config::{GenericSyntaxConfig, TypePresentationConfig};
    use crate::type_name::FunctionPresentation;

    #[derive(Debug, Clone, Copy)]
    enum CompatibilityStyle {
        Default,
        PrefixGenerics,
        PostfixGenerics,
        NonDefaultPresentations,
        Curried,
        IndexedAssociation,
    }

    #[derive(Debug)]
    struct CompatibilityLang(CompatibilityStyle);

    impl RendererLang for CompatibilityLang {
        fn file_extension(&self) -> &str {
            "compat"
        }

        fn line_comment_prefix(&self) -> &str {
            "//"
        }

        fn module_separator(&self) -> Option<&str> {
            (!matches!(self.0, CompatibilityStyle::IndexedAssociation)).then_some("::")
        }

        fn generic_syntax(&self) -> GenericSyntaxConfig<'_> {
            let mut config = GenericSyntaxConfig {
                application_style: match self.0 {
                    CompatibilityStyle::PrefixGenerics => {
                        GenericApplicationStyle::PrefixJuxtaposition
                    }
                    CompatibilityStyle::PostfixGenerics => {
                        GenericApplicationStyle::PostfixJuxtaposition
                    }
                    _ => GenericApplicationStyle::Delimited,
                },
                ..GenericSyntaxConfig::default()
            };
            if matches!(self.0, CompatibilityStyle::NonDefaultPresentations) {
                config.open = "";
                config.close = "";
            }
            config
        }

        fn type_presentation(&self) -> TypePresentationConfig<'_> {
            let mut config = TypePresentationConfig::default();
            match self.0 {
                CompatibilityStyle::NonDefaultPresentations => {
                    config.array = TypePresentation::GenericWrap { name: "Array" };
                    config.readonly_array = Some(TypePresentation::Surround {
                        prefix: "const ",
                        suffix: "&",
                    });
                    config.optional = TypePresentation::Postfix { suffix: "?" };
                    config.function = FunctionPresentation {
                        keyword: " Function",
                        return_first: true,
                        wrapper_open: "<",
                        wrapper_close: ">",
                        ..FunctionPresentation::default()
                    };
                    config.associated_type = AssociatedTypeStyle::DotAccess;
                }
                CompatibilityStyle::Curried => {
                    config.function.curried = true;
                    config.function.arrow = " -> ";
                }
                CompatibilityStyle::IndexedAssociation => {
                    config.associated_type = AssociatedTypeStyle::IndexAccess {
                        open: "[\"",
                        close: "\"]",
                    };
                }
                CompatibilityStyle::Default
                | CompatibilityStyle::PrefixGenerics
                | CompatibilityStyle::PostfixGenerics => {}
            }
            config
        }
    }

    impl CodeLang for CompatibilityLang {}

    fn render(lang: &CompatibilityLang, type_name: TypeName) -> String {
        CodeBlock::of("%T", (type_name,))
            .unwrap()
            .render_standalone(lang, 240)
            .unwrap()
    }

    #[test]
    fn default_interpreter_covers_every_supported_type_shape() {
        let lang = CompatibilityLang(CompatibilityStyle::Default);
        let value = || TypeName::primitive("Value");
        let cases = [
            (TypeName::qualified("pkg", "Item"), "pkg::Item"),
            (TypeName::importable("pkg", "Item"), "Item"),
            (TypeName::raw("Target.Type"), "Target.Type"),
            (
                TypeName::generic(value(), vec![TypeName::primitive("Item")]),
                "Value<Item>",
            ),
            (TypeName::array(value()), "Value[]"),
            (TypeName::readonly_array(value()), "readonly Value[]"),
            (
                TypeName::union(vec![value(), TypeName::primitive("Other")]),
                "Value | Other",
            ),
            (
                TypeName::intersection(vec![value(), TypeName::primitive("Other")]),
                "Value & Other",
            ),
            (TypeName::pointer(value()), "*Value"),
            (TypeName::slice(value()), "[]Value"),
            (
                TypeName::map(TypeName::primitive("Key"), value()),
                "Map<Key, Value>",
            ),
            (TypeName::optional(value()), "Value | null"),
            (
                TypeName::tuple(vec![value(), TypeName::primitive("Other")]),
                "(Value, Other)",
            ),
            (TypeName::reference(value()), "Value"),
            (TypeName::reference_mut(value()), "Value"),
            (
                TypeName::reference_with_lifetime(value(), "'a"),
                "&'a Value",
            ),
            (
                TypeName::Reference {
                    inner: Box::new(value()),
                    mutable: true,
                    lifetime: Some("'a".to_string()),
                },
                "&'a mut Value",
            ),
            (
                TypeName::function(
                    vec![value(), TypeName::primitive("Other")],
                    TypeName::primitive("Result"),
                ),
                "(Value, Other) => Result",
            ),
            (
                TypeName::associated_type(value(), Some(TypeName::primitive("Trait")), "Item"),
                "<Value as Trait>::Item",
            ),
            (TypeName::member_type(value(), "Item"), "Value::Item"),
            (
                TypeName::impl_trait(vec![value(), TypeName::primitive("Other")]),
                "impl Value + Other",
            ),
            (
                TypeName::dyn_trait(vec![value(), TypeName::primitive("Other")]),
                "dyn Value + Other",
            ),
            (TypeName::wildcard(), "?"),
            (TypeName::wildcard_extends(value()), "? extends Value"),
            (TypeName::wildcard_super(value()), "? super Value"),
        ];

        for (type_name, expected) in cases {
            assert_eq!(render(&lang, type_name), expected);
        }
    }

    #[test]
    fn non_default_compatibility_presentations_keep_their_frozen_branches() {
        let prefix = CompatibilityLang(CompatibilityStyle::PrefixGenerics);
        assert_eq!(
            render(
                &prefix,
                TypeName::generic(
                    TypeName::primitive("Either"),
                    vec![
                        TypeName::primitive("Left"),
                        TypeName::optional(TypeName::primitive("Right")),
                    ],
                ),
            ),
            "Either Left (Right | null)"
        );

        let postfix = CompatibilityLang(CompatibilityStyle::PostfixGenerics);
        assert_eq!(
            render(
                &postfix,
                TypeName::generic(
                    TypeName::primitive("Box"),
                    vec![TypeName::primitive("Value")],
                ),
            ),
            "Value Box"
        );
        assert_eq!(
            render(
                &postfix,
                TypeName::generic(
                    TypeName::primitive("Pair"),
                    vec![TypeName::primitive("Left"), TypeName::primitive("Right")],
                ),
            ),
            "(Left, Right) Pair"
        );

        let non_default = CompatibilityLang(CompatibilityStyle::NonDefaultPresentations);
        assert_eq!(
            render(&non_default, TypeName::array(TypeName::primitive("Value"))),
            "Array Value"
        );
        assert_eq!(
            render(
                &non_default,
                TypeName::readonly_array(TypeName::primitive("Value")),
            ),
            "const Value&"
        );
        assert_eq!(
            render(
                &non_default,
                TypeName::optional(TypeName::primitive("Value")),
            ),
            "Value?"
        );
        assert_eq!(
            render(
                &non_default,
                TypeName::function(
                    vec![TypeName::primitive("Value")],
                    TypeName::primitive("Result"),
                ),
            ),
            "<Result Function(Value)>"
        );
        assert_eq!(
            render(
                &non_default,
                TypeName::member_type(TypeName::primitive("Value"), "Item"),
            ),
            "Value.Item"
        );

        let curried = CompatibilityLang(CompatibilityStyle::Curried);
        assert_eq!(
            render(
                &curried,
                TypeName::function(
                    vec![TypeName::primitive("Left"), TypeName::primitive("Right")],
                    TypeName::primitive("Result"),
                ),
            ),
            "Left -> Right -> Result"
        );

        let indexed = CompatibilityLang(CompatibilityStyle::IndexedAssociation);
        assert_eq!(
            render(
                &indexed,
                TypeName::member_type(TypeName::primitive("Value"), "Item"),
            ),
            "Value[\"Item\"]"
        );
        assert_eq!(render(&indexed, TypeName::qualified("pkg", "Item")), "Item");
    }
}
