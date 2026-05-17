//! Parameter specification for function/method signatures.

use crate::code_block::{Arg, CodeBlockBuilder};
use crate::lang::CodeLang;
use crate::type_name::TypeName;

/// A single function parameter.
///
/// `ParameterSpec` models a named, typed parameter in a function signature.
/// It supports optional default values and variadic markers. For simple
/// parameters, use [`ParameterSpec::new()`]; for defaults or variadic,
/// use [`ParameterSpec::builder()`].
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// // Simple parameter:
/// let param = ParameterSpec::new("id", TypeName::primitive("string")).unwrap();
///
/// // Parameter with default value:
/// let default_val = CodeBlock::of("0", ()).unwrap();
/// let param = ParameterSpec::builder("count", TypeName::primitive("number"))
///     .default_value(default_val)
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParameterSpec {
    pub(crate) name: String,
    pub(crate) param_type: TypeName,
    pub(crate) default_value: Option<crate::code_block::CodeBlock>,
    pub(crate) is_variadic: bool,
    pub(crate) is_property: bool,
    pub(crate) is_mutable_property: bool,
}

impl ParameterSpec {
    /// Create a new builder for a parameter with the given name and type.
    pub fn builder(name: &str, param_type: TypeName) -> ParameterSpecBuilder {
        ParameterSpecBuilder {
            name: name.to_string(),
            param_type,
            default_value: None,
            is_variadic: false,
            is_property: false,
            is_mutable_property: false,
        }
    }

    /// Convenience constructor for a simple parameter (name + type, no frills).
    pub fn new(name: &str, param_type: TypeName) -> Result<Self, crate::error::SigilStitchError> {
        Self::builder(name, param_type).build()
    }

    /// Infallible convenience constructor for a simple parameter.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty.
    pub fn of(name: &str, param_type: TypeName) -> Self {
        Self::new(name, param_type).expect("ParameterSpec name must not be empty")
    }

    /// Return the parameter name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the parameter type.
    pub fn param_type(&self) -> &TypeName {
        &self.param_type
    }

    /// Emit this parameter into a CodeBlockBuilder (appends format parts + args).
    pub fn emit_into(&self, cb: &mut CodeBlockBuilder, lang: &dyn CodeLang) {
        let mut fmt = String::new();
        let mut args: Vec<Arg> = Vec::new();

        if lang.type_decl_syntax().type_before_name {
            // C-style: type name
            if !self.param_type.is_empty() {
                fmt.push_str("%T");
                args.push(Arg::TypeName(self.param_type.clone()));
                fmt.push(' ');
            }
            if self.is_property {
                fmt.push_str(lang.enum_and_annotation().readonly_keyword);
            } else if self.is_mutable_property {
                let mk = lang.enum_and_annotation().mutable_field_keyword;
                if !mk.is_empty() {
                    fmt.push_str(mk);
                }
            }
            fmt.push_str(&lang.escape_reserved(&self.name));
        } else {
            // TS/Rust/Go/Python-style: name sep type
            if self.is_variadic {
                fmt.push_str("...");
            }
            if self.is_property {
                fmt.push_str(lang.enum_and_annotation().readonly_keyword);
            } else if self.is_mutable_property {
                let mk = lang.enum_and_annotation().mutable_field_keyword;
                if !mk.is_empty() {
                    fmt.push_str(mk);
                }
            }
            fmt.push_str(&lang.escape_reserved(&self.name));

            // Skip type annotation when the type is empty (e.g., Python's bare `self`).
            if !self.param_type.is_empty() {
                let sep = lang.type_decl_syntax().type_annotation_separator;
                fmt.push_str(sep);
                fmt.push_str("%T");
                args.push(Arg::TypeName(self.param_type.clone()));
            }
        }

        if let Some(default) = &self.default_value {
            fmt.push_str(" = %L");
            args.push(Arg::Code(default.clone()));
        }

        cb.add(&fmt, args);
    }
}

/// Builder for [`ParameterSpec`].
#[derive(Debug)]
pub struct ParameterSpecBuilder {
    name: String,
    param_type: TypeName,
    default_value: Option<crate::code_block::CodeBlock>,
    is_variadic: bool,
    is_property: bool,
    is_mutable_property: bool,
}

impl ParameterSpecBuilder {
    /// Set the default value for this parameter.
    pub fn default_value(mut self, value: crate::code_block::CodeBlock) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Mark this parameter as variadic.
    pub fn variadic(mut self) -> Self {
        self.is_variadic = true;
        self
    }

    /// Mark this parameter as a readonly property declaration.
    ///
    /// When emitting, prepends the language's readonly keyword (e.g., `val` in Kotlin).
    /// Used for primary constructor parameters that declare properties.
    pub fn is_property(mut self) -> Self {
        self.is_property = true;
        self
    }

    /// Mark this parameter as a mutable property declaration.
    ///
    /// When emitting, prepends the language's mutable field keyword (e.g., `var` in Kotlin).
    /// Used for primary constructor parameters that declare mutable properties.
    pub fn is_mutable_property(mut self) -> Self {
        self.is_mutable_property = true;
        self
    }

    /// Build the [`ParameterSpec`].
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`](crate::error::SigilStitchError::EmptyName) if `name` is empty.
    pub fn build(self) -> Result<ParameterSpec, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "ParameterSpecBuilder",
            }
        );
        Ok(ParameterSpec {
            name: self.name,
            param_type: self.param_type,
            default_value: self.default_value,
            is_variadic: self.is_variadic,
            is_property: self.is_property,
            is_mutable_property: self.is_mutable_property,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_block::CodeBlock;
    use crate::lang::typescript::TypeScript;

    fn emit_param(spec: &ParameterSpec) -> String {
        let lang = TypeScript::new();
        let mut cb = CodeBlock::builder();
        spec.emit_into(&mut cb, &lang);
        // Build and render the parts to a simple string for testing.
        let block = cb.build().unwrap();
        // Use a basic renderer to get the output.
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
        renderer.render(&block).unwrap()
    }

    #[test]
    fn test_simple_param() {
        let param = ParameterSpec::new("id", TypeName::primitive("string")).unwrap();
        let output = emit_param(&param);
        assert_eq!(output, "id: string");
    }

    #[test]
    fn test_variadic_param() {
        let param = ParameterSpec::builder("args", TypeName::primitive("string"))
            .variadic()
            .build()
            .unwrap();
        let output = emit_param(&param);
        assert_eq!(output, "...args: string");
    }

    #[test]
    fn test_param_with_default() {
        let default = CodeBlock::of("0", ()).unwrap();
        let param = ParameterSpec::builder("count", TypeName::primitive("number"))
            .default_value(default)
            .build()
            .unwrap();
        let output = emit_param(&param);
        assert_eq!(output, "count: number = 0");
    }

    #[test]
    fn test_build_empty_name_errors() {
        let result = ParameterSpec::builder("", TypeName::primitive("string")).build();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }

    #[test]
    fn test_property_param_kotlin() {
        let kt = crate::lang::kotlin::Kotlin::new();
        let param = ParameterSpec::builder("name", TypeName::primitive("String"))
            .is_property()
            .build()
            .unwrap();
        let mut cb = CodeBlock::builder();
        param.emit_into(&mut cb, &kt);
        let block = cb.build().unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&kt, &imports, 80);
        let output = renderer.render(&block).unwrap();
        assert_eq!(output, "val name: String");
    }

    #[test]
    fn test_mutable_property_param_kotlin() {
        let kt = crate::lang::kotlin::Kotlin::new();
        let param = ParameterSpec::builder("name", TypeName::primitive("String"))
            .is_mutable_property()
            .build()
            .unwrap();
        let mut cb = CodeBlock::builder();
        param.emit_into(&mut cb, &kt);
        let block = cb.build().unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&kt, &imports, 80);
        let output = renderer.render(&block).unwrap();
        assert_eq!(output, "var name: String");
    }
}
