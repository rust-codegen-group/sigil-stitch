//! C#-owned generic-constraint validation and ordering.

use std::borrow::Cow;

use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ConstraintPlacement {
    Primary,
    Type,
    Constructor,
    Anti,
}

pub(crate) fn merged_constraint_bounds(
    parameter: &TypeParamSpec,
    constraints: &[WhereConstraint],
) -> Vec<TypeName> {
    let explicit = constraints
        .iter()
        .filter(|constraint| constraint.parameter_subject_name() == Some(parameter.name()))
        .flat_map(|constraint| constraint.bounds());
    let mut bounds = Vec::new();
    for bound in parameter
        .bounds()
        .iter()
        .chain(parameter.context_bounds())
        .chain(explicit)
    {
        if let Some(existing) = bounds
            .iter_mut()
            .find(|existing| same_constraint_type(existing, bound))
        {
            merge_constraint_presentation(existing, bound);
        } else {
            bounds.push(bound.clone());
        }
    }
    bounds.sort_by_key(constraint_placement);
    bounds
}

pub(crate) fn validate_constraint_bounds(
    parameter: &TypeParamSpec,
    constraints: &[WhereConstraint],
) -> Result<(), &'static str> {
    let bounds = merged_constraint_bounds(parameter, constraints);
    validate_merged_constraint_bounds(&bounds)
}

pub(crate) fn validate_type_constraint_bounds(
    parameter: &TypeParamSpec,
    constraints: &[WhereConstraint],
) -> Result<(), &'static str> {
    let bounds = merged_constraint_bounds(parameter, constraints);
    validate_merged_constraint_bounds(&bounds)?;
    if bounds.iter().any(is_default_constraint) {
        return Err("C# type declarations cannot use the default constraint");
    }
    Ok(())
}

pub(crate) fn validate_function_constraint_context(
    parameter: &TypeParamSpec,
    constraints: &[WhereConstraint],
    is_override: bool,
) -> Result<(), &'static str> {
    let bounds = merged_constraint_bounds(parameter, constraints);
    if !is_override && bounds.iter().any(is_default_constraint) {
        return Err("C# default constraints require an override method");
    }
    if is_override && bounds.iter().any(|bound| !is_override_constraint(bound)) {
        return Err("C# override methods may specify only class, struct, or default constraints");
    }
    Ok(())
}

fn validate_merged_constraint_bounds(bounds: &[TypeName]) -> Result<(), &'static str> {
    if bounds.iter().any(invalid_constraint_type_shape) {
        return Err(
            "C# generic constraints require an interface, non-sealed class, or type parameter",
        );
    }

    for (index, bound) in bounds.iter().enumerate() {
        if bounds[index + 1..]
            .iter()
            .any(|other| nullable_counterparts(bound, other))
        {
            return Err(
                "C# cannot constrain one type parameter by both nullable and non-nullable forms of the same type",
            );
        }
    }

    let primary = bounds
        .iter()
        .filter_map(|bound| primary_constraint(bound))
        .collect::<Vec<_>>();
    if primary.len() > 1 {
        return Err(
            "C# type parameters accept at most one class, class?, struct, unmanaged, notnull, or default constraint",
        );
    }

    if bounds.iter().any(is_default_constraint) && bounds.len() != 1 {
        return Err("C# default must be the only constraint on its type parameter");
    }

    let has_constructor = bounds
        .iter()
        .any(|bound| constraint_placement(bound) == ConstraintPlacement::Constructor);
    if has_constructor
        && primary
            .iter()
            .any(|constraint| matches!(*constraint, "struct" | "unmanaged"))
    {
        return Err("C# new() constraints cannot be combined with struct or unmanaged");
    }

    let has_ref_struct_anti_constraint = bounds
        .iter()
        .any(|bound| constraint_placement(bound) == ConstraintPlacement::Anti);
    if has_ref_struct_anti_constraint
        && primary
            .iter()
            .any(|constraint| matches!(*constraint, "class" | "class?"))
    {
        return Err("C# allows ref struct cannot be combined with class or class?");
    }

    Ok(())
}

fn constraint_placement(type_name: &TypeName) -> ConstraintPlacement {
    match terminal_constraint_spelling(type_name).as_deref() {
        Some("class" | "class?" | "struct" | "unmanaged" | "notnull" | "default") => {
            ConstraintPlacement::Primary
        }
        Some("new()") => ConstraintPlacement::Constructor,
        Some("allows ref struct") => ConstraintPlacement::Anti,
        _ => ConstraintPlacement::Type,
    }
}

fn primary_constraint(type_name: &TypeName) -> Option<&str> {
    match terminal_constraint_spelling(type_name).as_deref() {
        Some("class") => Some("class"),
        Some("class?") => Some("class?"),
        Some("struct") => Some("struct"),
        Some("unmanaged") => Some("unmanaged"),
        Some("notnull") => Some("notnull"),
        Some("default") => Some("default"),
        _ => None,
    }
}

fn is_default_constraint(type_name: &TypeName) -> bool {
    terminal_constraint_spelling(type_name).as_deref() == Some("default")
}

fn is_override_constraint(type_name: &TypeName) -> bool {
    matches!(
        terminal_constraint_spelling(type_name).as_deref(),
        Some("class" | "struct" | "default")
    )
}

fn invalid_constraint_type_shape(type_name: &TypeName) -> bool {
    let type_name = match type_name {
        TypeName::Optional(inner)
            if matches!(inner.as_ref(), TypeName::Optional(_))
                || terminal_constraint_spelling(inner).is_some_and(|spelling| {
                    spelling.ends_with('?')
                        || matches!(
                            spelling.as_ref(),
                            "struct"
                                | "unmanaged"
                                | "notnull"
                                | "default"
                                | "new()"
                                | "allows ref struct"
                        )
                }) =>
        {
            return true;
        }
        TypeName::Optional(inner) => inner,
        type_name => type_name,
    };
    if matches!(type_name, TypeName::Pointer(_) | TypeName::Tuple(_)) {
        return true;
    }
    matches!(
        terminal_constraint_spelling(type_name).as_deref(),
        Some(
            "bool"
                | "byte"
                | "sbyte"
                | "char"
                | "decimal"
                | "double"
                | "float"
                | "int"
                | "uint"
                | "nint"
                | "nuint"
                | "long"
                | "ulong"
                | "short"
                | "ushort"
                | "string"
                | "object"
                | "dynamic"
                | "void"
        )
    )
}

fn nullable_counterparts(left: &TypeName, right: &TypeName) -> bool {
    fn one_way(nullable: &TypeName, non_nullable: &TypeName) -> bool {
        match nullable {
            TypeName::Optional(inner) => same_constraint_type(inner, non_nullable),
            TypeName::Primitive(spelling) | TypeName::Raw(spelling) => {
                spelling.strip_suffix('?').is_some_and(|base| {
                    matches!(
                        non_nullable,
                        TypeName::Primitive(other) | TypeName::Raw(other) if other == base
                    )
                })
            }
            _ => false,
        }
    }

    one_way(left, right) || one_way(right, left)
}

fn terminal_constraint_spelling(type_name: &TypeName) -> Option<Cow<'_, str>> {
    match type_name {
        TypeName::Primitive(spelling) | TypeName::Raw(spelling) => {
            Some(Cow::Borrowed(spelling.as_str()))
        }
        TypeName::Optional(inner) => {
            terminal_constraint_spelling(inner).map(|spelling| Cow::Owned(format!("{spelling}?")))
        }
        _ => None,
    }
}

fn merge_constraint_presentation(existing: &mut TypeName, duplicate: &TypeName) {
    match (existing, duplicate) {
        (
            TypeName::Importable {
                is_type_only: existing_type_only,
                qualified: existing_qualified,
                ..
            },
            TypeName::Importable {
                is_type_only: duplicate_type_only,
                qualified: duplicate_qualified,
                ..
            },
        ) => {
            *existing_type_only &= duplicate_type_only;
            *existing_qualified |= duplicate_qualified;
        }
        (TypeName::Array(existing), TypeName::Array(duplicate))
        | (TypeName::ReadonlyArray(existing), TypeName::ReadonlyArray(duplicate))
        | (TypeName::Optional(existing), TypeName::Optional(duplicate)) => {
            merge_constraint_presentation(existing, duplicate);
        }
        (
            TypeName::Generic {
                base: existing_base,
                params: existing_params,
            },
            TypeName::Generic {
                base: duplicate_base,
                params: duplicate_params,
            },
        ) => {
            merge_constraint_presentation(existing_base, duplicate_base);
            merge_constraint_presentations(existing_params, duplicate_params);
        }
        (TypeName::Tuple(existing), TypeName::Tuple(duplicate)) => {
            merge_constraint_presentations(existing, duplicate);
        }
        (
            TypeName::AssociatedType {
                base: existing_base,
                qualifier: None,
                ..
            },
            TypeName::AssociatedType {
                base: duplicate_base,
                qualifier: None,
                ..
            },
        ) => merge_constraint_presentation(existing_base, duplicate_base),
        _ => {}
    }
}

fn merge_constraint_presentations(existing: &mut [TypeName], duplicate: &[TypeName]) {
    for (existing, duplicate) in existing.iter_mut().zip(duplicate) {
        merge_constraint_presentation(existing, duplicate);
    }
}

fn same_terminal_constraint_spelling(left: &TypeName, right: &TypeName) -> bool {
    matches!(
        (
            terminal_constraint_spelling(left),
            terminal_constraint_spelling(right)
        ),
        (Some(left), Some(right)) if left == right
    )
}

fn same_constraint_type(left: &TypeName, right: &TypeName) -> bool {
    if same_terminal_constraint_spelling(left, right) {
        return true;
    }
    match (left, right) {
        (
            TypeName::Importable {
                module: left_module,
                name: left_name,
                ..
            },
            TypeName::Importable {
                module: right_module,
                name: right_name,
                ..
            },
        ) => left_module == right_module && left_name == right_name,
        (TypeName::Array(left), TypeName::Array(right))
        | (TypeName::ReadonlyArray(left), TypeName::ReadonlyArray(right))
        | (TypeName::Optional(left), TypeName::Optional(right)) => {
            same_constraint_type(left, right)
        }
        (
            TypeName::Generic {
                base: left_base,
                params: left_params,
            },
            TypeName::Generic {
                base: right_base,
                params: right_params,
            },
        ) => {
            same_constraint_type(left_base, right_base)
                && same_constraint_types(left_params, right_params)
        }
        (TypeName::Tuple(left), TypeName::Tuple(right)) => same_constraint_types(left, right),
        (
            TypeName::AssociatedType {
                base: left_base,
                qualifier: None,
                member: left_member,
            },
            TypeName::AssociatedType {
                base: right_base,
                qualifier: None,
                member: right_member,
            },
        ) => left_member == right_member && same_constraint_type(left_base, right_base),
        _ => left == right,
    }
}

fn same_constraint_types(left: &[TypeName], right: &[TypeName]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| same_constraint_type(left, right))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge_duplicate(first: TypeName, duplicate: TypeName) -> TypeName {
        let parameter = TypeParamSpec::new("T")
            .with_bound(first)
            .with_context_bound(duplicate);
        let mut merged = merged_constraint_bounds(&parameter, &[]);
        assert_eq!(merged.len(), 1);
        merged.pop().unwrap()
    }

    #[test]
    fn supported_compound_constraints_merge_import_presentation_recursively() {
        let imported = || TypeName::importable_type("constraints", "Bound");
        let qualified = || TypeName::qualified("constraints", "Bound");

        assert_eq!(
            merge_duplicate(TypeName::array(imported()), TypeName::array(qualified())),
            TypeName::array(qualified())
        );
        assert_eq!(
            merge_duplicate(
                TypeName::readonly_array(imported()),
                TypeName::readonly_array(qualified())
            ),
            TypeName::readonly_array(qualified())
        );
        assert_eq!(
            merge_duplicate(
                TypeName::optional(imported()),
                TypeName::optional(qualified())
            ),
            TypeName::optional(qualified())
        );
        assert_eq!(
            merge_duplicate(
                TypeName::tuple(vec![imported(), TypeName::primitive("int")]),
                TypeName::tuple(vec![qualified(), TypeName::raw("int")]),
            ),
            TypeName::tuple(vec![qualified(), TypeName::primitive("int")])
        );
        assert_eq!(
            merge_duplicate(
                TypeName::member_type(imported(), "Nested"),
                TypeName::member_type(qualified(), "Nested"),
            ),
            TypeName::member_type(qualified(), "Nested")
        );
        assert_eq!(
            merge_duplicate(
                TypeName::string_literal("value"),
                TypeName::string_literal("value"),
            ),
            TypeName::string_literal("value")
        );
    }

    #[test]
    fn distinct_compound_constraints_remain_distinct() {
        let parameter = TypeParamSpec::new("T")
            .with_bound(TypeName::array(TypeName::importable("first", "Bound")))
            .with_context_bound(TypeName::array(TypeName::importable("second", "Bound")))
            .with_context_bound(TypeName::tuple(vec![TypeName::primitive("A")]))
            .with_context_bound(TypeName::tuple(vec![
                TypeName::primitive("A"),
                TypeName::primitive("B"),
            ]))
            .with_context_bound(TypeName::member_type(TypeName::primitive("Owner"), "First"))
            .with_context_bound(TypeName::member_type(
                TypeName::primitive("Owner"),
                "Second",
            ));

        assert_eq!(merged_constraint_bounds(&parameter, &[]).len(), 6);
    }
}
