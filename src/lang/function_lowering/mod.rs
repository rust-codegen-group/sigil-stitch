//! Policy-free structural helpers for function declaration lowering.
//!
//! Language adapters choose every token and its relative order. These helpers
//! only keep `TypeName` and nested `CodeBlock` values structured while building
//! format strings. The frozen compatibility module is the sole interpreter of
//! pre-0.6.8 declaration configuration.

mod compatibility;

use std::borrow::Cow;

use crate::code_block::{Arg, CodeBlock, CodeBlockBuilder};
use crate::error::SigilStitchError;
use crate::spec::fun_spec::ValidatedFunction;
use crate::spec::parameter_spec::ParameterSpec;
use crate::spec::where_spec::{TypeParamSpec, WhereConstraint};
use crate::type_name::TypeName;

pub(crate) use compatibility::lower as lower_compatibility;

/// Structured signature accumulator with no language or grammar policy.
pub(crate) struct SignatureBuilder {
    format: String,
    args: Vec<Arg>,
}

impl SignatureBuilder {
    pub(crate) fn new() -> Self {
        Self {
            format: String::new(),
            args: Vec::new(),
        }
    }

    pub(crate) fn push_literal(&mut self, literal: &str) {
        self.format.push_str(literal);
    }

    pub(crate) fn push_type(&mut self, type_name: &TypeName) {
        self.format.push_str("%T");
        self.args.push(Arg::TypeName(type_name.clone()));
    }

    pub(crate) fn push_code(&mut self, code: CodeBlock) {
        self.format.push_str("%L");
        self.args.push(Arg::Code(code));
    }

    pub(crate) fn append_to(self, block: &mut CodeBlockBuilder) {
        block.add(&self.format, self.args);
    }
}

pub(crate) fn tupled_parameter_list(
    parameters: &[ParameterSpec],
    mut emit_parameter: impl FnMut(&mut CodeBlockBuilder, &ParameterSpec),
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    block.add("%>", ());
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            block.add(",%W", ());
        }
        emit_parameter(&mut block, parameter);
    }
    block.add("%<", ());
    block.build()
}

pub(crate) fn curried_parameter_list(
    parameters: &[ParameterSpec],
    mut emit_parameter: impl FnMut(&mut CodeBlockBuilder, &ParameterSpec),
) -> Result<CodeBlock, SigilStitchError> {
    let mut block = CodeBlock::builder();
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            block.add(" ", ());
        }
        block.add("(", ());
        emit_parameter(&mut block, parameter);
        block.add(")", ());
    }
    block.build()
}

/// Validate constraints for a language that lowers every subject onto a
/// declared type parameter.
///
/// The caller owns the grammar decision to use this helper. The helper only
/// checks the structural precondition shared by those lowering strategies.
pub(crate) fn validate_constraints_target_declared_type_params(
    language: &str,
    function_name: &str,
    type_params: &[TypeParamSpec],
    constraints: &[WhereConstraint],
) -> Result<(), SigilStitchError> {
    for constraint in constraints {
        let Some(subject) = constraint.parameter_subject_name() else {
            return Err(SigilStitchError::InvalidFunctionConstraintSubject {
                language: language.to_string(),
                function_name: function_name.to_string(),
                subject: format!("{:?}", constraint.subject()),
            });
        };
        if !type_params.iter().any(|param| param.name() == subject) {
            return Err(SigilStitchError::InvalidFunctionConstraintSubject {
                language: language.to_string(),
                function_name: function_name.to_string(),
                subject: subject.to_string(),
            });
        }
    }
    Ok(())
}

/// Merge explicit where constraints into declared type parameters.
///
/// Calling this helper is a language-local grammar decision; the helper only
/// performs the checked semantic transformation and preserves `TypeName`s.
pub(crate) fn type_params_with_inline_constraints<'a>(
    function: ValidatedFunction<'a>,
    language: &str,
) -> Result<Cow<'a, [TypeParamSpec]>, SigilStitchError> {
    if function.where_constraints().is_empty() {
        return Ok(Cow::Borrowed(function.type_params()));
    }

    let mut type_params = function.type_params().to_vec();
    for constraint in function.where_constraints() {
        let Some(subject) = constraint.parameter_subject_name() else {
            return Err(SigilStitchError::InvalidFunctionConstraintSubject {
                language: language.to_string(),
                function_name: function.name().to_string(),
                subject: format!("{:?}", constraint.subject()),
            });
        };
        let Some(type_param) = type_params.iter_mut().find(|param| param.name() == subject) else {
            return Err(SigilStitchError::InvalidFunctionConstraintSubject {
                language: language.to_string(),
                function_name: function.name().to_string(),
                subject: subject.to_string(),
            });
        };
        for bound in constraint.bounds() {
            if !type_param.bounds.contains(bound) {
                type_param.bounds.push(bound.clone());
            }
        }
    }

    Ok(Cow::Owned(type_params))
}
