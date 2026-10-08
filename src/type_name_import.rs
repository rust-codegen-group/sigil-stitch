use crate::import::ImportRef;
use crate::type_name::TypeName;

#[expect(
    deprecated,
    reason = "one semantic path also accepts released compatibility inputs"
)]
pub fn collect_imports(tn: &TypeName, out: &mut Vec<ImportRef>) {
    match tn {
        TypeName::Importable {
            qualified: true, ..
        } => {}
        TypeName::Importable {
            module,
            name,
            is_type_only,
            alias,
            ..
        } => {
            out.push(ImportRef {
                module: module.clone(),
                name: name.clone(),
                is_type_only: *is_type_only,
                alias: alias.clone(),
            });
        }
        TypeName::Array(inner)
        | TypeName::ReadonlyArray(inner)
        | TypeName::Pointer(inner)
        | TypeName::Slice(inner)
        | TypeName::Optional(inner) => {
            collect_imports(inner, out);
        }
        TypeName::Reference { inner, .. } => {
            collect_imports(inner, out);
        }
        TypeName::Generic { base, params } => {
            collect_imports(base, out);
            for p in params {
                collect_imports(p, out);
            }
        }
        TypeName::Application { base, arguments } => {
            collect_imports(base, out);
            for argument in arguments {
                match argument {
                    crate::spec::where_spec::TypeArgument::Single(value)
                    | crate::spec::where_spec::TypeArgument::Expansion { pattern: value } => {
                        collect_imports(value, out)
                    }
                }
            }
        }
        TypeName::Union(members) | TypeName::Intersection(members) | TypeName::Tuple(members) => {
            for m in members {
                collect_imports(m, out);
            }
        }
        TypeName::Map { key, value } => {
            collect_imports(key, out);
            collect_imports(value, out);
        }
        TypeName::Function {
            params,
            return_type,
        } => {
            for p in params {
                collect_imports(p, out);
            }
            collect_imports(return_type, out);
        }
        TypeName::Callable {
            parameters,
            return_type,
        } => {
            for parameter in parameters {
                match parameter {
                    crate::spec::where_spec::CallableParam::Single { type_name, .. } => {
                        collect_imports(type_name, out)
                    }
                    crate::spec::where_spec::CallableParam::Repeated { element_type, .. } => {
                        collect_imports(element_type, out)
                    }
                    crate::spec::where_spec::CallableParam::Expansion { pattern, .. } => {
                        collect_imports(pattern, out)
                    }
                }
            }
            collect_imports(return_type, out);
        }
        TypeName::AssociatedType {
            base, qualifier, ..
        } => {
            collect_imports(base, out);
            if let Some(q) = qualifier {
                collect_imports(q, out);
            }
        }
        TypeName::ImplTrait { bounds } | TypeName::DynTrait { bounds } => {
            for b in bounds {
                collect_imports(b, out);
            }
        }
        TypeName::Wildcard {
            upper_bound,
            lower_bound,
        } => {
            if let Some(ub) = upper_bound {
                collect_imports(ub, out);
            }
            if let Some(lb) = lower_bound {
                collect_imports(lb, out);
            }
        }
        TypeName::Primitive(_)
        | TypeName::Parameter(_)
        | TypeName::Raw(_)
        | TypeName::StringLiteral(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::where_spec::{CallableParam, CallableParamPresence, TypeArgument};

    #[test]
    fn modern_walker_preserves_all_callable_segments_and_expansion_patterns() {
        let ty = TypeName::callable(
            vec![
                CallableParam::Single {
                    name: None,
                    type_name: TypeName::importable("Inputs", "Input"),
                    presence: CallableParamPresence::Required,
                },
                CallableParam::Repeated {
                    name: None,
                    element_type: TypeName::importable("Items", "Item"),
                },
                CallableParam::Expansion {
                    name: None,
                    pattern: TypeName::application(
                        TypeName::importable("Containers", "Container"),
                        vec![TypeArgument::Expansion {
                            pattern: TypeName::importable("Patterns", "Pattern"),
                        }],
                    ),
                },
            ],
            TypeName::importable("Results", "Result"),
        );
        let mut imports = Vec::new();
        collect_imports(&ty, &mut imports);
        assert_eq!(
            imports
                .iter()
                .map(|import| import.name.as_str())
                .collect::<Vec<_>>(),
            ["Input", "Item", "Container", "Pattern", "Result"]
        );
    }
}
