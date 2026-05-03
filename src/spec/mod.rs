/// Annotation specifications (e.g., `@Override`, `#[derive(...)]`).
pub mod annotation_spec;
/// Trait for spec types that can emit themselves as top-level file members.
pub mod emittable;
/// Enum variant specifications.
pub mod enum_variant_spec;
/// Struct field and class property specifications.
pub mod field_spec;
/// Top-level file orchestrator with import resolution.
pub mod file_spec;
/// Function and method specifications.
pub mod fun_spec;
/// Import specification types for resolved imports.
pub mod import_spec;
/// Visibility, type kind, and declaration context modifiers.
pub mod modifiers;
/// Function parameter specifications.
pub mod parameter_spec;
/// Multi-file project generation orchestrator.
pub mod project_spec;
/// Computed property specifications (getters/setters).
pub mod property_spec;
/// Type declaration specifications (struct, class, interface, trait, enum).
pub mod type_spec;
