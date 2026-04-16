//! Function/method specification.

use crate::code_block::{Arg, CodeBlock};
use crate::lang::CodeLang;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::modifiers::{
    ConstructorDelegationStyle, DeclarationContext, Modifiers, Visibility,
};
use crate::spec::parameter_spec::ParameterSpec;
use crate::type_name::TypeName;

/// A generic type parameter with optional bounds.
///
/// Used with [`FunSpec`] and [`TypeSpec`](crate::spec::type_spec::TypeSpec) for
/// generic declarations (e.g., `<T extends Serializable>` in TypeScript,
/// `<T: Clone>` in Rust).
///
/// # Examples
///
/// ```ignore
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let tp = TypeParamSpec::<TypeScript>::new("T")
///     .with_bound(TypeName::primitive("Serializable"));
/// let mut fb = FunSpec::<TypeScript>::builder("serialize");
/// fb.add_type_param(tp);
/// ```
#[derive(Debug, Clone)]
pub struct TypeParamSpec<L: CodeLang> {
    pub(crate) name: String,
    pub(crate) bounds: Vec<TypeName<L>>,
}

impl<L: CodeLang> TypeParamSpec<L> {
    /// Create a new type parameter with the given name and no bounds.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            bounds: Vec::new(),
        }
    }

    /// Add a trait/interface bound to this type parameter.
    pub fn with_bound(mut self, bound: TypeName<L>) -> Self {
        self.bounds.push(bound);
        self
    }
}

/// Render type parameters into `<T: Bound, U>` form, appending to a format string and args vec.
/// Returns the format string fragment (empty string if no type params).
pub fn render_type_params<L: CodeLang>(
    params: &[TypeParamSpec<L>],
    lang: &L,
    args: &mut Vec<Arg<L>>,
) -> String {
    if params.is_empty() {
        return String::new();
    }

    let constraint_kw = lang.generic_constraint_keyword();
    let constraint_sep = lang.generic_constraint_separator();

    let mut fmt = String::from(lang.generic_open());
    for (i, tp) in params.iter().enumerate() {
        if i > 0 {
            fmt.push_str(", ");
        }
        fmt.push_str(&tp.name);
        if !tp.bounds.is_empty() {
            fmt.push_str(constraint_kw);
            for (j, bound) in tp.bounds.iter().enumerate() {
                if j > 0 {
                    fmt.push_str(constraint_sep);
                }
                fmt.push_str("%T");
                args.push(Arg::TypeName(bound.clone()));
            }
        }
    }
    fmt.push_str(lang.generic_close());
    fmt
}

/// A function or method specification.
///
/// `FunSpec` models a function declaration with parameters, return type, body,
/// modifiers (visibility, async, static, abstract, constructor), type parameters,
/// annotations, and doc comments. It emits a language-appropriate `CodeBlock` via
/// [`FunSpec::emit()`].
///
/// Use [`FunSpec::builder()`] to construct. Add to a [`FileSpec`](crate::spec::file_spec::FileSpec)
/// with `add_function()` or to a [`TypeSpec`](crate::spec::type_spec::TypeSpec)
/// with `add_method()`.
///
/// # Examples
///
/// ```ignore
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let body = CodeBlock::<TypeScript>::of("return this.name", ()).unwrap();
///
/// let mut fb = FunSpec::<TypeScript>::builder("getName");
/// fb.returns(TypeName::primitive("string"));
/// fb.body(body);
/// let fun = fb.build();
/// ```
#[derive(Debug, Clone)]
pub struct FunSpec<L: CodeLang> {
    pub(crate) name: String,
    pub(crate) params: Vec<ParameterSpec<L>>,
    pub(crate) return_type: Option<TypeName<L>>,
    pub(crate) body: Option<CodeBlock<L>>,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) type_params: Vec<TypeParamSpec<L>>,
    pub(crate) annotations: Vec<CodeBlock<L>>,
    pub(crate) annotation_specs: Vec<AnnotationSpec<L>>,
    /// Receiver parameter (e.g., Go: `func (s *Server) Handle()`).
    pub(crate) receiver: Option<ParameterSpec<L>>,
    /// Suffixes appended after the parameter list (e.g., C++: `const`, `override`, `= 0`).
    pub(crate) suffixes: Vec<String>,
    /// Constructor delegation call (e.g., `super(arg1, arg2)` or `this(arg1)`).
    ///
    /// For body-style languages (TS, Java, Dart, Swift): emitted as the first
    /// statement in the constructor body.
    /// For signature-style languages (Kotlin): emitted after the parameter list
    /// as ` : super(...)` / ` : this(...)`.
    pub(crate) delegation: Option<CodeBlock<L>>,
}

impl<L: CodeLang> FunSpec<L> {
    /// Create a new builder for a function with the given name.
    pub fn builder(name: &str) -> FunSpecBuilder<L> {
        FunSpecBuilder {
            name: name.to_string(),
            params: Vec::new(),
            return_type: None,
            body: None,
            modifiers: Modifiers::default(),
            doc: Vec::new(),
            type_params: Vec::new(),
            annotations: Vec::new(),
            annotation_specs: Vec::new(),
            receiver: None,
            suffixes: Vec::new(),
            delegation: None,
        }
    }

    /// Return the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Emit this function as a CodeBlock.
    pub fn emit(
        &self,
        lang: &L,
        ctx: DeclarationContext,
    ) -> Result<CodeBlock<L>, crate::error::SigilStitchError> {
        let mut cb = CodeBlock::<L>::builder();

        // Annotations (structured specs first, then raw CodeBlocks).
        for spec in &self.annotation_specs {
            cb.add_code(spec.emit(lang)?);
            cb.add_line();
        }
        for ann in &self.annotations {
            cb.add_code(ann.clone());
            cb.add_line();
        }

        // Doc comment (above the declaration, unless lang puts it inside the body).
        let doc_inside = lang.doc_comment_inside_body();
        if !self.doc.is_empty() && !doc_inside {
            let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
            let doc_str = lang.render_doc_comment(&doc_lines);
            cb.add("%L", doc_str);
            cb.add_line();
        }

        // Build signature.
        let vis = lang.render_visibility(self.modifiers.visibility, ctx);
        let fn_kw = if self.modifiers.is_constructor {
            lang.constructor_keyword()
        } else {
            lang.function_keyword(ctx)
        };

        let mut sig = String::new();
        let mut sig_args: Vec<Arg<L>> = Vec::new();

        sig.push_str(vis);
        if self.modifiers.is_abstract {
            sig.push_str(lang.abstract_keyword());
        }
        if self.modifiers.is_static {
            sig.push_str("static ");
        }
        if self.modifiers.is_override {
            sig.push_str("override ");
        }
        if self.modifiers.is_async {
            sig.push_str(lang.async_keyword());
        }

        // Return type as prefix (C-style: `int add(...)`).
        if lang.return_type_is_prefix()
            && let Some(ret) = &self.return_type
        {
            sig.push_str("%T");
            sig_args.push(Arg::TypeName(ret.clone()));
            sig.push(' ');
        }

        if !fn_kw.is_empty() {
            sig.push_str(fn_kw);
            sig.push(' ');
        }

        // Receiver (e.g., Go: `func (s *Server) Handle()`).
        if let Some(recv) = &self.receiver {
            sig.push('(');
            sig.push_str(&lang.escape_reserved(&recv.name));
            sig.push_str(lang.type_annotation_separator());
            sig.push_str("%T");
            sig_args.push(Arg::TypeName(recv.param_type.clone()));
            sig.push_str(") ");
        }

        sig.push_str(&self.name);

        // Type parameters.
        let tp_str = render_type_params(&self.type_params, lang, &mut sig_args);
        sig.push_str(&tp_str);

        // Parameters — build as a sub-block for %W support.
        sig.push('(');
        sig.push_str("%L");
        let params_block = self.build_params_block(lang)?;
        sig_args.push(Arg::Code(params_block));
        sig.push(')');

        // Method suffixes (C++: const, override, noexcept, = 0).
        for s in &self.suffixes {
            sig.push(' ');
            sig.push_str(s);
        }

        // Return type as suffix (TS/Rust/Go-style: `fn add(...) -> int`).
        if !lang.return_type_is_prefix()
            && let Some(ret) = &self.return_type
        {
            sig.push_str(lang.return_type_separator());
            sig.push_str("%T");
            sig_args.push(Arg::TypeName(ret.clone()));
        }

        // Constructor delegation — signature style (Kotlin: `constructor(x: Int) : this(x, 0)`).
        let delegation_in_body = if let Some(deleg) = &self.delegation {
            if lang.constructor_delegation_style() == ConstructorDelegationStyle::Signature {
                sig.push_str(" : %L");
                sig_args.push(Arg::Code(deleg.clone()));
                false
            } else {
                true
            }
        } else {
            false
        };

        // Body or abstract.
        if let Some(body) = &self.body {
            sig.push_str(lang.block_open());
            cb.add(&sig, sig_args);
            cb.add_line();
            cb.add("%>", ());
            // Docstring inside body (Python).
            if !self.doc.is_empty() && doc_inside {
                let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
                let doc_str = lang.render_doc_comment(&doc_lines);
                cb.add("%L", doc_str);
                cb.add_line();
            }
            // Constructor delegation — body style (TS/Java/Dart/Swift).
            if delegation_in_body && let Some(deleg) = &self.delegation {
                cb.add_statement("%L", deleg.clone());
            }
            cb.add_code(body.clone());
            cb.add_line();
            cb.add("%<", ());
            let close = lang.block_close();
            if !close.is_empty() {
                cb.add(close, ());
                cb.add_line();
            }
        } else {
            let empty = lang.empty_body();
            if !empty.is_empty() {
                // Language requires a body placeholder (e.g., Python `...`).
                sig.push_str(lang.block_open());
                cb.add(&sig, sig_args);
                cb.add_line();
                cb.add("%>", ());
                // Docstring inside body (Python).
                if !self.doc.is_empty() && doc_inside {
                    let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
                    let doc_str = lang.render_doc_comment(&doc_lines);
                    cb.add("%L", doc_str);
                    cb.add_line();
                }
                // Constructor delegation — body style.
                if delegation_in_body && let Some(deleg) = &self.delegation {
                    cb.add_statement("%L", deleg.clone());
                }
                cb.add_statement(empty, ());
                cb.add("%<", ());
                let close = lang.block_close();
                if !close.is_empty() {
                    cb.add(close, ());
                    cb.add_line();
                }
            } else {
                if lang.uses_semicolons() {
                    sig.push(';');
                }
                cb.add(&sig, sig_args);
                cb.add_line();
            }
        }

        cb.build()
    }

    fn build_params_block(&self, lang: &L) -> Result<CodeBlock<L>, crate::error::SigilStitchError> {
        let mut pb = CodeBlock::<L>::builder();
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                pb.add(",%W", ());
            }
            param.emit_into(&mut pb, lang);
        }
        pb.build()
    }
}

/// Builder for [`FunSpec`].
#[derive(Debug)]
pub struct FunSpecBuilder<L: CodeLang> {
    name: String,
    params: Vec<ParameterSpec<L>>,
    return_type: Option<TypeName<L>>,
    body: Option<CodeBlock<L>>,
    modifiers: Modifiers,
    doc: Vec<String>,
    type_params: Vec<TypeParamSpec<L>>,
    annotations: Vec<CodeBlock<L>>,
    annotation_specs: Vec<AnnotationSpec<L>>,
    receiver: Option<ParameterSpec<L>>,
    suffixes: Vec<String>,
    delegation: Option<CodeBlock<L>>,
}

impl<L: CodeLang> FunSpecBuilder<L> {
    /// Add a parameter to the function signature.
    pub fn add_param(&mut self, param: ParameterSpec<L>) -> &mut Self {
        self.params.push(param);
        self
    }

    /// Set the return type.
    pub fn returns(&mut self, ret: TypeName<L>) -> &mut Self {
        self.return_type = Some(ret);
        self
    }

    /// Set the function body.
    pub fn body(&mut self, body: CodeBlock<L>) -> &mut Self {
        self.body = Some(body);
        self
    }

    /// Set the visibility modifier.
    pub fn visibility(&mut self, vis: Visibility) -> &mut Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this function as async.
    pub fn is_async(&mut self) -> &mut Self {
        self.modifiers.is_async = true;
        self
    }

    /// Mark this function as static.
    pub fn is_static(&mut self) -> &mut Self {
        self.modifiers.is_static = true;
        self
    }

    /// Mark this function as abstract.
    pub fn is_abstract(&mut self) -> &mut Self {
        self.modifiers.is_abstract = true;
        self
    }

    /// Mark this function as an override.
    pub fn is_override(&mut self) -> &mut Self {
        self.modifiers.is_override = true;
        self
    }

    /// Mark this function as a constructor.
    pub fn is_constructor(&mut self) -> &mut Self {
        self.modifiers.is_constructor = true;
        self
    }

    /// Add a documentation comment line.
    pub fn doc(&mut self, line: &str) -> &mut Self {
        self.doc.push(line.to_string());
        self
    }

    /// Add a generic type parameter.
    pub fn add_type_param(&mut self, tp: TypeParamSpec<L>) -> &mut Self {
        self.type_params.push(tp);
        self
    }

    /// Add a raw annotation CodeBlock.
    pub fn annotation(&mut self, ann: CodeBlock<L>) -> &mut Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured annotation spec.
    pub fn annotate(&mut self, spec: AnnotationSpec<L>) -> &mut Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Set the receiver parameter (e.g., Go's `(s *Server)`).
    pub fn receiver(&mut self, recv: ParameterSpec<L>) -> &mut Self {
        self.receiver = Some(recv);
        self
    }

    /// Append a suffix after the parameter list (e.g., C++ `const`, `override`).
    pub fn suffix(&mut self, s: &str) -> &mut Self {
        self.suffixes.push(s.to_string());
        self
    }

    /// Set a constructor delegation call (e.g., `super(arg1, arg2)` or `this(arg1)`).
    ///
    /// For body-style languages (TS, JS, Java, Dart, Swift), this is emitted as
    /// the first statement in the constructor body.
    /// For signature-style languages (Kotlin), this appears after the parameter
    /// list: `constructor(x: Int) : this(x, 0) { ... }`.
    pub fn delegation(&mut self, call: CodeBlock<L>) -> &mut Self {
        self.delegation = Some(call);
        self
    }

    /// Consume the builder and produce a [`FunSpec`].
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`] if `name` is empty.
    pub fn build(self) -> Result<FunSpec<L>, crate::error::SigilStitchError> {
        snafu::ensure!(
            !self.name.is_empty(),
            crate::error::EmptyNameSnafu {
                builder: "FunSpecBuilder",
            }
        );
        Ok(FunSpec {
            name: self.name,
            params: self.params,
            return_type: self.return_type,
            body: self.body,
            modifiers: self.modifiers,
            doc: self.doc,
            type_params: self.type_params,
            annotations: self.annotations,
            annotation_specs: self.annotation_specs,
            receiver: self.receiver,
            suffixes: self.suffixes,
            delegation: self.delegation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::rust_lang::RustLang;
    use crate::lang::typescript::TypeScript;

    fn emit_fun_ts(spec: &FunSpec<TypeScript>, ctx: DeclarationContext) -> String {
        let lang = TypeScript::new();
        let block = spec.emit(&lang, ctx).unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
        renderer.render(&block).unwrap()
    }

    fn emit_fun_rs(spec: &FunSpec<RustLang>, ctx: DeclarationContext) -> String {
        let lang = RustLang::new();
        let block = spec.emit(&lang, ctx).unwrap();
        let imports = crate::import::ImportGroup::new();
        let mut renderer = crate::code_renderer::CodeRenderer::new(&lang, &imports, 80);
        renderer.render(&block).unwrap()
    }

    #[test]
    fn test_ts_simple_function() {
        let mut fb = FunSpec::<TypeScript>::builder("greet");
        fb.add_param(ParameterSpec::new("name", TypeName::primitive("string")).unwrap());
        fb.returns(TypeName::primitive("void"));
        let body = CodeBlock::<TypeScript>::of("console.log(name)", ()).unwrap();
        fb.body(body);
        let fun = fb.build().unwrap();
        let output = emit_fun_ts(&fun, DeclarationContext::TopLevel);
        assert!(output.contains("function greet(name: string): void {"));
        assert!(output.contains("console.log(name)"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_ts_async_method() {
        let mut fb = FunSpec::<TypeScript>::builder("getUser");
        fb.add_param(ParameterSpec::new("id", TypeName::primitive("string")).unwrap());
        fb.returns(TypeName::generic(
            TypeName::primitive("Promise"),
            vec![TypeName::primitive("User")],
        ));
        fb.is_async();
        fb.visibility(Visibility::Public);
        let body = CodeBlock::<TypeScript>::of("return db.find(id)", ()).unwrap();
        fb.body(body);
        let fun = fb.build().unwrap();
        let output = emit_fun_ts(&fun, DeclarationContext::Member);
        assert!(output.contains("public async getUser(id: string): Promise<User> {"));
    }

    #[test]
    fn test_ts_abstract_method() {
        let mut fb = FunSpec::<TypeScript>::builder("validate");
        fb.is_abstract();
        fb.returns(TypeName::primitive("boolean"));
        let fun = fb.build().unwrap();
        let output = emit_fun_ts(&fun, DeclarationContext::Member);
        assert!(output.contains("abstract validate(): boolean;"));
    }

    #[test]
    fn test_rust_simple_function() {
        let mut fb = FunSpec::<RustLang>::builder("add");
        fb.visibility(Visibility::Public);
        fb.add_param(ParameterSpec::new("a", TypeName::primitive("i32")).unwrap());
        fb.add_param(ParameterSpec::new("b", TypeName::primitive("i32")).unwrap());
        fb.returns(TypeName::primitive("i32"));
        let body = CodeBlock::<RustLang>::of("a + b", ()).unwrap();
        fb.body(body);
        let fun = fb.build().unwrap();
        let output = emit_fun_rs(&fun, DeclarationContext::TopLevel);
        assert!(output.contains("pub fn add(a: i32, b: i32) -> i32 {"));
        assert!(output.contains("a + b"));
    }

    #[test]
    fn test_fun_with_type_params() {
        let tp =
            TypeParamSpec::<TypeScript>::new("T").with_bound(TypeName::primitive("Serializable"));
        let mut fb = FunSpec::<TypeScript>::builder("serialize");
        fb.add_type_param(tp);
        fb.add_param(ParameterSpec::new("value", TypeName::primitive("T")).unwrap());
        fb.returns(TypeName::primitive("string"));
        let body = CodeBlock::<TypeScript>::of("return JSON.stringify(value)", ()).unwrap();
        fb.body(body);
        let fun = fb.build().unwrap();
        let output = emit_fun_ts(&fun, DeclarationContext::TopLevel);
        assert!(output.contains("function serialize<T extends Serializable>(value: T): string {"));
    }

    #[test]
    fn test_build_empty_name_errors() {
        let result = FunSpec::<TypeScript>::builder("").build();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'name' must not be empty")
        );
    }
}
