use pretty::BoxDoc;

use crate::lang::CodeLang;
use crate::lang::config::GenericSyntaxConfig;
use crate::type_name::{
    AssociatedTypeStyle, FunctionPresentation, GenericApplicationStyle, TypeName, TypePresentation,
};

pub(crate) fn render_presentation(
    pres: &TypePresentation<'_>,
    inner_docs: Vec<BoxDoc<'static, ()>>,
    gs: &GenericSyntaxConfig<'_>,
) -> BoxDoc<'static, ()> {
    match pres {
        TypePresentation::GenericWrap { name } => {
            let sep = BoxDoc::text(",").append(BoxDoc::softline());
            let params = BoxDoc::intersperse(inner_docs, sep);
            BoxDoc::text(name.to_string())
                .append(BoxDoc::text(gs.open.to_string()))
                .append(params.nest(2).group())
                .append(BoxDoc::text(gs.close.to_string()))
        }
        TypePresentation::Prefix { prefix } => {
            debug_assert_eq!(inner_docs.len(), 1);
            let inner = inner_docs.into_iter().next().unwrap_or_else(BoxDoc::nil);
            BoxDoc::text(prefix.to_string()).append(inner)
        }
        TypePresentation::Postfix { suffix } => {
            debug_assert_eq!(inner_docs.len(), 1);
            let inner = inner_docs.into_iter().next().unwrap_or_else(BoxDoc::nil);
            inner.append(BoxDoc::text(suffix.to_string()))
        }
        TypePresentation::Surround { prefix, suffix } => {
            debug_assert_eq!(inner_docs.len(), 1);
            let inner = inner_docs.into_iter().next().unwrap_or_else(BoxDoc::nil);
            BoxDoc::text(prefix.to_string())
                .append(inner)
                .append(BoxDoc::text(suffix.to_string()))
        }
        TypePresentation::Delimited { open, sep, close } => {
            let separator = BoxDoc::text(sep.to_string());
            let body = BoxDoc::intersperse(inner_docs, separator);
            BoxDoc::text(open.to_string())
                .append(body.nest(2).group())
                .append(BoxDoc::text(close.to_string()))
        }
        TypePresentation::Infix { sep } => {
            let sep_trimmed = sep.trim_start();
            let separator = BoxDoc::softline().append(BoxDoc::text(sep_trimmed.to_string()));
            BoxDoc::intersperse(inner_docs, separator).group()
        }
    }
}

fn render_function_presentation(
    pres: &FunctionPresentation<'_>,
    param_docs: Vec<BoxDoc<'static, ()>>,
    return_doc: BoxDoc<'static, ()>,
) -> BoxDoc<'static, ()> {
    if pres.curried {
        let mut all = param_docs;
        all.push(return_doc);
        let sep = BoxDoc::text(pres.arrow.to_string());
        return BoxDoc::intersperse(all, sep);
    }

    let sep = BoxDoc::text(pres.params_sep.to_string());
    let params_doc = BoxDoc::intersperse(param_docs, sep);
    let params_block = BoxDoc::text(pres.params_open.to_string())
        .append(params_doc.nest(2).group())
        .append(BoxDoc::text(pres.params_close.to_string()));

    let keyword_doc = if pres.keyword.is_empty() {
        BoxDoc::nil()
    } else {
        BoxDoc::text(pres.keyword.to_string())
    };

    let signature = if pres.return_first {
        return_doc.append(keyword_doc).append(params_block)
    } else {
        keyword_doc
            .append(params_block)
            .append(BoxDoc::text(pres.arrow.to_string()))
            .append(return_doc)
    };

    if pres.wrapper_open.is_empty() {
        signature
    } else {
        BoxDoc::text(pres.wrapper_open.to_string())
            .append(signature)
            .append(BoxDoc::text(pres.wrapper_close.to_string()))
    }
}

pub(crate) fn is_compound_type(t: &TypeName) -> bool {
    matches!(
        t,
        TypeName::Generic { .. }
            | TypeName::Union(_)
            | TypeName::Intersection(_)
            | TypeName::Function { .. }
            | TypeName::Tuple(_)
    )
}

pub fn to_doc<F>(tn: &TypeName, resolve: &F) -> BoxDoc<'static, ()>
where
    F: Fn(&str, &str) -> String,
{
    match tn {
        TypeName::Importable { module, name, .. } => {
            let display = resolve(module, name);
            BoxDoc::text(display)
        }
        TypeName::Primitive(name) => BoxDoc::text(name.clone()),
        TypeName::Raw(s) => BoxDoc::text(s.clone()),
        TypeName::Array(inner) => inner.to_doc(resolve).append(BoxDoc::text("[]")),
        TypeName::ReadonlyArray(inner) => BoxDoc::text("readonly ")
            .append(inner.to_doc(resolve))
            .append(BoxDoc::text("[]")),
        TypeName::Generic { base, params } => {
            let base_doc = base.to_doc(resolve);
            let params_docs: Vec<_> = params.iter().map(|p| p.to_doc(resolve)).collect();
            let sep = BoxDoc::text(",").append(BoxDoc::softline());
            let params_doc = BoxDoc::intersperse(params_docs, sep);
            base_doc
                .append(BoxDoc::text("<"))
                .append(params_doc.nest(2).group())
                .append(BoxDoc::text(">"))
        }
        TypeName::Union(members) => {
            let docs: Vec<_> = members.iter().map(|m| m.to_doc(resolve)).collect();
            let sep = BoxDoc::softline().append(BoxDoc::text("| "));
            BoxDoc::intersperse(docs, sep).group()
        }
        TypeName::Intersection(members) => {
            let docs: Vec<_> = members.iter().map(|m| m.to_doc(resolve)).collect();
            let sep = BoxDoc::softline().append(BoxDoc::text("& "));
            BoxDoc::intersperse(docs, sep).group()
        }
        TypeName::Pointer(inner) => BoxDoc::text("*").append(inner.to_doc(resolve)),
        TypeName::Slice(inner) => BoxDoc::text("[]").append(inner.to_doc(resolve)),
        TypeName::Map { key, value } => BoxDoc::text("map[")
            .append(key.to_doc(resolve))
            .append(BoxDoc::text("]"))
            .append(value.to_doc(resolve)),
        TypeName::Optional(inner) => {
            let inner_doc = inner.to_doc(resolve);
            inner_doc
                .append(BoxDoc::softline())
                .append(BoxDoc::text("| null"))
                .group()
        }
        TypeName::Tuple(elements) => {
            let docs: Vec<_> = elements.iter().map(|e| e.to_doc(resolve)).collect();
            if docs.is_empty() {
                return BoxDoc::text("()");
            }
            let sep = BoxDoc::text(",").append(BoxDoc::softline());
            BoxDoc::text("(")
                .append(BoxDoc::intersperse(docs, sep).nest(2).group())
                .append(BoxDoc::text(")"))
        }
        TypeName::Reference {
            inner,
            mutable,
            lifetime,
        } => {
            let mut prefix = String::from("&");
            if let Some(lt) = lifetime {
                prefix.push_str(lt);
                prefix.push(' ');
            }
            if *mutable {
                prefix.push_str("mut ");
            }
            BoxDoc::text(prefix).append(inner.to_doc(resolve))
        }
        TypeName::Function {
            params,
            return_type,
        } => {
            let params_docs: Vec<_> = params.iter().map(|p| p.to_doc(resolve)).collect();
            let sep = BoxDoc::text(",").append(BoxDoc::softline());
            let params_doc = BoxDoc::intersperse(params_docs, sep);
            BoxDoc::text("(")
                .append(params_doc.nest(2).group())
                .append(BoxDoc::text(") => "))
                .append(return_type.to_doc(resolve))
        }
        TypeName::AssociatedType {
            base,
            qualifier,
            member,
        } => {
            if let Some(qual) = qualifier {
                BoxDoc::text("<")
                    .append(base.to_doc(resolve))
                    .append(BoxDoc::text(" as "))
                    .append(qual.to_doc(resolve))
                    .append(BoxDoc::text(">::"))
                    .append(BoxDoc::text(member.clone()))
            } else {
                base.to_doc(resolve)
                    .append(BoxDoc::text("::"))
                    .append(BoxDoc::text(member.clone()))
            }
        }
        TypeName::ImplTrait { bounds } => {
            let docs: Vec<_> = bounds.iter().map(|b| b.to_doc(resolve)).collect();
            let sep = BoxDoc::text(" + ");
            BoxDoc::text("impl ").append(BoxDoc::intersperse(docs, sep))
        }
        TypeName::DynTrait { bounds } => {
            let docs: Vec<_> = bounds.iter().map(|b| b.to_doc(resolve)).collect();
            let sep = BoxDoc::text(" + ");
            BoxDoc::text("dyn ").append(BoxDoc::intersperse(docs, sep))
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => {
            debug_assert!(
                upper_bound.is_none() || lower_bound.is_none(),
                "Wildcard cannot have both upper and lower bounds"
            );
            if let Some(ub) = upper_bound {
                BoxDoc::text("? extends ").append(ub.to_doc(resolve))
            } else if let Some(lb) = lower_bound {
                BoxDoc::text("? super ").append(lb.to_doc(resolve))
            } else {
                BoxDoc::text("?")
            }
        }
    }
}

pub fn render(
    tn: &TypeName,
    width: usize,
    resolve: &impl Fn(&str, &str) -> String,
) -> Result<String, crate::error::SigilStitchError> {
    let doc = to_doc(tn, resolve);
    let mut buf = Vec::new();
    doc.render(width, &mut buf)
        .map_err(|e| crate::error::SigilStitchError::Render {
            context: "TypeName::render".to_string(),
            message: e.to_string(),
        })?;
    String::from_utf8(buf).map_err(|e| crate::error::SigilStitchError::Render {
        context: "TypeName::render UTF-8 conversion".to_string(),
        message: e.to_string(),
    })
}

pub fn to_doc_with_lang<F>(tn: &TypeName, resolve: &F, lang: &dyn CodeLang) -> BoxDoc<'static, ()>
where
    F: Fn(&str, &str) -> String,
{
    let tp = lang.type_presentation();
    let gs = lang.generic_syntax();

    match tn {
        TypeName::Generic { base, params } => {
            let base_doc = base.to_doc_with_lang(resolve, lang);
            let params_docs: Vec<_> = params
                .iter()
                .map(|p| p.to_doc_with_lang(resolve, lang))
                .collect();
            match gs.application_style {
                GenericApplicationStyle::Delimited => {
                    let sep = BoxDoc::text(",").append(BoxDoc::softline());
                    let params_doc = BoxDoc::intersperse(params_docs, sep);
                    base_doc
                        .append(BoxDoc::text(gs.open.to_string()))
                        .append(params_doc.nest(2).group())
                        .append(BoxDoc::text(gs.close.to_string()))
                }
                GenericApplicationStyle::PrefixJuxtaposition => {
                    let mut doc = base_doc;
                    for (i, param_doc) in params_docs.into_iter().enumerate() {
                        doc = doc.append(BoxDoc::text(" "));
                        if is_compound_type(&params[i]) {
                            doc = doc
                                .append(BoxDoc::text("("))
                                .append(param_doc)
                                .append(BoxDoc::text(")"));
                        } else {
                            doc = doc.append(param_doc);
                        }
                    }
                    doc
                }
                GenericApplicationStyle::PostfixJuxtaposition => {
                    if params_docs.len() == 1 {
                        params_docs
                            .into_iter()
                            .next()
                            .unwrap()
                            .append(BoxDoc::text(" "))
                            .append(base_doc)
                    } else {
                        let sep = BoxDoc::text(",").append(BoxDoc::softline());
                        let params_doc = BoxDoc::intersperse(params_docs, sep);
                        BoxDoc::text("(")
                            .append(params_doc.nest(2).group())
                            .append(BoxDoc::text(") "))
                            .append(base_doc)
                    }
                }
            }
        }
        TypeName::Array(inner) => {
            let inner_doc = inner.to_doc_with_lang(resolve, lang);
            render_presentation(&tp.array, vec![inner_doc], &gs)
        }
        TypeName::ReadonlyArray(inner) => {
            let inner_doc = inner.to_doc_with_lang(resolve, lang);
            if let Some(pres) = tp.readonly_array {
                render_presentation(&pres, vec![inner_doc], &gs)
            } else {
                let array_doc = render_presentation(&tp.array, vec![inner_doc], &gs);
                BoxDoc::text("readonly ").append(array_doc)
            }
        }
        TypeName::Union(members) => {
            let docs: Vec<_> = members
                .iter()
                .map(|m| m.to_doc_with_lang(resolve, lang))
                .collect();
            render_presentation(&tp.union, docs, &gs)
        }
        TypeName::Intersection(members) => {
            let docs: Vec<_> = members
                .iter()
                .map(|m| m.to_doc_with_lang(resolve, lang))
                .collect();
            render_presentation(&tp.intersection, docs, &gs)
        }
        TypeName::Pointer(inner) => {
            let inner_doc = inner.to_doc_with_lang(resolve, lang);
            render_presentation(&tp.pointer, vec![inner_doc], &gs)
        }
        TypeName::Slice(inner) => {
            let inner_doc = inner.to_doc_with_lang(resolve, lang);
            render_presentation(&tp.slice, vec![inner_doc], &gs)
        }
        TypeName::Map { key, value } => {
            let key_doc = key.to_doc_with_lang(resolve, lang);
            let value_doc = value.to_doc_with_lang(resolve, lang);
            render_presentation(&tp.map, vec![key_doc, value_doc], &gs)
        }
        TypeName::Optional(inner) => {
            let inner_doc = inner.to_doc_with_lang(resolve, lang);
            match &tp.optional {
                TypePresentation::Infix { .. } => {
                    let null_doc = BoxDoc::text(tp.optional_absent_literal.to_string());
                    render_presentation(&tp.optional, vec![inner_doc, null_doc], &gs)
                }
                _ => render_presentation(&tp.optional, vec![inner_doc], &gs),
            }
        }
        TypeName::Tuple(elements) => {
            let docs: Vec<_> = elements
                .iter()
                .map(|e| e.to_doc_with_lang(resolve, lang))
                .collect();
            render_presentation(&tp.tuple, docs, &gs)
        }
        TypeName::Reference {
            inner,
            mutable,
            lifetime,
        } => {
            let inner_doc = inner.to_doc_with_lang(resolve, lang);
            if let Some(lt) = lifetime {
                let mut prefix = String::from("&");
                prefix.push_str(lt);
                prefix.push(' ');
                if *mutable {
                    prefix.push_str("mut ");
                }
                BoxDoc::text(prefix).append(inner_doc)
            } else {
                let pres = if *mutable {
                    tp.reference_mut
                } else {
                    tp.reference
                };
                render_presentation(&pres, vec![inner_doc], &gs)
            }
        }
        TypeName::Function {
            params,
            return_type,
        } => {
            let param_docs: Vec<_> = params
                .iter()
                .map(|p| p.to_doc_with_lang(resolve, lang))
                .collect();
            let return_doc = return_type.to_doc_with_lang(resolve, lang);
            render_function_presentation(&tp.function, param_docs, return_doc)
        }
        TypeName::AssociatedType {
            base,
            qualifier,
            member,
        } => {
            let base_doc = base.to_doc_with_lang(resolve, lang);
            match tp.associated_type {
                AssociatedTypeStyle::QualifiedPath {
                    open,
                    as_kw,
                    close_sep,
                    simple_sep,
                } => {
                    if let Some(qual) = qualifier {
                        let qual_doc = qual.to_doc_with_lang(resolve, lang);
                        BoxDoc::text(open.to_string())
                            .append(base_doc)
                            .append(BoxDoc::text(as_kw.to_string()))
                            .append(qual_doc)
                            .append(BoxDoc::text(close_sep.to_string()))
                            .append(BoxDoc::text(member.clone()))
                    } else {
                        base_doc
                            .append(BoxDoc::text(simple_sep.to_string()))
                            .append(BoxDoc::text(member.clone()))
                    }
                }
                AssociatedTypeStyle::DotAccess => base_doc
                    .append(BoxDoc::text("."))
                    .append(BoxDoc::text(member.clone())),
                AssociatedTypeStyle::IndexAccess { open, close } => base_doc
                    .append(BoxDoc::text(open.to_string()))
                    .append(BoxDoc::text(member.clone()))
                    .append(BoxDoc::text(close.to_string())),
            }
        }
        TypeName::ImplTrait { bounds } => {
            let docs: Vec<_> = bounds
                .iter()
                .map(|b| b.to_doc_with_lang(resolve, lang))
                .collect();
            let sep = BoxDoc::text(tp.impl_trait.separator.to_string());
            BoxDoc::text(tp.impl_trait.keyword.to_string()).append(BoxDoc::intersperse(docs, sep))
        }
        TypeName::DynTrait { bounds } => {
            let docs: Vec<_> = bounds
                .iter()
                .map(|b| b.to_doc_with_lang(resolve, lang))
                .collect();
            let sep = BoxDoc::text(tp.dyn_trait.separator.to_string());
            BoxDoc::text(tp.dyn_trait.keyword.to_string()).append(BoxDoc::intersperse(docs, sep))
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => {
            debug_assert!(
                upper_bound.is_none() || lower_bound.is_none(),
                "Wildcard cannot have both upper and lower bounds"
            );
            if let Some(ub) = upper_bound {
                let ub_doc = ub.to_doc_with_lang(resolve, lang);
                BoxDoc::text(tp.wildcard.upper_keyword.to_string()).append(ub_doc)
            } else if let Some(lb) = lower_bound {
                let lb_doc = lb.to_doc_with_lang(resolve, lang);
                BoxDoc::text(tp.wildcard.lower_keyword.to_string()).append(lb_doc)
            } else {
                BoxDoc::text(tp.wildcard.unbounded.to_string())
            }
        }
        TypeName::Importable {
            module,
            name,
            qualified: true,
            ..
        } => {
            if let Some(sep) = lang.module_separator() {
                BoxDoc::text(format!("{module}{sep}{name}"))
            } else {
                let display = resolve(module, name);
                BoxDoc::text(display)
            }
        }
        _ => to_doc(tn, resolve),
    }
}
