use std::io::Write;

fn executable_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "enki_app".to_string())
}

pub(crate) fn emit_and_abort(diag: &anu::diagnostics::Diagnostic) -> ! {
    let _ = std::io::stdout().flush();
    eprintln!();

    let formatted = anu::diagnostics::emit_diagnostic(diag);
    eprint!("{formatted}");

    let bin_name = executable_name();
    eprintln!(
        "\x1b[1;91merror\x1b[0m: aborting execution of `{bin_name}` due to 1 previous error\n"
    );

    let _ = std::io::stderr().flush();
    std::process::exit(101);
}

pub(crate) fn handle_execution_error(err: &anyhow::Error) -> ! {
    let err_str = err.root_cause().to_string();

    eprint!("{err_str}");

    if !err_str.ends_with('\n') {
        eprintln!();
    }

    if !err_str.contains("could not compile") {
        let bin_name = executable_name();
        eprintln!(
            "\x1b[1;91merror\x1b[0m: execution halted in `{bin_name}` due to 1 previous error\n"
        );
    }

    let _ = std::io::stderr().flush();
    std::process::exit(101);
}

pub(crate) fn emit_warning(diag: &anu::diagnostics::Diagnostic) {
    let formatted = anu::diagnostics::emit_diagnostic(diag);
    eprint!("{formatted}");
    let _ = std::io::stderr().flush();
}
