pub mod compiler;
pub mod contract;
pub mod entry;
pub mod hardware;
pub mod runtime;

pub use entry::{DiagnosticTemplate, Severity};

#[inline]
pub fn lookup(code: u32) -> Option<DiagnosticTemplate> {
    match code {
        1..=999 | 9999 => compiler::lookup(code),

        1000..=1999 => contract::lookup(code),
        2000..=2999 => runtime::lookup(code),
        3000..=3999 => hardware::lookup(code),
        _ => None,
    }
}

pub fn lookup_or_unknown(code: u32) -> DiagnosticTemplate {
    if let Some(template) = lookup(code) {
        template
    } else {
        DiagnosticTemplate::error(
            code,
            "unregistered or unknown GPU diagnostic",
            "unclassified error occurred here",
            None,
            Some(
                "this diagnostic code has no registered human-readable template in the master catalog.",
            ),
            Some(
                "ensure your Enki components (compiler, runtime, and API) are synchronized and up to date.",
            ),
        )
    }
}
