//! Modifier types for structural specs: visibility, declaration context, type kind.

/// Where a declaration appears: top-level file scope vs. inside a type body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarationContext {
    /// Top-level (file scope): e.g., `export class` (TS), `pub struct` (Rust).
    TopLevel,
    /// Inside a type body: e.g., `private name: string` (TS), `pub name: String` (Rust).
    Member,
}

/// Visibility level for declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    /// No explicit visibility keyword.
    #[default]
    Inherited,
    /// `pub` (Rust), `export` (TS top-level), `public` (TS member).
    Public,
    /// `private` (TS member). Rust default is private, no keyword needed.
    Private,
    /// `protected` (TS member). No Rust equivalent.
    Protected,
    /// `pub(crate)` (Rust). No TS equivalent.
    PublicCrate,
    /// `pub(super)` (Rust). No TS equivalent.
    PublicSuper,
}

/// The kind of type declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    /// TS: `class`, Rust: `struct` (struct-with-methods pattern).
    Class,
    /// Rust: `struct` (data-only). TS: maps to `interface`.
    Struct,
    /// TS: `interface`, Rust: `trait`.
    Interface,
    /// Rust: `trait`, TS: `interface`.
    Trait,
    /// Both languages: `enum`.
    Enum,
}

/// How a `PropertySpec` renders: accessor methods or inline field body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyStyle {
    /// Emit getter/setter as separate accessor methods: `get name(): T { ... }` (TS/JS).
    Accessor,
    /// Emit as a field with inline getter/setter body: `var x: T { get { } set { } }` (Swift/Kotlin).
    Field,
}

/// Where a constructor delegation call (`super(...)` / `this(...)`) is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructorDelegationStyle {
    /// Delegation is the first statement in the constructor body.
    /// Used by TS, JS, Java, Dart, Swift, Python, C++.
    Body,
    /// Delegation appears between the parameter list and the body.
    /// Used by Kotlin: `constructor(x: Int) : this(x, 0) { ... }`.
    Signature,
}

/// Modifier flags for a declaration.
#[derive(Debug, Clone, Default)]
pub struct Modifiers {
    /// Visibility level for the declaration.
    pub visibility: Visibility,
    /// Whether the declaration is static.
    pub is_static: bool,
    /// Whether the declaration is abstract.
    pub is_abstract: bool,
    /// Whether the declaration is readonly.
    pub is_readonly: bool,
    /// Whether the declaration is async.
    pub is_async: bool,
    /// Whether the declaration is an override.
    pub is_override: bool,
    /// Whether the declaration is a constructor.
    pub is_constructor: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_default() {
        let vis = Visibility::default();
        assert_eq!(vis, Visibility::Inherited);
    }

    #[test]
    fn test_modifiers_default() {
        let m = Modifiers::default();
        assert_eq!(m.visibility, Visibility::Inherited);
        assert!(!m.is_static);
        assert!(!m.is_abstract);
        assert!(!m.is_readonly);
        assert!(!m.is_async);
        assert!(!m.is_override);
        assert!(!m.is_constructor);
    }
}
