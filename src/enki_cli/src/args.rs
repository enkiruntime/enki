#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Run { cargo_args: Vec<String> },
    Debug { token: String },
    Help,
    Version,
    Unknown(String),
}

pub fn parse() -> Action {
    let mut raw_args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(first) = raw_args.first() {
        if first == "enki" {
            raw_args.remove(0);
        }
    }

    if raw_args.is_empty() {
        return Action::Help;
    }

    let command = &raw_args[0];
    match command.as_str() {
        "run" => {
            let cargo_args = raw_args[1..].to_vec();
            Action::Run { cargo_args }
        }
        "debug" | "decode" => {
            let token = if raw_args.len() > 1 {
                raw_args[1..].join(" ")
            } else {
                println!("Paste the Enki Crash Token below (then press Enter):");
                let mut buffer = String::new();
                let _ = std::io::stdin().read_line(&mut buffer);
                buffer.trim().to_string()
            };
            Action::Debug { token }
        }
        "-h" | "--help" | "help" => Action::Help,
        "-V" | "--version" | "version" => Action::Version,
        other => Action::Unknown(other.to_string()),
    }
}
