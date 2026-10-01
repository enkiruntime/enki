use std::process::{Command, Stdio};

pub fn execute_run(cargo_args: &[String]) -> ! {
    let mut cmd = Command::new("cargo");
    cmd.arg("run");

    cmd.arg("--config").arg("profile.dev.opt-level=2");
    cmd.arg("--config").arg("profile.dev.codegen-units=1");
    cmd.arg("--config")
        .arg("profile.dev.package.\"*\".opt-level=2");

    cmd.arg("--config").arg("profile.release.opt-level=2");
    cmd.arg("--config").arg("profile.release.codegen-units=1");
    cmd.arg("--config")
        .arg("profile.release.package.\"*\".opt-level=2");

    for arg in cargo_args {
        cmd.arg(arg);
    }

    let current_rustflags = std::env::var("RUSTFLAGS").unwrap_or_default();
    let enki_rustflags = if current_rustflags.is_empty() {
        "--emit=llvm-bc".to_string()
    } else if current_rustflags.contains("--emit=llvm-bc") {
        current_rustflags
    } else {
        format!("{current_rustflags} --emit=llvm-bc")
    };

    cmd.env("RUSTFLAGS", enki_rustflags);

    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match cmd.status() {
        Ok(status) => {
            let code = status.code().unwrap_or(1);
            std::process::exit(code);
        }
        Err(err) => {
            eprintln!("\x1b[1;91merror\x1b[0m: failed to execute `cargo run`: {err}");
            std::process::exit(1);
        }
    }
}
