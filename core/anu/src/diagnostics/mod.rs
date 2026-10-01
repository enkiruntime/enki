pub mod catalog;
pub mod ingress;
pub mod render;
pub mod source;

pub use catalog::{DiagnosticTemplate, Severity};
pub use ingress::{
    ContractDiagnosticBuilder, Diagnostic, DynamicPayload, from_parsu, from_parsu_batch, hw, rt,
};
pub use render::{Theme, render, render_batch};
pub use source::{Role, SourceCache, Span};

pub fn emit_diagnostic(diag: &Diagnostic) -> String {
    let cache = SourceCache::new();
    render(diag, &cache)
}

pub fn emit_batch(diags: &[Diagnostic]) -> String {
    let cache = SourceCache::new();
    render_batch(diags, &cache)
}
