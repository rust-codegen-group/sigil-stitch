use unicode_width::UnicodeWidthStr;

use super::{LayoutGroup, RenderAdapter};
use crate::error::SigilStitchError;

pub(super) struct DirectAdapter<'a> {
    indent_unit: &'a str,
    output: String,
    indent_depth: usize,
    current_column: usize,
    at_line_start: bool,
}

impl<'a> DirectAdapter<'a> {
    pub(super) fn new(indent_unit: &'a str, _width: usize) -> Self {
        Self {
            indent_unit,
            output: String::new(),
            indent_depth: 0,
            current_column: 0,
            at_line_start: true,
        }
    }

    pub(super) fn finish(self) -> String {
        self.output
    }
}

impl RenderAdapter for DirectAdapter<'_> {
    fn raw_text(&mut self, text: &str) -> Result<(), SigilStitchError> {
        debug_assert!(!text.contains('\n'));
        self.output.push_str(text);
        self.current_column += UnicodeWidthStr::width(text);
        if !text.is_empty() {
            self.at_line_start = false;
        }
        Ok(())
    }

    fn ensure_indent(&mut self) -> Result<(), SigilStitchError> {
        if self.at_line_start {
            let indent = self.indent_unit.repeat(self.indent_depth);
            self.output.push_str(&indent);
            self.current_column += UnicodeWidthStr::width(indent.as_str());
            self.at_line_start = false;
        }
        Ok(())
    }

    fn hard_break(&mut self) -> Result<(), SigilStitchError> {
        self.output.push('\n');
        self.current_column = 0;
        self.at_line_start = true;
        Ok(())
    }

    fn soft_break(&mut self) -> Result<(), SigilStitchError> {
        self.raw_text(" ")
    }

    fn indent(&mut self) -> Result<(), SigilStitchError> {
        self.indent_depth =
            self.indent_depth
                .checked_add(1)
                .ok_or_else(|| SigilStitchError::Render {
                    context: "CodeRenderer direct indentation".to_string(),
                    message: "indent depth overflow".to_string(),
                })?;
        Ok(())
    }

    fn dedent(&mut self) -> Result<(), SigilStitchError> {
        self.indent_depth =
            self.indent_depth
                .checked_sub(1)
                .ok_or_else(|| SigilStitchError::Render {
                    context: "CodeRenderer direct indentation".to_string(),
                    message: "dedent below zero".to_string(),
                })?;
        Ok(())
    }

    fn begin_group(&mut self, _group: LayoutGroup) -> Result<(), SigilStitchError> {
        Ok(())
    }

    fn end_group(&mut self) -> Result<(), SigilStitchError> {
        Ok(())
    }
}
