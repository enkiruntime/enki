//! CLI Driver for Girra Engine.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use girra::generator::{FuzzPolicy, generate_program};

#[derive(Parser, Debug)]
#[command(name = "girra")]
#[command(
    about = "Crucible & Differential Fuzzing Engine for Enki & Parsu",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Synthesize and inspect a single randomized program
    Run {
        /// Random seed (optional; generated automatically if omitted)
        #[arg(short, long)]
        seed: Option<u64>,

        /// Fuzzing policy preset: unconstrained | gpu-legal | spmd-spatial
        #[arg(short, long, default_value = "gpu-legal")]
        policy: String,

        /// Print only the raw synthesized Rust source code
        #[arg(long, default_value_t = false)]
        print: bool,
    },
    /// Execute a batch synthesis test suite
    Suite {
        /// Number of programs to generate
        #[arg(short, long, default_value_t = 10)]
        count: usize,

        /// Fuzzing policy preset: unconstrained | gpu-legal | spmd-spatial
        #[arg(short, long, default_value = "gpu-legal")]
        policy: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            seed,
            policy,
            print,
        } => {
            let actual_seed = seed.unwrap_or_else(rand::random);
            let fuzz_policy = FuzzPolicy::from_name(&policy);

            if !print {
                println!(
                    "{}",
                    "=======================================================".bright_cyan()
                );
                println!(
                    "{}",
                    ">>> [GIRRA] The Crucible of Enki GPU Engine"
                        .bright_yellow()
                        .bold()
                );
                println!(
                    "{}",
                    "=======================================================".bright_cyan()
                );
                println!("{} {}", "Target Seed:".green().bold(), actual_seed);
                println!("{} {}", "Policy:     ".magenta().bold(), fuzz_policy.name);
                println!(
                    "{}",
                    "-------------------------------------------------------".dimmed()
                );
            }

            // Synthesize and sanitize the full program
            let program = generate_program(actual_seed, fuzz_policy);
            let rust_source = program.to_rust_source();

            if print {
                println!("{}", rust_source);
            } else {
                println!(
                    "{}",
                    "[Generation & UB Sanitization Successful]".green().bold()
                );
                println!("├── Structs Defined:          {}", program.structs.len());
                println!("├── Helper Functions:         {}", program.functions.len());
                println!(
                    "└── Has #[nam] GPU Kernel:    {}",
                    if program.nam_kernel.is_some() {
                        "Yes (GPU Ready)".bright_green()
                    } else {
                        "No".yellow()
                    }
                );

                println!(
                    "\n{}",
                    ">>> [Synthesized Rust Source Code]:"
                        .bright_magenta()
                        .bold()
                );
                println!(
                    "{}",
                    "-------------------------------------------------------".dimmed()
                );
                println!("{}", rust_source);
                println!(
                    "{}",
                    "=======================================================".bright_cyan()
                );
            }
        }
        Commands::Suite { count, policy } => {
            let fuzz_policy = FuzzPolicy::from_name(&policy);
            println!(
                "{}",
                "=======================================================".bright_cyan()
            );
            println!(
                "{} {} programs with policy: {}",
                "Running batch generation of:".green().bold(),
                count,
                fuzz_policy.name.magenta()
            );
            println!(
                "{}",
                "=======================================================".bright_cyan()
            );

            for i in 0..count {
                let seed: u64 = rand::random();
                let prog = generate_program(seed, fuzz_policy.clone());
                println!(
                    "  [{:03}/{:03}] Seed: {:<20} | Structs: {} | Fns: {} | Nam: {}",
                    i + 1,
                    count,
                    seed,
                    prog.structs.len(),
                    prog.functions.len(),
                    prog.nam_kernel.is_some()
                );
            }

            println!(
                "\n{}",
                "All programs synthesized and sanitized successfully without crashes!"
                    .bright_green()
                    .bold()
            );
        }
    }

    Ok(())
}
