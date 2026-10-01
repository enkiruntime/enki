use std::env;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

const BRIGHT_RED: &str = "\x1b[1;91m";
const BRIGHT_BLUE: &str = "\x1b[1;94m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub enabled: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Theme {
    pub fn from_env() -> Self {
        let no_color = env::var_os("NO_COLOR").is_some();
        Self { enabled: !no_color }
    }

    pub const fn plain() -> Self {
        Self { enabled: false }
    }

    pub fn header(
        &self,
        severity: crate::diagnostics::catalog::Severity,
        code_str: &str,
        title: &str,
    ) -> String {
        if self.enabled {
            match severity {
                crate::diagnostics::catalog::Severity::Warning => {
                    format!("\x1b[1;33mwarning\x1b[0m\x1b[1m: {title}\x1b[0m")
                }
                _ => {
                    format!("\x1b[1;91merror[{code_str}]\x1b[0m\x1b[1m: {title}\x1b[0m")
                }
            }
        } else {
            match severity {
                crate::diagnostics::catalog::Severity::Warning => format!("warning: {title}"),
                _ => format!("error[{code_str}]: {title}"),
            }
        }
    }

    pub fn carets_for_severity(
        &self,
        severity: crate::diagnostics::catalog::Severity,
        count: usize,
    ) -> String {
        let carets = "^".repeat(count.max(1));
        if self.enabled {
            match severity {
                crate::diagnostics::catalog::Severity::Warning => {
                    format!("\x1b[1;33m{carets}\x1b[0m")
                }
                _ => format!("{BRIGHT_RED}{carets}{RESET}"),
            }
        } else {
            carets
        }
    }

    pub fn label_for_severity(
        &self,
        severity: crate::diagnostics::catalog::Severity,
        text: &str,
    ) -> String {
        if self.enabled {
            match severity {
                crate::diagnostics::catalog::Severity::Warning => {
                    format!("\x1b[1;33m{text}\x1b[0m")
                }
                _ => format!("{BRIGHT_RED}{text}{RESET}"),
            }
        } else {
            text.to_string()
        }
    }

    pub fn arrow(&self) -> &str {
        if self.enabled {
            const ARROW: &str = "\x1b[1;94m-->\x1b[0m";
            ARROW
        } else {
            "-->"
        }
    }

    pub fn bar(&self) -> &str {
        if self.enabled {
            const BAR: &str = "\x1b[1;94m|\x1b[0m";
            BAR
        } else {
            "|"
        }
    }

    pub fn dots(&self) -> &str {
        if self.enabled {
            const DOTS: &str = "\x1b[1;94m...\x1b[0m";
            DOTS
        } else {
            "..."
        }
    }

    pub fn line_number(&self, num: usize) -> String {
        if self.enabled {
            format!("{BRIGHT_BLUE}{num}{RESET}")
        } else {
            num.to_string()
        }
    }

    pub fn primary_carets(&self, count: usize) -> String {
        let carets = "^".repeat(count.max(1));
        if self.enabled {
            format!("{BRIGHT_RED}{carets}{RESET}")
        } else {
            carets
        }
    }

    pub fn primary_label(&self, text: &str) -> String {
        if self.enabled {
            format!("{BRIGHT_RED}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    pub fn secondary_carets(&self, count: usize) -> String {
        let dashes = "-".repeat(count.max(1));
        if self.enabled {
            format!("{BRIGHT_BLUE}{dashes}{RESET}")
        } else {
            dashes
        }
    }

    pub fn secondary_label(&self, text: &str) -> String {
        if self.enabled {
            format!("{BRIGHT_BLUE}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    pub fn note_prefix(&self) -> &str {
        if self.enabled {
            const NOTE: &str = "\x1b[1;94m=\x1b[0m \x1b[1;96mnote:\x1b[0m";
            NOTE
        } else {
            {}
            "= note:"
        }
    }

    pub fn help_prefix(&self) -> &str {
        if self.enabled {
            const HELP: &str = "\x1b[1;94m=\x1b[0m \x1b[1;96mhelp:\x1b[0m";
            HELP
        } else {
            "= help:"
        }
    }

    pub fn bold(&self, text: &str) -> String {
        if self.enabled {
            format!("{BOLD}{text}{RESET}")
        } else {
            text.to_string()
        }
    }
}
