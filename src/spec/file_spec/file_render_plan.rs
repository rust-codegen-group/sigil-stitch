use crate::code_block::CodeBlock;
use crate::code_renderer::CodeRenderer;
use crate::error::SigilStitchError;
use crate::import::{ImportGroup, ImportRef};
use crate::import_collector;
use crate::lang::CodeLang;
use crate::source_preparation::SourcePreparer;
use crate::spec::file_spec::{FileSpec, StoredFileMember};
use crate::spec::modifiers::DeclarationContext;
use crate::type_name::TypeName;
use crate::type_name_lowering::DiagnosticPath;

pub(super) struct FileRenderPlan {
    header: Option<CodeBlock>,
    members: Vec<PreparedMember>,
}

enum EmittedMember {
    Blocks(Vec<CodeBlock>),
    Raw(String),
    RawWithImports {
        content: String,
        types: Vec<TypeName>,
    },
}

enum PreparedMember {
    Blocks(Vec<CodeBlock>),
    Raw(String),
    RawWithImports {
        content: String,
        metadata: Vec<CodeBlock>,
    },
}

impl FileRenderPlan {
    pub(super) fn prepare(file: &FileSpec, lang: &dyn CodeLang) -> Result<Self, SigilStitchError> {
        let emitted = materialize_members(&file.members, lang)?;
        let preparer = SourcePreparer::new(lang);
        let header = file
            .header
            .as_ref()
            .map(|header| preparer.prepare_source_block(header, DiagnosticPath::root("header")))
            .transpose()?;
        let mut members = Vec::with_capacity(emitted.len());
        for (member_index, member) in emitted.into_iter().enumerate() {
            members.push(prepare_member(member, member_index, &preparer)?);
        }

        Ok(Self { header, members })
    }

    pub(super) fn collect_imports(&self) -> Vec<ImportRef> {
        let mut import_refs = Vec::new();

        if let Some(header) = &self.header {
            import_refs.extend(import_collector::collect_imports(header));
        }

        for member in &self.members {
            match member {
                PreparedMember::Blocks(blocks) => {
                    for block in blocks {
                        import_refs.extend(import_collector::collect_imports(block));
                    }
                }
                PreparedMember::RawWithImports { metadata, .. } => {
                    for block in metadata {
                        import_refs.extend(import_collector::collect_imports(block));
                    }
                }
                PreparedMember::Raw(_) => {}
            }
        }

        import_refs
    }

    pub(super) fn render(
        &self,
        lang: &dyn CodeLang,
        imports: &ImportGroup,
        width: usize,
    ) -> Result<String, SigilStitchError> {
        let mut output = String::new();
        let renderer = CodeRenderer::new(lang, imports, width);

        if let Some(header) = &self.header {
            let header_output = renderer.render_prepared(header)?;
            if !header_output.is_empty() {
                output.push_str(&header_output);
                if !header_output.ends_with('\n') {
                    output.push('\n');
                }
                output.push('\n');
            }
        }

        let import_header = lang.render_imports(imports);
        if !import_header.is_empty() {
            output.push_str(&import_header);
            output.push('\n');
            if self.members.is_empty() {
                output.push('\n');
            } else {
                output.push_str(lang.top_level_member_separator());
            }
        }

        for (member_index, member) in self.members.iter().enumerate() {
            if member_index > 0 {
                output.push_str(lang.top_level_member_separator());
            }
            match member {
                PreparedMember::Blocks(blocks) => {
                    for (block_index, block) in blocks.iter().enumerate() {
                        if block_index > 0 {
                            output.push_str(lang.top_level_member_separator());
                        }
                        let member_output = renderer.render_prepared(block)?;
                        append_member_output(
                            &mut output,
                            &member_output,
                            lang.trim_trailing_blank_lines(),
                        );
                    }
                }
                PreparedMember::Raw(content) => {
                    append_member_output(&mut output, content, false);
                }
                PreparedMember::RawWithImports { content, .. } => {
                    append_member_output(&mut output, content, false);
                }
            }
        }

        Ok(output)
    }
}

fn append_member_output(output: &mut String, content: &str, trim_trailing_blank_lines: bool) {
    let content = if trim_trailing_blank_lines {
        content.trim_end_matches('\n')
    } else {
        content
    };
    output.push_str(content);
    if !output.ends_with('\n') {
        output.push('\n');
    }
}

fn materialize_members(
    members: &[StoredFileMember],
    lang: &dyn CodeLang,
) -> Result<Vec<EmittedMember>, SigilStitchError> {
    let mut emitted = Vec::with_capacity(members.len());
    for member in members {
        emitted.push(match member {
            StoredFileMember::Code(block) => EmittedMember::Blocks(vec![block.clone()]),
            StoredFileMember::RawContent(content) => EmittedMember::Raw(content.clone()),
            StoredFileMember::RawContentWithImports { content, types } => {
                EmittedMember::RawWithImports {
                    content: content.clone(),
                    types: types.clone(),
                }
            }
            StoredFileMember::Type(spec) => EmittedMember::Blocks(spec.emit(lang)?),
            StoredFileMember::ClosedSum(spec) => EmittedMember::Blocks(spec.emit(lang)?),
            StoredFileMember::Fun(spec) => {
                EmittedMember::Blocks(vec![spec.emit(lang, DeclarationContext::TopLevel)?])
            }
            StoredFileMember::Spec(spec) => EmittedMember::Blocks(spec.emit_members(lang)?),
        });
    }
    Ok(emitted)
}

fn prepare_member(
    member: EmittedMember,
    member_index: usize,
    preparer: &SourcePreparer<'_>,
) -> Result<PreparedMember, SigilStitchError> {
    match member {
        EmittedMember::Blocks(blocks) => Ok(PreparedMember::Blocks(
            blocks
                .iter()
                .enumerate()
                .map(|(block_index, block)| {
                    preparer.prepare_source_block(
                        block,
                        DiagnosticPath::member_block(member_index, block_index),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        EmittedMember::Raw(content) => Ok(PreparedMember::Raw(content)),
        EmittedMember::RawWithImports { content, types } => {
            let metadata = types
                .iter()
                .enumerate()
                .map(|(type_index, type_name)| {
                    preparer.prepare_metadata_type(
                        type_name,
                        DiagnosticPath::raw_metadata(member_index, type_index),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PreparedMember::RawWithImports { content, metadata })
        }
    }
}
