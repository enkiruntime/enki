pub const TAB_WIDTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedSourceLine {
    pub text: String,
    pub visual_column: usize,
    pub visual_length: usize,
}

pub fn format_source_line(raw_line: &str, column: usize, length: usize) -> FormattedSourceLine {
    let col_start = column.max(1);

    let effective_len = if length <= 1 {
        let remaining_chars: String = raw_line.chars().skip(col_start - 1).collect();
        if let Some(semi_pos) = remaining_chars.find(';') {
            semi_pos.max(1)
        } else {
            remaining_chars.trim_end().len().max(1)
        }
    } else {
        length
    };

    let col_end = col_start + effective_len;

    let mut expanded_text = String::with_capacity(raw_line.len() + 16);
    let mut current_visual_col = 1;
    let mut target_visual_col = 1;
    let mut target_visual_len = 0;

    for (char_idx, ch) in raw_line.chars().enumerate() {
        let orig_col = char_idx + 1;
        let char_visual_start = current_visual_col;

        if ch == '\t' {
            let spaces = TAB_WIDTH - ((current_visual_col - 1) % TAB_WIDTH);
            for _ in 0..spaces {
                expanded_text.push(' ');
            }
            current_visual_col += spaces;
        } else if ch != '\r' && ch != '\n' {
            expanded_text.push(ch);
            current_visual_col += 1;
        }

        let char_visual_width = current_visual_col - char_visual_start;

        if orig_col == col_start {
            target_visual_col = char_visual_start;
        }

        if orig_col >= col_start && orig_col < col_end {
            target_visual_len += char_visual_width;
        }
    }

    FormattedSourceLine {
        text: expanded_text,
        visual_column: target_visual_col,
        visual_length: target_visual_len.max(1),
    }
}
