pub mod args;
pub mod decoder;
pub mod runner;

use args::Action;

pub fn run() {
    let action = args::parse();

    match action {
        Action::Run { cargo_args } => {
            runner::execute_run(&cargo_args);
        }
        Action::Debug { token } => {
            decoder::execute_debug(&token);
        }
        Action::Help => {
            print_help();
        }
        Action::Version => {
            println!("enki {}", env!("CARGO_PKG_VERSION"));
        }
        Action::Unknown(subcmd) => {
            eprintln!("\x1b[1;91merror\x1b[0m: no such subcommand: `{subcmd}`");
            eprintln!("\n       For more information, try `enki --help`\n");
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!(
        "\
enki {version}
The command-line interface for the Enki heterogeneous GPU compute platform

USAGE:
    enki [OPTIONS] [SUBCOMMAND]
    cargo enki [OPTIONS] [SUBCOMMAND]

SUBCOMMANDS:
    run        Compile and run the current package with GPU JIT support

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

EXAMPLES:
    enki run
    enki run --release
    enki run --bin my_app
    enki run --example demo -- --custom-arg
",
        version = env!("CARGO_PKG_VERSION")
    );
}
