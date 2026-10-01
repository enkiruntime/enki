use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gutter {
    pub width: usize,
}

impl Gutter {
    pub fn from_max_line(max_line: usize) -> Self {
        let width = if max_line == 0 {
            1
        } else {
            max_line.to_string().len()
        };
        Self { width }
    }

    pub fn arrow_header(&self, theme: &Theme, file_and_pos: &str) -> String {
        let pad = " ".repeat(self.width);
        format!("{pad}{} {file_and_pos}", theme.arrow())
    }

    pub fn empty_bar(&self, theme: &Theme) -> String {
        let pad = " ".repeat(self.width);
        format!("{pad} {}", theme.bar())
    }

    pub fn line_bar(&self, line_num: usize, theme: &Theme) -> String {
        let line_str = theme.line_number(line_num);
        let raw_len = line_num.to_string().len();
        let pad = " ".repeat(self.width.saturating_sub(raw_len));
        format!("{pad}{line_str} {}", theme.bar())
    }

    pub fn dots_bar(&self, theme: &Theme) -> String {
        let pad = " ".repeat(self.width.saturating_sub(1));
        format!("{pad}{}", theme.dots())
    }

    pub fn footnote_indent(&self) -> String {
        let pad = " ".repeat(self.width);
        format!("{pad} ")
    }
}
