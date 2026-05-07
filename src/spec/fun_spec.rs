//! Function/method specification.

use crate::code_block::{Arg, CodeBlock};
use crate::lang::CodeLang;
use crate::spec::annotation_spec::AnnotationSpec;
use crate::spec::modifiers::{
    ConstructorDelegationStyle, DeclarationContext, Modifiers, Visibility,
};
use crate::spec::parameter_spec::ParameterSpec;
use crate::type_name::TypeName;

/// A single constraint in a where clause.
///
/// Represents `Subject: Bound1 + Bound2` in Rust's `where` block.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WhereConstraint {
    pub(crate) subject: TypeName,
    pub(crate) bounds: Vec<TypeName>,
}

/// How where-clause constraints are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WhereClauseStyle {
    /// Bounds stay inline in the type parameter list.
    Inline,
    /// Rust-style `where` block after the signature.
    WhereBlock,
}

/// How function parameter lists are formatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamListStyle {
    /// All params in a single `(name: T, name: T)` list (most languages).
    Tupled,
    /// Each param gets its own wrapper: `(name : T) (name : T)` (OCaml).
    Curried,
}

/// How function signatures are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSignatureStyle {
    /// Single line: `fn add(x: Int, y: Int) -> Int {` (most languages).
    Merged,
    /// Separate type signature + definition (Haskell):
    /// ```text
    /// add :: Int -> Int -> Int
    /// add x y =
    /// ```
    Split,
}

/// The kind of a type parameter (for higher-kinded type support).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TypeParamKind {
    /// Type constructor taking one argument: `* -> *` (Haskell), `F[_]` (Scala).
    Constructor1,
    /// Type constructor taking two arguments: `* -> * -> *`.
    Constructor2,
    /// Arbitrary kind expressed as a raw string.
    Raw(String),
}

/// A generic type parameter with optional bounds.
///
/// Used with [`FunSpec`] and [`TypeSpec`](crate::spec::type_spec::TypeSpec) for
/// generic declarations (e.g., `<T extends Serializable>` in TypeScript,
/// `<T: Clone>` in Rust).
///
/// # Examples
///
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let tp = TypeParamSpec::new("T")
///     .with_bound(TypeName::primitive("Serializable"));
/// let fb = FunSpec::builder("serialize")
///     .add_type_param(tp);
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeParamSpec {
    pub(crate) name: String,
    pub(crate) bounds: Vec<TypeName>,
    #[serde(default)]
    pub(crate) kind: Option<TypeParamKind>,
    #[serde(default)]
    pub(crate) is_lifetime: bool,
    #[serde(default)]
    pub(crate) context_bounds: Vec<TypeName>,
}

impl TypeParamSpec {
    /// Create a new type parameter with the given name and no bounds.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            bounds: Vec::new(),
            kind: None,
            is_lifetime: false,
            context_bounds: Vec::new(),
        }
    }

    /// Create a lifetime parameter (Rust `'a`).
    ///
    /// The name should include the tick: `"'a"`, `"'static"`.
    pub fn lifetime(name: &str) -> Self {
        Self {
            name: name.to_string(),
            bounds: Vec::new(),
            kind: None,
            is_lifetime: true,
            context_bounds: Vec::new(),
        }
    }

    /// Add a trait/interface bound to this type parameter.
    pub fn with_bound(mut self, bound: TypeName) -> Self {
        self.bounds.push(bound);
        self
    }

    /// Set this parameter as a higher-kinded type constructor.
    pub fn with_kind(mut self, kind: TypeParamKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Add a context bound (e.g., Scala `[T : Ordering]`).
    pub fn with_context_bound(mut self, bound: TypeName) -> Self {
        self.context_bounds.push(bound);
        self
    }
}

/// Render type parameters into `<T: Bound, U>` form, appending to a format string and args vec.
/// Returns the format string fragment (empty string if no type params).
pub fn render_type_params(
    params: &[TypeParamSpec],
    lang: &dyn CodeLang,
    args: &mut Vec<Arg>,
) -> String {
    if params.is_empty() {
        return String::new();
    }

    let generic = lang.generic_syntax();
    let constraint_kw = generic.constraint_keyword;
    let constraint_sep = generic.constraint_separator;

    let mut fmt = String::from(generic.open);
    let mut first = true;

    // Lifetimes first (Rust convention: `<'a, 'b, T, U>`).
    for tp in params.iter().filter(|p| p.is_lifetime) {
        if !first {
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
        first = false;
    }

    // Then type parameters.
    for tp in params.iter().filter(|p| !p.is_lifetime) {
        if !first {
            fmt.push_str(", ");
        }
        fmt.push_str(&tp.name);
        if let Some(ref kind) = tp.kind {
            fmt.push_str(&lang.render_type_param_kind(kind));
        }
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
        let ctx_kw = generic.context_bound_keyword;
        for ctx_bound in &tp.context_bounds {
            fmt.push_str(ctx_kw);
            fmt.push_str("%T");
            args.push(Arg::TypeName(ctx_bound.clone()));
        }
        first = false;
    }

    fmt.push_str(generic.close);
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
/// ```
/// use sigil_stitch::prelude::*;
/// use sigil_stitch::lang::typescript::TypeScript;
///
/// let body = CodeBlock::of("return this.name", ()).unwrap();
///
/// let fun = FunSpec::builder("getName")
///     .returns(TypeName::primitive("string"))
///     .body(body)
///     .build().unwrap();
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunSpec {
    pub(crate) name: String,
    pub(crate) params: Vec<ParameterSpec>,
    pub(crate) return_type: Option<TypeName>,
    pub(crate) body: Option<CodeBlock>,
    pub(crate) modifiers: Modifiers,
    pub(crate) doc: Vec<String>,
    pub(crate) type_params: Vec<TypeParamSpec>,
    pub(crate) annotations: Vec<CodeBlock>,
    pub(crate) annotation_specs: Vec<AnnotationSpec>,
    /// Receiver parameter (e.g., Go: `func (s *Server) Handle()`).
    pub(crate) receiver: Option<ParameterSpec>,
    /// Suffixes appended after the parameter list (e.g., C++: `const`, `override`, `= 0`).
    pub(crate) suffixes: Vec<String>,
    /// Constructor delegation call (e.g., `super(arg1, arg2)` or `this(arg1)`).
    ///
    /// For body-style languages (TS, Java, Dart, Swift): emitted as the first
    /// statement in the constructor body.
    /// For signature-style languages (Kotlin): emitted after the parameter list
    /// as ` : super(...)` / ` : this(...)`.
    pub(crate) delegation: Option<CodeBlock>,
    /// Where-clause constraints (e.g., Rust `where T: Clone + Send`).
    #[serde(default)]
    pub(crate) where_constraints: Vec<WhereConstraint>,
}

impl FunSpec {
    /// Create a new builder for a function with the given name.
    pub fn builder(name: &str) -> FunSpecBuilder {
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
            where_constraints: Vec::new(),
        }
    }

    /// Return the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Emit this function as a CodeBlock.
    pub fn emit(
        &self,
        lang: &dyn CodeLang,
        ctx: DeclarationContext,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let mut cb = CodeBlock::builder();

        let emit_doc = || -> Option<String> {
            if self.doc.is_empty() || lang.doc_comment_inside_body() {
                return None;
            }
            let doc_lines: Vec<&str> = self.doc.iter().map(|s| s.as_str()).collect();
            Some(lang.render_doc_comment(&doc_lines))
        };

        if lang.doc_before_annotations()
            && let Some(doc_str) = emit_doc()
        {
            cb.add("%L", doc_str);
            cb.add_line();
        }

        for spec in &self.annotation_specs {
            cb.add_code(spec.emit(lang)?);
            cb.add_line();
        }
        for ann in &self.annotations {
            cb.add_code(ann.clone());
            cb.add_line();
        }

        if !lang.doc_before_annotations()
            && let Some(doc_str) = emit_doc()
        {
            cb.add("%L", doc_str);
            cb.add_line();
        }

        // Build signature.
        if lang.function_syntax().function_signature_style == FunctionSignatureStyle::Split {
            return self.emit_split_signature(cb, lang);
        }
        let vis = lang.render_visibility(self.modifiers.visibility, ctx);
        let fn_kw = if self.modifiers.is_constructor {
            lang.function_syntax().constructor_keyword
        } else {
            lang.function_keyword(ctx)
        };

        let mut sig = String::new();
        let mut sig_args: Vec<Arg> = Vec::new();

        sig.push_str(vis);
        if self.modifiers.is_abstract {
            sig.push_str(lang.function_syntax().abstract_keyword);
        }
        if self.modifiers.is_static {
            sig.push_str("static ");
        }
        if self.modifiers.is_override {
            sig.push_str("override ");
        }
        if self.modifiers.is_async {
            sig.push_str(lang.function_syntax().async_keyword);
        }

        // Return type as prefix (C-style: `int add(...)`).
        if lang.type_decl_syntax().return_type_is_prefix
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
            sig.push_str(lang.type_decl_syntax().type_annotation_separator);
            sig.push_str("%T");
            sig_args.push(Arg::TypeName(recv.param_type.clone()));
            sig.push_str(") ");
        }

        sig.push_str(&self.name);

        // Type parameters.
        let tp_str = render_type_params(&self.type_params, lang, &mut sig_args);
        sig.push_str(&tp_str);

        // Parameters — build as a sub-block for %W support.
        if lang.function_syntax().param_list_style == ParamListStyle::Curried {
            if !self.params.is_empty() {
                sig.push(' ');
                sig.push_str("%L");
                let params_block = self.build_curried_params_block(lang)?;
                sig_args.push(Arg::Code(params_block));
            }
        } else {
            sig.push('(');
            sig.push_str("%L");
            let params_block = self.build_params_block(lang)?;
            sig_args.push(Arg::Code(params_block));
            sig.push(')');
        }

        // Method suffixes (C++: const, override, noexcept, = 0).
        for s in &self.suffixes {
            sig.push(' ');
            sig.push_str(s);
        }

        // Return type as suffix (TS/Rust/Go-style: `fn add(...) -> int`).
        // Skip when separator is empty (e.g. Lua) — nothing to separate.
        if !lang.type_decl_syntax().return_type_is_prefix
            && let Some(ret) = &self.return_type
        {
            let sep = lang.function_syntax().return_type_separator;
            if !sep.is_empty() {
                sig.push_str(sep);
                sig.push_str("%T");
                sig_args.push(Arg::TypeName(ret.clone()));
            }
        }

        // Constructor delegation — signature style (Kotlin: `constructor(x: Int) : this(x, 0)`).
        let delegation_in_body = if let Some(deleg) = &self.delegation {
            if lang.function_syntax().constructor_delegation_style
                == ConstructorDelegationStyle::Signature
            {
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
            self.emit_where_and_open(&mut sig, &mut sig_args, lang);
            cb.add(&sig, sig_args);
            cb.add_line();
            cb.add("%>", ());
            // Docstring inside body (Python).
            if !self.doc.is_empty() && lang.doc_comment_inside_body() {
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
            if !body.ends_with_newline_or_block_close() {
                cb.add_line();
            }
            cb.add("%<", ());
            let close = lang.block_syntax().block_close;
            if !close.is_empty() {
                cb.add(close, ());
                cb.add_line();
            }
        } else {
            let empty = lang.function_syntax().empty_body;
            if !empty.is_empty() {
                // Language requires a body placeholder (e.g., Python `...`).
                self.emit_where_and_open(&mut sig, &mut sig_args, lang);
                cb.add(&sig, sig_args);
                cb.add_line();
                cb.add("%>", ());
                // Docstring inside body (Python).
                if !self.doc.is_empty() && lang.doc_comment_inside_body() {
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
                let close = lang.block_syntax().block_close;
                if !close.is_empty() {
                    cb.add(close, ());
                    cb.add_line();
                }
            } else {
                if lang.block_syntax().uses_semicolons {
                    sig.push(';');
                }
                cb.add(&sig, sig_args);
                cb.add_line();
            }
        }

        cb.build()
    }

    fn build_params_block(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let mut pb = CodeBlock::builder();
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                pb.add(",%W", ());
            }
            param.emit_into(&mut pb, lang);
        }
        pb.build()
    }

    fn build_curried_params_block(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let mut pb = CodeBlock::builder();
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                pb.add(" ", ());
            }
            pb.add("(", ());
            param.emit_into(&mut pb, lang);
            pb.add(")", ());
        }
        pb.build()
    }

    fn emit_where_and_open(&self, sig: &mut String, sig_args: &mut Vec<Arg>, lang: &dyn CodeLang) {
        if !self.where_constraints.is_empty()
            && lang.function_syntax().where_clause_style == WhereClauseStyle::WhereBlock
        {
            emit_where_block(sig, sig_args, &self.where_constraints, lang);
        }
        sig.push_str(lang.fun_block_open());
    }

    /// Emit a function with separate type signature and definition (Haskell style).
    ///
    /// Renders:
    /// ```text
    /// add :: Int -> Int -> Int
    /// add x y =
    ///   body
    /// ```
    fn emit_split_signature(
        &self,
        mut cb: crate::code_block::CodeBlockBuilder,
        lang: &dyn CodeLang,
    ) -> Result<CodeBlock, crate::error::SigilStitchError> {
        let resolve = |_module: &str, name: &str| name.to_string();

        // Type context (e.g., "(Show a) => ").
        let context = lang.render_type_context(&self.type_params);

        // Build type signature: name :: context param1_type -> param2_type -> return_type
        let mut type_parts: Vec<String> = Vec::new();
        for param in &self.params {
            let t = param.param_type.render(80, &resolve).unwrap_or_default();
            type_parts.push(t);
        }
        if let Some(ret) = &self.return_type {
            let r = ret.render(80, &resolve).unwrap_or_default();
            type_parts.push(r);
        }

        if !type_parts.is_empty() {
            let type_sig = format!("{} :: {}{}", self.name, context, type_parts.join(" -> "));
            cb.add("%L", type_sig);
            cb.add_line();
        }

        // Build definition: name param1_name param2_name block_open
        let mut def = String::new();
        def.push_str(&self.name);
        for param in &self.params {
            def.push(' ');
            def.push_str(&lang.escape_reserved(&param.name));
        }
        def.push_str(lang.block_syntax().block_open);

        if let Some(body) = &self.body {
            cb.add(&def, ());
            cb.add_line();
            cb.add("%>", ());
            cb.add_code(body.clone());
            if !body.ends_with_newline_or_block_close() {
                cb.add_line();
            }
            cb.add("%<", ());
            let close = lang.block_syntax().block_close;
            if !close.is_empty() {
                cb.add(close, ());
                cb.add_line();
            }
        } else {
            let empty = lang.function_syntax().empty_body;
            if !empty.is_empty() {
                cb.add(&def, ());
                cb.add_line();
                cb.add("%>", ());
                cb.add_statement(empty, ());
                cb.add("%<", ());
                let close = lang.block_syntax().block_close;
                if !close.is_empty() {
                    cb.add(close, ());
                    cb.add_line();
                }
            } else {
                // No body and no empty_body placeholder: emit only the type signature.
            }
        }

        cb.build()
    }
}

/// Append a where-clause block to a format string.
///
/// Renders Rust-style:
/// ```text
/// \nwhere\n    T: Clone + Send,\n    U: Debug,
/// ```
pub(crate) fn emit_where_block(
    fmt: &mut String,
    args: &mut Vec<Arg>,
    constraints: &[WhereConstraint],
    lang: &dyn CodeLang,
) {
    let generic = lang.generic_syntax();
    let constraint_sep = generic.constraint_separator;
    let indent = lang.block_syntax().indent_unit;
    fmt.push_str("\nwhere\n");
    for (i, wc) in constraints.iter().enumerate() {
        if i > 0 {
            fmt.push('\n');
        }
        fmt.push_str(indent);
        fmt.push_str("%T");
        args.push(Arg::TypeName(wc.subject.clone()));
        fmt.push_str(lang.generic_syntax().constraint_keyword);
        for (j, bound) in wc.bounds.iter().enumerate() {
            if j > 0 {
                fmt.push_str(constraint_sep);
            }
            fmt.push_str("%T");
            args.push(Arg::TypeName(bound.clone()));
        }
        fmt.push(',');
    }
}

/// Builder for [`FunSpec`].
#[derive(Debug)]
pub struct FunSpecBuilder {
    name: String,
    params: Vec<ParameterSpec>,
    return_type: Option<TypeName>,
    body: Option<CodeBlock>,
    modifiers: Modifiers,
    doc: Vec<String>,
    type_params: Vec<TypeParamSpec>,
    annotations: Vec<CodeBlock>,
    annotation_specs: Vec<AnnotationSpec>,
    receiver: Option<ParameterSpec>,
    suffixes: Vec<String>,
    delegation: Option<CodeBlock>,
    where_constraints: Vec<WhereConstraint>,
}

impl FunSpecBuilder {
    /// Add a parameter to the function signature.
    pub fn add_param(mut self, param: ParameterSpec) -> Self {
        self.params.push(param);
        self
    }

    /// Set the return type.
    pub fn returns(mut self, ret: TypeName) -> Self {
        self.return_type = Some(ret);
        self
    }

    /// Set the function body.
    pub fn body(mut self, body: CodeBlock) -> Self {
        self.body = Some(body);
        self
    }

    /// Set the visibility modifier.
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.modifiers.visibility = vis;
        self
    }

    /// Mark this function as async.
    pub fn is_async(mut self) -> Self {
        self.modifiers.is_async = true;
        self
    }

    /// Mark this function as static.
    pub fn is_static(mut self) -> Self {
        self.modifiers.is_static = true;
        self
    }

    /// Mark this function as abstract.
    pub fn is_abstract(mut self) -> Self {
        self.modifiers.is_abstract = true;
        self
    }

    /// Mark this function as an override.
    pub fn is_override(mut self) -> Self {
        self.modifiers.is_override = true;
        self
    }

    /// Mark this function as a constructor.
    pub fn is_constructor(mut self) -> Self {
        self.modifiers.is_constructor = true;
        self
    }

    /// Add a documentation comment line.
    pub fn doc(mut self, line: &str) -> Self {
        self.doc.push(line.to_string());
        self
    }

    /// Add a generic type parameter.
    pub fn add_type_param(mut self, tp: TypeParamSpec) -> Self {
        self.type_params.push(tp);
        self
    }

    /// Add a raw annotation CodeBlock.
    pub fn annotation(mut self, ann: CodeBlock) -> Self {
        self.annotations.push(ann);
        self
    }

    /// Add a structured annotation spec.
    pub fn annotate(mut self, spec: AnnotationSpec) -> Self {
        self.annotation_specs.push(spec);
        self
    }

    /// Set the receiver parameter (e.g., Go's `(s *Server)`).
    pub fn receiver(mut self, recv: ParameterSpec) -> Self {
        self.receiver = Some(recv);
        self
    }

    /// Append a suffix after the parameter list (e.g., C++ `const`, `override`).
    pub fn suffix(mut self, s: &str) -> Self {
        self.suffixes.push(s.to_string());
        self
    }

    /// Set a constructor delegation call (e.g., `super(arg1, arg2)` or `this(arg1)`).
    ///
    /// For body-style languages (TS, JS, Java, Dart, Swift), this is emitted as
    /// the first statement in the constructor body.
    /// For signature-style languages (Kotlin), this appears after the parameter
    /// list: `constructor(x: Int) : this(x, 0) { ... }`.
    pub fn delegation(mut self, call: CodeBlock) -> Self {
        self.delegation = Some(call);
        self
    }

    /// Add a where-clause constraint (e.g., `T: Clone + Send`).
    pub fn add_where_constraint(mut self, subject: TypeName, bounds: Vec<TypeName>) -> Self {
        self.where_constraints
            .push(WhereConstraint { subject, bounds });
        self
    }

    /// Convenience: add a single bound to an existing or new where constraint for
    /// the named type parameter.
    pub fn where_bound(mut self, param_name: &str, bound: TypeName) -> Self {
        if let Some(wc) = self
            .where_constraints
            .iter_mut()
            .find(|wc| wc.subject.simple_name() == Some(param_name))
        {
            wc.bounds.push(bound);
        } else {
            self.where_constraints.push(WhereConstraint {
                subject: TypeName::primitive(param_name),
                bounds: vec![bound],
            });
        }
        self
    }

    /// Consume the builder and produce a [`FunSpec`].
    ///
    /// # Errors
    ///
    /// Returns [`SigilStitchError::EmptyName`](crate::error::SigilStitchError::EmptyName) if `name` is empty.
    pub fn build(self) -> Result<FunSpec, crate::error::SigilStitchError> {
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
            where_constraints: self.where_constraints,
        })
    }
}

impl crate::spec::emittable::Emittable for FunSpec {
    fn emit_members(
        &self,
        lang: &dyn CodeLang,
    ) -> Result<Vec<crate::code_block::CodeBlock>, crate::error::SigilStitchError> {
        Ok(vec![self.emit(lang, DeclarationContext::TopLevel)?])
    }
}
