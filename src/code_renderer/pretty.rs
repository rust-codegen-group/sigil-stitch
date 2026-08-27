use ::pretty::BoxDoc;

use super::{LayoutGroup, RenderAdapter};
use crate::error::SigilStitchError;

struct DocumentGroup {
    doc: BoxDoc<'static, ()>,
    layout: LayoutGroup,
}

pub(super) struct PrettyAdapter {
    indent_unit: String,
    width: usize,
    docs: Vec<DocumentGroup>,
    indent_depth: usize,
    at_line_start: bool,
    pending_soft_break: Option<(String, LayoutGroup)>,
}

impl PrettyAdapter {
    pub(super) fn new(indent_unit: &str, width: usize) -> Self {
        Self {
            indent_unit: indent_unit.to_string(),
            width,
            docs: vec![DocumentGroup {
                doc: BoxDoc::nil(),
                layout: LayoutGroup::IndependentBreaks,
            }],
            indent_depth: 0,
            at_line_start: true,
            pending_soft_break: None,
        }
    }

    fn append_doc(&mut self, doc: BoxDoc<'static, ()>) -> Result<(), SigilStitchError> {
        let current = self
            .docs
            .last_mut()
            .ok_or_else(|| SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "missing document group".to_string(),
            })?;
        current.doc = std::mem::replace(&mut current.doc, BoxDoc::nil()).append(doc);
        Ok(())
    }

    fn flush_soft_break(&mut self, indent_if_broken: bool) -> Result<(), SigilStitchError> {
        let Some((indent, layout)) = self.pending_soft_break.take() else {
            return Ok(());
        };
        let broken = if indent_if_broken {
            BoxDoc::hardline().append(BoxDoc::text(indent))
        } else {
            BoxDoc::hardline()
        };
        let soft_break = broken.flat_alt(BoxDoc::space());
        match layout {
            LayoutGroup::IndependentBreaks => self.append_doc(soft_break.group()),
            LayoutGroup::ConsistentBreaks => self.append_doc(soft_break),
        }
    }

    pub(super) fn finish(mut self) -> Result<String, SigilStitchError> {
        self.flush_soft_break(false)?;
        if self.docs.len() != 1 {
            return Err(SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "unclosed document group".to_string(),
            });
        }
        let group = self.docs.pop().ok_or_else(|| SigilStitchError::Render {
            context: "CodeRenderer pretty groups".to_string(),
            message: "missing root document".to_string(),
        })?;
        let mut buf = Vec::new();
        group
            .doc
            .render(self.width, &mut buf)
            .map_err(|error| SigilStitchError::Render {
                context: "CodeRenderer pretty output".to_string(),
                message: error.to_string(),
            })?;
        String::from_utf8(buf).map_err(|error| SigilStitchError::Render {
            context: "CodeRenderer pretty output UTF-8".to_string(),
            message: error.to_string(),
        })
    }
}

impl RenderAdapter for PrettyAdapter {
    fn raw_text(&mut self, text: &str) -> Result<(), SigilStitchError> {
        debug_assert!(!text.contains('\n'));
        if !text.is_empty() {
            self.flush_soft_break(true)?;
            self.append_doc(BoxDoc::text(text.to_string()))?;
            self.at_line_start = false;
        }
        Ok(())
    }

    fn ensure_indent(&mut self) -> Result<(), SigilStitchError> {
        self.flush_soft_break(true)?;
        if self.at_line_start {
            let indent = self.indent_unit.repeat(self.indent_depth);
            if !indent.is_empty() {
                self.append_doc(BoxDoc::text(indent))?;
            }
            self.at_line_start = false;
        }
        Ok(())
    }

    fn hard_break(&mut self) -> Result<(), SigilStitchError> {
        self.flush_soft_break(false)?;
        self.append_doc(BoxDoc::hardline())?;
        self.at_line_start = true;
        Ok(())
    }

    fn soft_break(&mut self) -> Result<(), SigilStitchError> {
        self.flush_soft_break(false)?;
        let layout =
            self.docs
                .last()
                .map(|group| group.layout)
                .ok_or_else(|| SigilStitchError::Render {
                    context: "CodeRenderer pretty groups".to_string(),
                    message: "missing document group".to_string(),
                })?;
        self.pending_soft_break = Some((self.indent_unit.repeat(self.indent_depth), layout));
        self.at_line_start = false;
        Ok(())
    }

    fn indent(&mut self) -> Result<(), SigilStitchError> {
        self.indent_depth =
            self.indent_depth
                .checked_add(1)
                .ok_or_else(|| SigilStitchError::Render {
                    context: "CodeRenderer pretty indentation".to_string(),
                    message: "indent depth overflow".to_string(),
                })?;
        Ok(())
    }

    fn dedent(&mut self) -> Result<(), SigilStitchError> {
        self.indent_depth =
            self.indent_depth
                .checked_sub(1)
                .ok_or_else(|| SigilStitchError::Render {
                    context: "CodeRenderer pretty indentation".to_string(),
                    message: "dedent below zero".to_string(),
                })?;
        Ok(())
    }

    fn begin_group(&mut self, layout: LayoutGroup) -> Result<(), SigilStitchError> {
        self.flush_soft_break(true)?;
        self.docs.push(DocumentGroup {
            doc: BoxDoc::nil(),
            layout,
        });
        Ok(())
    }

    fn end_group(&mut self) -> Result<(), SigilStitchError> {
        if self.docs.len() <= 1 {
            return Err(SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "group end without group begin".to_string(),
            });
        }
        self.flush_soft_break(false)?;
        let group = self.docs.pop().ok_or_else(|| SigilStitchError::Render {
            context: "CodeRenderer pretty groups".to_string(),
            message: "missing completed document group".to_string(),
        })?;
        self.append_doc(group.doc.group())
    }
}
