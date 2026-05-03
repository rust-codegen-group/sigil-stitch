use crate::import::ImportRef;
use crate::type_name::TypeName;

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
        TypeName::Primitive(_) | TypeName::Raw(_) => {}
    }
}
