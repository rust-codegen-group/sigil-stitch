//! Parameter specification for function/method signatures.

use crate::code_block::{Arg, CodeBlockBuilder};
use crate::lang::CodeLang;
use crate::type_name::TypeName;

/// A single function parameter.
#[derive(Debug, Clone)]
pub struct ParameterSpec<L: CodeLang> {
    pub(crate) name: String,
    pub(crate) param_type: TypeName<L>,
    pub(crate) default_value: Option<crate::code_block::CodeBlock<L>>,
    pub(crate) is_variadic: bool,
}

impl<L: CodeLang> ParameterSpec<L> {
    /// Create a new builder for a parameter with the given name and type.
    pub fn builder(name: &str, param_type: TypeName<L>) -> ParameterSpecBuilder<L> {
        ParameterSpecBuilder {
            name: name.to_string(),
            param_type,
            default_value: None,
            is_variadic: false,
        }
    }

    /// Convenience constructor for a simple parameter (name + type, no frills).
    pub fn new(name: &str, param_type: TypeName<L>) -> Self {
        Self::builder(name, param_type).build()
    }

    /// Return the parameter name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the parameter type.
    pub fn param_type(&self) -> &TypeName<L> {
        &self.param_type
    }

    /// Emit this parameter into a CodeBlockBuilder (appends format parts + args).
    pub fn emit_into(&self, cb: &mut CodeBlockBuilder<L>, lang: &L) {
        let mut fmt = String::new();
        let mut args: Vec<Arg<L>> = Vec::new();

        if lang.type_before_name() {
            // C-style: type name
            if !self.param_type.is_empty() {
                fmt.push_str("%T");
                args.push(Arg::TypeName(self.param_type.clone()));
                fmt.push(' ');
            }
            fmt.push_str(&lang.escape_reserved(&self.name));
        } else {
            // TS/Rust/Go/Python-style: name sep type
            if self.is_variadic {
                fmt.push_str("...");
            }
            fmt.push_str(&lang.escape_reserved(&self.name));

            // Skip type annotation when the type is empty (e.g., Python's bare `self`).
            if !self.param_type.is_empty() {
                let sep = lang.type_annotation_separator();
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
pub struct ParameterSpecBuilder<L: CodeLang> {
    name: String,
    param_type: TypeName<L>,
    default_value: Option<crate::code_block::CodeBlock<L>>,
    is_variadic: bool,
}

impl<L: CodeLang> ParameterSpecBuilder<L> {
    /// Set the default value for this parameter.
    pub fn default_value(&mut self, value: crate::code_block::CodeBlock<L>) -> &mut Self {
        self.default_value = Some(value);
        self
    }

    /// Mark this parameter as variadic.
    pub fn variadic(&mut self) -> &mut Self {
        self.is_variadic = true;
        self
    }

    /// Build the [`ParameterSpec`].
    pub fn build(self) -> ParameterSpec<L> {
        ParameterSpec {
            name: self.name,
            param_type: self.param_type,
            default_value: self.default_value,
            is_variadic: self.is_variadic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_block::CodeBlock;
    use crate::lang::typescript::TypeScript;

    fn emit_param(spec: &ParameterSpec<TypeScript>) -> String {
        let lang = TypeScript::new();
        let mut cb = CodeBlock::<TypeScript>::builder();
        spec.emit_into(&mut cb, &lang);
        // Build and render the parts to a simple string for testing.
        let block = cb.build().unwrap();
        // Use a basic renderer to get the output.
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
        renderer.render(&block)
    }

    #[test]
    fn test_simple_param() {
        let param = ParameterSpec::<TypeScript>::new("id", TypeName::primitive("string"));
        let output = emit_param(&param);
        assert_eq!(output, "id: string");
    }

    #[test]
    fn test_variadic_param() {
        let mut pb = ParameterSpec::builder("args", TypeName::<TypeScript>::primitive("string"));
        pb.variadic();
        let param = pb.build();
        let output = emit_param(&param);
        assert_eq!(output, "...args: string");
    }

    #[test]
    fn test_param_with_default() {
        let default = CodeBlock::<TypeScript>::of("0", ()).unwrap();
        let mut pb = ParameterSpec::builder("count", TypeName::<TypeScript>::primitive("number"));
        pb.default_value(default);
        let param = pb.build();
        let output = emit_param(&param);
        assert_eq!(output, "count: number = 0");
    }
}
