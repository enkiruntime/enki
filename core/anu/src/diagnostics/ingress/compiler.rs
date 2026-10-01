use super::event::Diagnostic;
use crate::diagnostics::source::Span;
use parsu::compiler::GpuDiagnostic;

pub fn from_parsu(diag: GpuDiagnostic) -> Diagnostic {
    let spans = diag
        .spans
        .into_iter()
        .map(|s| Span::from_coords(s.file_path, s.line, s.column, s.length, s.role))
        .collect();

    let payload = if diag.code == 9999 {
        diag.message
            .map(|token| super::event::DynamicPayload::InternalCompilerError { token })
    } else {
        None
    };

    Diagnostic {
        code: diag.code,
        spans,
        payload,
    }
}

pub fn from_parsu_batch(diags: Vec<GpuDiagnostic>) -> Vec<Diagnostic> {
    diags.into_iter().map(from_parsu).collect()
}

pub fn bitcode_not_found() -> Diagnostic {
    Diagnostic {
        code: 11,
        spans: Vec::new(),
        payload: None,
    }
}

pub fn toolchain_not_found() -> Diagnostic {
    Diagnostic {
        code: 12,
        spans: Vec::new(),
        payload: None,
    }
}
