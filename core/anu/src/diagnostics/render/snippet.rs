use super::gutter::Gutter;
use super::theme::Theme;
use crate::diagnostics::catalog::DiagnosticTemplate;
use crate::diagnostics::source::{Role, SourceCache, Span};
use std::path::Path;

fn format_display_path(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = path.strip_prefix(&cwd) {
            return rel.display().to_string();
        }
        if let Ok(canon_cwd) = cwd.canonicalize() {
            if let Ok(canon_path) = path.canonicalize() {
                if let Ok(rel) = canon_path.strip_prefix(&canon_cwd) {
                    return rel.display().to_string();
                }
            }
        }
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = Path::new(&manifest_dir);
        if let Ok(rel) = path.strip_prefix(manifest_path) {
            return rel.display().to_string();
        }
    }
    path.display().to_string()
}

pub fn render_snippets(
    spans: &[Span],
    template: &DiagnosticTemplate,
    cache: &SourceCache,
    gutter: &Gutter,
    theme: &Theme,
) -> String {
    if spans.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(512);

    let primary = spans.iter().find(|s| s.is_primary()).unwrap_or(&spans[0]);
    let file_str = format_display_path(&primary.file_path);
    let pos_str = format!("{}:{}:{}", file_str, primary.line, primary.column);

    out.push_str(&gutter.arrow_header(theme, &pos_str));
    out.push('\n');
    out.push_str(&gutter.empty_bar(theme));
    out.push('\n');

    let mut sorted_spans = spans.to_vec();
    sorted_spans.sort_by_key(|s| s.line);

    let mut last_line = 0;

    for span in &sorted_spans {
        if last_line != 0 && span.line > last_line + 1 {
            out.push_str(&gutter.dots_bar(theme));
            out.push('\n');
        }

        if let Some(line_info) = cache.get_formatted_line(span) {
            out.push_str(&gutter.line_bar(span.line, theme));
            out.push(' ');
            out.push_str(&line_info.text);
            out.push('\n');

            let indent = " ".repeat(line_info.visual_column.saturating_sub(1));
            out.push_str(&gutter.empty_bar(theme));
            out.push(' ');
            out.push_str(&indent);

            match span.role {
                Role::Primary => {
                    let carets =
                        theme.carets_for_severity(template.severity, line_info.visual_length);
                    let label = theme.label_for_severity(template.severity, template.primary_label);
                    out.push_str(&carets);
                    if !template.primary_label.is_empty() {
                        out.push(' ');
                        out.push_str(&label);
                    }
                }
                Role::Secondary => {
                    let dashes = theme.secondary_carets(line_info.visual_length);
                    out.push_str(&dashes);
                    if let Some(sec_label) = template.secondary_label {
                        if !sec_label.is_empty() {
                            out.push(' ');
                            out.push_str(&theme.secondary_label(sec_label));
                        }
                    }
                }
            }
            out.push('\n');
        }

        last_line = span.line;
    }

    out.push_str(&gutter.empty_bar(theme));
    out.push('\n');

    out
}
