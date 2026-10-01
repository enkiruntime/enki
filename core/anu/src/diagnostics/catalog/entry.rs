use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Severity {
    #[default]
    Error,
    Warning,
    Note,
    Help,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Note => write!(f, "note"),
            Self::Help => write!(f, "help"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticTemplate {
    pub code: u32,
    pub severity: Severity,
    pub title: &'static str,
    pub primary_label: &'static str,
    pub secondary_label: Option<&'static str>,
    pub note: Option<&'static str>,
    pub help: Option<&'static str>,
}

impl DiagnosticTemplate {
    #[inline(always)]
    pub fn code_str(&self) -> String {
        match self.severity {
            Severity::Warning => format!("W{:04}", self.code),
            _ => format!("E{:04}", self.code),
        }
    }

    pub const fn error(
        code: u32,
        title: &'static str,
        primary_label: &'static str,
        secondary_label: Option<&'static str>,
        note: Option<&'static str>,
        help: Option<&'static str>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Error,
            title,
            primary_label,
            secondary_label,
            note,
            help,
        }
    }

    pub const fn warning(
        code: u32,
        title: &'static str,
        primary_label: &'static str,
        secondary_label: Option<&'static str>,
        note: Option<&'static str>,
        help: Option<&'static str>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            title,
            primary_label,
            secondary_label,
            note,
            help,
        }
    }
}
