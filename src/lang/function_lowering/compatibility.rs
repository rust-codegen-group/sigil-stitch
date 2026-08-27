//! Frozen pre-0.6.8 function declaration lowering.

#![allow(deprecated)]

use std::borrow::Cow;

use crate::code_block::{CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::lang::CodeLang;
use crate::lang::capability::FunctionForm;
use crate::spec::fun_spec::{FunctionSignatureStyle, ParamListStyle, ValidatedFunction};
use crate::spec::modifiers::{ConstructorDelegationStyle, DeclarationContext};
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::where_spec::{
    TypeParamSpec, WhereClauseStyle, emit_separate_where_block, emit_where_block,
    render_type_params_for,
};

use super::{
    SignatureBuilder, curried_parameter_list, tupled_parameter_list,
    type_params_with_inline_constraints,
};

struct CompatibilityLowering<'lang, 'function, L: CodeLang + ?Sized> {
    lang: &'lang L,
    function: ValidatedFunction<'function>,
    block: CodeBlockBuilder,
    signature: SignatureBuilder,
}

impl<'lang, 'function, L: CodeLang + ?Sized> CompatibilityLowering<'lang, 'function, L> {
    fn new(lang: &'lang L, function: ValidatedFunction<'function>) -> Self {
        Self {
            lang,
            function,
            block: CodeBlock::builder(),
            signature: SignatureBuilder::new(),
        }
    }

    fn emit_preamble(&mut self) -> Result<(), SigilStitchError> {
        let emit_doc = || -> Option<String> {
            if self.function.doc().is_empty() || self.lang.doc_comment_inside_body() {
                return None;
            }
            let doc_lines: Vec<&str> = self.function.doc().iter().map(String::as_str).collect();
            Some(self.lang.render_doc_comment(&doc_lines))
        };

        if self.lang.doc_before_annotations()
            && let Some(doc) = emit_doc()
        {
            self.block.add("%L", doc);
            self.block.add_line();
        }

        for annotation in self.function.annotation_specs() {
            self.block.add_code(annotation.emit_with(self.lang)?);
            self.block.add_line();
        }
        for annotation in self.function.annotations() {
            self.block.add_code(annotation.clone());
            self.block.add_line();
        }

        if self.function.modifiers().is_override {
            let annotation = self.lang.function_syntax().override_annotation;
            if !annotation.is_empty() {
                self.block.add("%L", annotation.to_string());
                self.block.add_line();
            }
        }

        if !self.lang.doc_before_annotations()
            && let Some(doc) = emit_doc()
        {
            self.block.add("%L", doc);
            self.block.add_line();
        }

        Ok(())
    }

    fn push_type_params(&mut self) -> Result<bool, SigilStitchError> {
        let type_params = type_params_for_rendering(self.lang, self.function)?;
        let rendered =
            render_type_params_for(type_params.as_ref(), self.lang, &mut self.signature.args);
        let present = !rendered.is_empty();
        self.signature.format.push_str(&rendered);
        Ok(present)
    }

    fn push_receiver(&mut self) {
        let Some(receiver) = self.function.receiver() else {
            return;
        };
        self.signature.push_literal("(");
        self.signature.push_literal(self.lang.variable_prefix());
        self.signature
            .push_literal(&self.lang.escape_reserved(receiver.name()));
        self.signature
            .push_literal(self.lang.type_decl_syntax().type_annotation_separator);
        self.signature.push_type(receiver.param_type());
        self.signature.push_literal(") ");
    }

    fn push_parameters(
        &mut self,
        style: ParamListStyle,
        readonly_property_keyword: &str,
        mutable_property_keyword: &str,
    ) -> Result<(), SigilStitchError> {
        let emit = |block: &mut CodeBlockBuilder, parameter: &ParameterSpec| {
            emit_parameter(
                block,
                self.lang,
                parameter,
                readonly_property_keyword,
                mutable_property_keyword,
            );
        };
        match style {
            ParamListStyle::Tupled => {
                self.signature.push_literal("(");
                self.signature
                    .push_code(tupled_parameter_list(self.function.parameters(), emit)?);
                self.signature.push_literal(")");
            }
            ParamListStyle::Curried if !self.function.parameters().is_empty() => {
                self.signature.push_literal(" ");
                self.signature
                    .push_code(curried_parameter_list(self.function.parameters(), emit)?);
            }
            ParamListStyle::Curried => {}
        }
        Ok(())
    }

    fn append_suffixes(&mut self) {
        for suffix in self.function.suffixes() {
            self.signature.push_literal(" ");
            self.signature.push_literal(suffix);
        }
    }

    fn push_signature_delegation(&mut self, prefix: &str) {
        if let Some(delegation) = self.function.delegation() {
            self.signature.push_literal(prefix);
            self.signature.push_code(delegation.clone());
        }
    }

    fn finish(
        mut self,
        delegation_in_body: bool,
        syntax_requires_body: bool,
    ) -> Result<CodeBlock, SigilStitchError> {
        let emit_delegation_in_body = delegation_in_body
            && (self.function.body().is_some()
                || !self.lang.capabilities().function_validation_is_permissive());

        if let Some(body) = self.function.body() {
            self.push_where_and_open();
            self.signature.append_to(&mut self.block);
            self.block.add_line();
            emit_body_interior(
                &mut self.block,
                self.lang,
                self.function,
                emit_delegation_in_body,
                |block| {
                    block.add_code(body.clone());
                    if !body.ends_with_newline_or_block_close() {
                        block.add_line();
                    }
                },
            );
        } else {
            let empty_body = self.lang.function_syntax().empty_body;
            if !empty_body.is_empty() || emit_delegation_in_body || syntax_requires_body {
                self.push_where_and_open();
                self.signature.append_to(&mut self.block);
                self.block.add_line();
                emit_body_interior(
                    &mut self.block,
                    self.lang,
                    self.function,
                    emit_delegation_in_body,
                    |block| {
                        if !empty_body.is_empty() {
                            block.add_statement(empty_body, ());
                        }
                    },
                );
            } else {
                if self.lang.block_syntax().uses_semicolons {
                    self.signature.push_literal(";");
                }
                self.signature.append_to(&mut self.block);
                self.block.add_line();
            }
        }

        self.block.build()
    }

    fn push_where_and_open(&mut self) {
        let style = self.lang.function_syntax().where_clause_style;
        let constraints = self.function.where_constraints();
        if constraints.is_empty() || style == WhereClauseStyle::Inline {
            self.signature.push_literal(self.lang.fun_block_open());
            return;
        }

        match style {
            WhereClauseStyle::WhereBlock => emit_where_block(
                &mut self.signature.format,
                &mut self.signature.args,
                constraints,
                self.lang,
            ),
            WhereClauseStyle::SeparateWhere => emit_separate_where_block(
                &mut self.signature.format,
                &mut self.signature.args,
                constraints,
                self.lang,
            ),
            WhereClauseStyle::Inline => unreachable!(),
        }
        self.signature.push_literal("\n{");
    }
}

pub(crate) fn lower<L: CodeLang + ?Sized>(
    lang: &L,
    function: ValidatedFunction<'_>,
) -> Result<CodeBlock, SigilStitchError> {
    let mut lowering = CompatibilityLowering::new(lang, function);
    lowering.emit_preamble()?;
    if lang.function_syntax().function_signature_style == FunctionSignatureStyle::Split {
        return lower_split_signature(lang, function, lowering.block);
    }

    lowering.signature.push_literal(lang.render_visibility(
        function.modifiers().visibility,
        function.declaration_context(),
    ));
    if function.modifiers().is_abstract {
        lowering
            .signature
            .push_literal(lang.function_syntax().abstract_keyword);
    }
    if function.modifiers().is_static {
        lowering
            .signature
            .push_literal(lang.function_syntax().static_keyword);
    }
    if function.modifiers().is_override {
        lowering
            .signature
            .push_literal(lang.function_syntax().override_keyword);
    }

    let suppress_async = function.declaration_context() == DeclarationContext::InterfaceMember
        && lang.function_syntax().suppress_async_in_interface;
    if function.modifiers().is_async && !suppress_async {
        lowering
            .signature
            .push_literal(lang.function_syntax().async_keyword);
    }

    if lang.function_syntax().type_params_before_return_type && lowering.push_type_params()? {
        lowering.signature.push_literal(" ");
    }

    if lang.type_decl_syntax().return_type_is_prefix
        && let Some(return_type) = function.return_type()
    {
        lowering.signature.push_type(return_type);
        lowering.signature.push_literal(" ");
    }

    let keyword = if function.form() == FunctionForm::Constructor {
        lang.function_syntax().constructor_keyword
    } else {
        lang.function_keyword(function.declaration_context())
    };
    if !keyword.is_empty() {
        lowering.signature.push_literal(keyword);
        lowering.signature.push_literal(" ");
    }

    lowering.push_receiver();
    lowering.signature.push_literal(function.name());
    if !lang.function_syntax().type_params_before_return_type {
        lowering.push_type_params()?;
    }

    let parameter_config = lang.enum_and_annotation();
    lowering.push_parameters(
        lang.function_syntax().param_list_style,
        parameter_config.readonly_keyword,
        parameter_config.mutable_field_keyword,
    )?;
    lowering.append_suffixes();

    if function.modifiers().is_async
        && !suppress_async
        && lang.function_syntax().async_suffix_before_return
    {
        lowering
            .signature
            .push_literal(lang.function_syntax().async_suffix);
    }

    if !lang.type_decl_syntax().return_type_is_prefix
        && let Some(return_type) = function.return_type()
    {
        let separator = lang.function_syntax().return_type_separator;
        if !separator.is_empty() {
            lowering.signature.push_literal(separator);
            lowering.signature.push_type(return_type);
        }
    }

    let delegation_in_body = match (
        function.delegation(),
        lang.function_syntax().constructor_delegation_style,
    ) {
        (Some(_), ConstructorDelegationStyle::Signature) => {
            lowering.push_signature_delegation(" : ");
            false
        }
        (Some(_), ConstructorDelegationStyle::Body) => true,
        (None, _) => false,
    };

    if function.modifiers().is_async
        && !suppress_async
        && !lang.function_syntax().async_suffix_before_return
    {
        lowering
            .signature
            .push_literal(lang.function_syntax().async_suffix);
    }

    lowering.finish(delegation_in_body, false)
}

fn type_params_for_rendering<'a, L: CodeLang + ?Sized>(
    lang: &L,
    function: ValidatedFunction<'a>,
) -> Result<Cow<'a, [TypeParamSpec]>, SigilStitchError> {
    if lang.capabilities().function_validation_is_permissive()
        || function.where_constraints().is_empty()
        || lang.function_syntax().where_clause_style != WhereClauseStyle::Inline
    {
        Ok(Cow::Borrowed(function.type_params()))
    } else {
        type_params_with_inline_constraints(function, lang.file_extension())
    }
}

fn emit_parameter<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    parameter: &ParameterSpec,
    readonly_property_keyword: &str,
    mutable_property_keyword: &str,
) {
    let property_keyword = if parameter.is_property() {
        readonly_property_keyword
    } else if parameter.is_mutable_property() {
        mutable_property_keyword
    } else {
        ""
    };

    if lang.type_decl_syntax().type_before_name {
        if !parameter.param_type().is_empty() {
            block.add("%T ", parameter.param_type().clone());
        }
        block.add(
            "%L%L%L",
            (
                property_keyword,
                lang.variable_prefix(),
                lang.escape_reserved(parameter.name()),
            ),
        );
    } else {
        if parameter.is_variadic() {
            block.add("...", ());
        }
        block.add(
            "%L%L%L",
            (
                property_keyword,
                lang.variable_prefix(),
                lang.escape_reserved(parameter.name()),
            ),
        );
        if !parameter.param_type().is_empty() {
            block.add(
                "%L%T",
                (
                    lang.type_decl_syntax().type_annotation_separator,
                    parameter.param_type().clone(),
                ),
            );
        }
    }

    if let Some(default) = parameter.default_value() {
        block.add(" = %L", default.clone());
    }
}

fn emit_body_interior<L: CodeLang + ?Sized>(
    block: &mut CodeBlockBuilder,
    lang: &L,
    function: ValidatedFunction<'_>,
    delegation_in_body: bool,
    emit_content: impl FnOnce(&mut CodeBlockBuilder),
) {
    block.add("%>", ());
    if !function.doc().is_empty() && lang.doc_comment_inside_body() {
        let doc_lines: Vec<&str> = function.doc().iter().map(String::as_str).collect();
        block.add("%L", lang.render_doc_comment(&doc_lines));
        block.add_line();
    }
    if delegation_in_body && let Some(delegation) = function.delegation() {
        block.add_statement("%L", delegation.clone());
    }
    emit_content(block);
    block.add("%<", ());
    push_block_close(block, lang);
}

fn lower_split_signature<L: CodeLang + ?Sized>(
    lang: &L,
    function: ValidatedFunction<'_>,
    mut block: CodeBlockBuilder,
) -> Result<CodeBlock, SigilStitchError> {
    let type_params = type_params_for_rendering(lang, function)?;
    let context = lang.emit_type_context(type_params.as_ref())?;

    let emit_signature = if lang.capabilities().function_validation_is_permissive() {
        !function.parameters().is_empty() || function.return_type().is_some()
    } else {
        function.return_type().is_some()
    };
    if emit_signature {
        block.add(&format!("{} :: ", function.name()), ());
        if let Some(context) = context {
            block.add_code(context);
        }
        for (index, parameter) in function.parameters().iter().enumerate() {
            if index > 0 {
                block.add(" -> ", ());
            }
            block.add("%T", parameter.param_type().clone());
        }
        if let Some(return_type) = function.return_type() {
            if !function.parameters().is_empty() {
                block.add(" -> ", ());
            }
            block.add("%T", return_type.clone());
        }
        block.add_line();
    }

    let mut definition = function.name().to_string();
    for parameter in function.parameters() {
        definition.push(' ');
        definition.push_str(lang.variable_prefix());
        definition.push_str(&lang.escape_reserved(parameter.name()));
    }
    definition.push_str(lang.block_syntax().block_open);

    if let Some(body) = function.body() {
        block.add(&definition, ());
        block.add_line();
        block.add("%>", ());
        block.add_code(body.clone());
        if !body.ends_with_newline_or_block_close() {
            block.add_line();
        }
        block.add("%<", ());
        push_block_close(&mut block, lang);
    } else {
        let empty_body = lang.function_syntax().empty_body;
        if !empty_body.is_empty() {
            block.add(&definition, ());
            block.add_line();
            block.add("%>", ());
            block.add_statement(empty_body, ());
            block.add("%<", ());
            push_block_close(&mut block, lang);
        }
    }

    block.build()
}

fn push_block_close<L: CodeLang + ?Sized>(block: &mut CodeBlockBuilder, lang: &L) {
    let close = lang.block_syntax().block_close;
    if !close.is_empty() {
        block.add(close, ());
        block.add_line();
    }
}
