use std::rc::Rc;

use ::pretty::{BoxDoc, Doc};

use super::{LayoutGroup, RenderAdapter};
use crate::code_node::{
    LayoutBlock, LayoutBreakMode, LayoutEdge, LayoutGroupMode, TrailingCommaPolicy,
};
use crate::error::SigilStitchError;
use crate::lang::CommentLayout;

struct DocumentGroup {
    doc: BoxDoc<'static, ()>,
    layout: LayoutGroup,
    break_mode: LayoutBreakMode,
    is_layout: bool,
    conditional_indent: usize,
    force_break: bool,
    preferred: Option<PreferredEdges>,
}

#[derive(Default)]
struct PreferredEdges {
    open: Option<(BoxDoc<'static, ()>, usize)>,
    close: Option<BoxDoc<'static, ()>>,
}

struct PendingBreak {
    depth: usize,
    layout: LayoutGroup,
    mode: LayoutBreakMode,
    force_break: bool,
}

pub(super) struct PrettyAdapter {
    indent_unit: Rc<str>,
    width: usize,
    docs: Vec<DocumentGroup>,
    indent_depth: usize,
    at_line_start: bool,
    pending_soft_break: Option<PendingBreak>,
}

impl PrettyAdapter {
    pub(super) fn new(indent_unit: &str, width: usize) -> Self {
        Self {
            indent_unit: Rc::from(indent_unit),
            width,
            docs: vec![DocumentGroup {
                doc: BoxDoc::nil(),
                layout: LayoutGroup::IndependentBreaks,
                break_mode: LayoutBreakMode::Space,
                is_layout: false,
                conditional_indent: 0,
                force_break: false,
                preferred: None,
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
        let Some(pending) = self.pending_soft_break.take() else {
            return Ok(());
        };
        let broken = self.line_break(
            indent_if_broken.then_some(pending.depth),
            self.in_preferred_scope(),
        );
        let flat = match pending.mode {
            LayoutBreakMode::Space => BoxDoc::space(),
            LayoutBreakMode::Empty => BoxDoc::nil(),
        };
        let soft_break = if pending.force_break {
            broken
        } else {
            broken.flat_alt(flat)
        };
        match pending.layout {
            LayoutGroup::IndependentBreaks => self.append_doc(soft_break.group()),
            LayoutGroup::ConsistentBreaks => self.append_doc(soft_break),
        }
    }

    fn in_preferred_scope(&self) -> bool {
        self.docs.iter().any(|group| group.preferred.is_some())
    }

    fn line_break(&self, depth: Option<usize>, projected: bool) -> BoxDoc<'static, ()> {
        if projected {
            let unit = Rc::clone(&self.indent_unit);
            BoxDoc::nesting(move |extra| {
                // The nesting parameter carries the number of active alternative
                // scopes, not a column count. A hardline otherwise borrows the
                // *next* command's nesting as space padding. Keep its next
                // command inside the reset, even when the line has no indent.
                let indent = match depth {
                    Some(depth) if !unit.is_empty() => BoxDoc::text(unit.repeat(depth + extra)),
                    _ => BoxDoc::new(Doc::BorrowedText("")),
                };
                BoxDoc::new(Doc::Append(BoxDoc::hardline(), indent)).nest(-(extra as isize))
            })
        } else {
            let line = BoxDoc::hardline();
            match depth {
                Some(depth) => line.append(BoxDoc::text(self.indent_unit.repeat(depth))),
                None => line,
            }
        }
    }

    fn indentation(&self, depth: usize) -> BoxDoc<'static, ()> {
        if self.in_preferred_scope() {
            let unit = Rc::clone(&self.indent_unit);
            BoxDoc::nesting(move |extra| BoxDoc::text(unit.repeat(depth + extra)))
        } else {
            BoxDoc::text(self.indent_unit.repeat(depth))
        }
    }

    fn continuation_depth(&self) -> Result<usize, SigilStitchError> {
        self.docs
            .iter()
            .filter(|group| group.preferred.is_none())
            .try_fold(self.indent_depth, |depth, group| {
                depth.checked_add(group.conditional_indent)
            })
            .ok_or_else(|| SigilStitchError::Render {
                context: "CodeRenderer pretty indentation".to_string(),
                message: "indent depth overflow".to_string(),
            })
    }

    fn queue_break(&mut self, mode: Option<LayoutBreakMode>) -> Result<(), SigilStitchError> {
        self.flush_soft_break(false)?;
        let group = self.docs.last().ok_or_else(|| SigilStitchError::Render {
            context: "CodeRenderer pretty groups".to_string(),
            message: "missing document group".to_string(),
        })?;
        self.pending_soft_break = Some(PendingBreak {
            depth: self.continuation_depth()?,
            layout: group.layout,
            mode: mode.unwrap_or(group.break_mode),
            force_break: group.force_break,
        });
        self.at_line_start = false;
        Ok(())
    }

    fn capture_preferred_edge(&mut self, edge: LayoutEdge) -> Result<(), SigilStitchError> {
        self.flush_soft_break(false)?;
        let (group, parents) = self
            .docs
            .split_last()
            .ok_or_else(|| SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "missing document group".to_string(),
            })?;
        let scopes = group.conditional_indent;
        let projected = parents.iter().any(|group| group.preferred.is_some());
        let depth = self
            .continuation_depth()?
            .checked_add(scopes)
            .ok_or_else(|| SigilStitchError::Render {
                context: "CodeRenderer pretty indentation".to_string(),
                message: "indent depth overflow".to_string(),
            })?;
        let doc = self.line_break(Some(depth), projected);
        let edges = self
            .docs
            .last_mut()
            .and_then(|group| group.preferred.as_mut())
            .ok_or_else(|| SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "missing preferred layout edges".to_string(),
            })?;
        match edge {
            LayoutEdge::Open => edges.open = Some((doc, scopes)),
            LayoutEdge::Close => edges.close = Some(doc),
        }
        Ok(())
    }

    fn append_preferred_layout(
        &mut self,
        layout: &LayoutBlock,
        body: BoxDoc<'static, ()>,
        edges: PreferredEdges,
    ) -> Result<(), SigilStitchError> {
        let (Some((open, scopes)), Some(close)) = (edges.open, edges.close) else {
            return Err(SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "incomplete preferred layout edges".to_string(),
            });
        };
        let preferred = BoxDoc::text(layout.flat_prefix().to_owned())
            .append(body.clone())
            .append(BoxDoc::text(layout.flat_suffix().to_owned()));
        let wrapped = BoxDoc::text(layout.broken_prefix().to_owned())
            .append(open)
            .append(body.nest(scopes as isize))
            .append(close)
            .append(BoxDoc::text(layout.broken_suffix().to_owned()));
        self.append_doc(preferred.union(wrapped))
    }

    #[cfg(test)]
    pub(super) fn set_indent_depth_for_test(&mut self, depth: usize) {
        self.indent_depth = depth;
    }

    #[cfg(test)]
    pub(super) fn set_layout_indent_for_test(&mut self, depth: usize) {
        if let Some(group) = self.docs.last_mut() {
            group.conditional_indent = depth;
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

    fn comment_text(&mut self, text: &str, layout: CommentLayout) -> Result<(), SigilStitchError> {
        if !matches!(layout, CommentLayout::WidthOpaque) {
            return self.structured_text(text);
        }
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                self.hard_break()?;
            }
            if !line.is_empty() {
                self.ensure_indent()?;
                // RenderLen must wrap a bare text node, including for Unicode.
                self.append_doc(BoxDoc::new(Doc::RenderLen(
                    0,
                    BoxDoc::new(Doc::OwnedText(line.into())),
                )))?;
                self.at_line_start = false;
            }
        }
        Ok(())
    }

    fn ensure_indent(&mut self) -> Result<(), SigilStitchError> {
        self.flush_soft_break(true)?;
        if self.at_line_start {
            self.append_doc(self.indentation(self.indent_depth))?;
            self.at_line_start = false;
        }
        Ok(())
    }

    fn hard_break(&mut self) -> Result<(), SigilStitchError> {
        self.flush_soft_break(false)?;
        self.append_doc(self.line_break(None, self.in_preferred_scope()))?;
        self.at_line_start = true;
        Ok(())
    }

    fn soft_break(&mut self) -> Result<(), SigilStitchError> {
        self.queue_break(None)
    }

    fn layout_edge(&mut self, edge: LayoutEdge) -> Result<(), SigilStitchError> {
        if self
            .docs
            .last()
            .is_some_and(|group| group.preferred.is_some())
        {
            self.capture_preferred_edge(edge)
        } else {
            self.queue_break(Some(LayoutBreakMode::Empty))
        }
    }

    fn indent(&mut self) -> Result<(), SigilStitchError> {
        let is_layout = self
            .docs
            .last()
            .map(|group| group.is_layout)
            .unwrap_or(false);
        if is_layout {
            let group = self
                .docs
                .last_mut()
                .ok_or_else(|| SigilStitchError::Render {
                    context: "CodeRenderer pretty groups".to_string(),
                    message: "missing document group".to_string(),
                })?;
            group.conditional_indent =
                group.conditional_indent.checked_add(1).ok_or_else(|| {
                    SigilStitchError::Render {
                        context: "CodeRenderer pretty indentation".to_string(),
                        message: "indent depth overflow".to_string(),
                    }
                })?;
        } else {
            self.indent_depth =
                self.indent_depth
                    .checked_add(1)
                    .ok_or_else(|| SigilStitchError::Render {
                        context: "CodeRenderer pretty indentation".to_string(),
                        message: "indent depth overflow".to_string(),
                    })?;
        }
        Ok(())
    }

    fn dedent(&mut self) -> Result<(), SigilStitchError> {
        let is_layout = self
            .docs
            .last()
            .map(|group| group.is_layout)
            .unwrap_or(false);
        if is_layout {
            let group = self
                .docs
                .last_mut()
                .ok_or_else(|| SigilStitchError::Render {
                    context: "CodeRenderer pretty groups".to_string(),
                    message: "missing document group".to_string(),
                })?;
            group.conditional_indent =
                group.conditional_indent.checked_sub(1).ok_or_else(|| {
                    SigilStitchError::Render {
                        context: "CodeRenderer pretty indentation".to_string(),
                        message: "dedent below zero".to_string(),
                    }
                })?;
        } else {
            self.indent_depth =
                self.indent_depth
                    .checked_sub(1)
                    .ok_or_else(|| SigilStitchError::Render {
                        context: "CodeRenderer pretty indentation".to_string(),
                        message: "dedent below zero".to_string(),
                    })?;
        }
        Ok(())
    }

    fn begin_group(&mut self, layout: LayoutGroup) -> Result<(), SigilStitchError> {
        self.flush_soft_break(true)?;
        self.docs.push(DocumentGroup {
            doc: BoxDoc::nil(),
            layout,
            break_mode: LayoutBreakMode::Space,
            is_layout: false,
            conditional_indent: 0,
            force_break: false,
            preferred: None,
        });
        Ok(())
    }

    fn begin_layout(&mut self, layout: &LayoutBlock) -> Result<(), SigilStitchError> {
        self.flush_soft_break(true)?;
        let group_layout = match layout.group_mode() {
            LayoutGroupMode::Independent => LayoutGroup::IndependentBreaks,
            LayoutGroupMode::Consistent => LayoutGroup::ConsistentBreaks,
        };
        self.docs.push(DocumentGroup {
            doc: BoxDoc::nil(),
            layout: group_layout,
            break_mode: layout.break_mode(),
            is_layout: true,
            conditional_indent: 0,
            force_break: layout.force_break(),
            preferred: layout.prefers_inner_breaks().then(PreferredEdges::default),
        });
        if !layout.prefers_inner_breaks() {
            self.append_doc(delimiter(
                layout.broken_prefix(),
                layout.flat_prefix(),
                layout.force_break(),
            ))?;
        }
        Ok(())
    }

    fn end_layout(&mut self, layout: &LayoutBlock) -> Result<(), SigilStitchError> {
        if self.docs.len() <= 1 {
            return Err(SigilStitchError::Render {
                context: "CodeRenderer pretty groups".to_string(),
                message: "layout end without layout begin".to_string(),
            });
        }
        if self
            .docs
            .last()
            .is_some_and(|group| group.is_layout && group.conditional_indent != 0)
        {
            return Err(SigilStitchError::Render {
                context: "CodeRenderer pretty indentation".to_string(),
                message: "unbalanced layout indentation scope".to_string(),
            });
        }
        if !layout.prefers_inner_breaks() {
            match layout.trailing_comma() {
                TrailingCommaPolicy::Never => {}
                TrailingCommaPolicy::IfBroken => {
                    self.append_doc(BoxDoc::text(",").flat_alt(BoxDoc::nil()))?
                }
                TrailingCommaPolicy::Always => self.append_doc(BoxDoc::text(","))?,
            }
        }
        self.flush_soft_break(true)?;
        if !layout.prefers_inner_breaks() {
            self.append_doc(delimiter(
                layout.broken_suffix(),
                layout.flat_suffix(),
                layout.force_break(),
            ))?;
        }
        let group = self.docs.pop().ok_or_else(|| SigilStitchError::Render {
            context: "CodeRenderer pretty groups".to_string(),
            message: "missing completed layout group".to_string(),
        })?;
        if let Some(edges) = group.preferred {
            return self.append_preferred_layout(layout, group.doc, edges);
        }
        let doc = if layout.force_break() {
            group.doc
        } else {
            group.doc.group()
        };
        self.append_doc(doc)
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

fn delimiter(broken: &str, flat: &str, force_break: bool) -> BoxDoc<'static, ()> {
    let broken = BoxDoc::text(broken.to_owned());
    if force_break {
        broken
    } else {
        broken.flat_alt(BoxDoc::text(flat.to_owned()))
    }
}
