use ::pretty::BoxDoc;

use super::RenderAdapter;
use crate::error::SigilStitchError;

pub(super) struct PrettyAdapter {
    indent_unit: String,
    width: usize,
    docs: Vec<BoxDoc<'static, ()>>,
    indent_depth: usize,
    at_line_start: bool,
    pending_soft_break_indent: Option<String>,
}

impl PrettyAdapter {
    pub(super) fn new(indent_unit: &str, width: usize) -> Self {
        Self {
            indent_unit: indent_unit.to_string(),
            width,
            docs: vec![BoxDoc::nil()],
            indent_depth: 0,
            at_line_start: true,
            pending_soft_break_indent: None,
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
        *current = std::mem::replace(current, BoxDoc::nil()).append(doc);
        Ok(())
    }

    fn flush_soft_break(&mut self, indent_if_broken: bool) -> Result<(), SigilStitchError> {
        let Some(indent) = self.pending_soft_break_indent.take() else {
            return Ok(());
        };
        let broken = if indent_if_broken {
            BoxDoc::hardline().append(BoxDoc::text(indent))
        } else {
            BoxDoc::hardline()
        };
        self.append_doc(broken.flat_alt(BoxDoc::space()).group())
    }

    pub(super) fn finish(mut self) -> Result<String, SigilStitchError> {
        self.flush_soft_break(false)?;
        if self.docs.len() != 1 {
            return Err(SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "unclosed document group".to_string(),
            });
        }
        let doc = self.docs.pop().ok_or_else(|| SigilStitchError::Render {
            context: "CodeRenderer pretty groups".to_string(),
            message: "missing root document".to_string(),
        })?;
        let mut buf = Vec::new();
        doc.render(self.width, &mut buf)
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
        self.pending_soft_break_indent = Some(self.indent_unit.repeat(self.indent_depth));
        self.at_line_start = false;
        Ok(())
    }

    fn type_doc(&mut self, doc: BoxDoc<'static, ()>) -> Result<(), SigilStitchError> {
        self.ensure_indent()?;
        let width = self.width;
        let indent = self.indent_unit.repeat(self.indent_depth);
        let column_doc = BoxDoc::column(move |column| {
            let mut buf = Vec::new();
            if doc.render(width.saturating_sub(column), &mut buf).is_err() {
                return BoxDoc::fail();
            }
            let Ok(rendered) = String::from_utf8(buf) else {
                return BoxDoc::fail();
            };
            type_lines_to_doc(&rendered, &indent)
        });
        self.append_doc(column_doc)?;
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

    fn begin_group(&mut self) -> Result<(), SigilStitchError> {
        self.docs.push(BoxDoc::nil());
        Ok(())
    }

    fn end_group(&mut self) -> Result<(), SigilStitchError> {
        if self.docs.len() <= 1 {
            return Err(SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "group end without group begin".to_string(),
            });
        }
        let doc = self.docs.pop().ok_or_else(|| SigilStitchError::Render {
            context: "CodeRenderer pretty groups".to_string(),
            message: "missing completed document group".to_string(),
        })?;
        self.append_doc(doc.group())
    }
}

fn type_lines_to_doc(rendered: &str, indent: &str) -> BoxDoc<'static, ()> {
    let mut lines = rendered.split('\n');
    let mut doc = lines
        .next()
        .map(|line| BoxDoc::text(line.to_string()))
        .unwrap_or_else(BoxDoc::nil);
    for line in lines {
        doc = doc
            .append(BoxDoc::hardline())
            .append(BoxDoc::text(indent.to_string()))
            .append(BoxDoc::text(line.to_string()));
    }
    doc
}
