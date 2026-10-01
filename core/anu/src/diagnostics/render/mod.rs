pub mod footnotes;
pub mod gutter;
pub mod snippet;
pub mod theme;

pub use footnotes::render_footnotes;
pub use gutter::Gutter;
pub use snippet::render_snippets;
pub use theme::Theme;

use crate::diagnostics::catalog;
use crate::diagnostics::ingress::Diagnostic;
use crate::diagnostics::source::SourceCache;

pub fn render(diag: &Diagnostic, cache: &SourceCache) -> String {
    let mut out = render_with_theme(diag, cache, &Theme::default());
    let template = catalog::lookup_or_unknown(diag.code);
    let code_str = template.code_str();

    if template.severity == crate::diagnostics::catalog::Severity::Error {
        out.push_str(&format!(
            "\x1b[1mFor more information about this nam diagnostic, try `enki --explain {code_str}`.\x1b[0m\n"
        ));
    }

    out
}

pub fn render_batch(diags: &[Diagnostic], cache: &SourceCache) -> String {
    let theme = Theme::default();
    let mut out = String::new();

    for diag in diags {
        out.push_str(&render_with_theme(diag, cache, &theme));
    }

    if let Some(first) = diags.first() {
        let template = catalog::lookup_or_unknown(first.code);
        let code_str = template.code_str();
        out.push_str(&format!(
            "\x1b[1mFor more information about this nam diagnostic, try `enki --explain {code_str}`.\x1b[0m\n"
        ));
    }

    out
}

pub fn render_with_theme(diag: &Diagnostic, cache: &SourceCache, theme: &Theme) -> String {
    let template = catalog::lookup_or_unknown(diag.code);

    let max_line = diag.spans.iter().map(|s| s.line).max().unwrap_or(1);
    let gutter = Gutter::from_max_line(max_line);

    let mut out = String::with_capacity(512);

    out.push('\n');
    out.push_str(&theme.header(template.severity, &template.code_str(), template.title));
    out.push('\n');

    if !diag.spans.is_empty() {
        out.push_str(&render_snippets(
            &diag.spans,
            &template,
            cache,
            &gutter,
            theme,
        ));
    }

    out.push_str(&render_footnotes(
        &template,
        diag.payload.as_ref(),
        &gutter,
        theme,
    ));
    out.push('\n');

    out
}
