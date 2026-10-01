use base64::Engine;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

const MASTER_PIN_HASH: &str = "218e19b44b9de9e57aad8f3520a0605838eb58af2fa1ac49a4a080acf35ee421";
const SALT: &str = "ENKI_DEV_SECRET_SALT_2026";

const PUBLIC_KEY_SEED: [u8; 32] = [
    0x45, 0x6E, 0x6B, 0x69, 0x5F, 0x50, 0x61, 0x72, 0x73, 0x75, 0x5F, 0x53, 0x65, 0x63, 0x72, 0x65,
    0x74, 0x4B, 0x65, 0x79, 0x5F, 0x32, 0x30, 0x32, 0x36, 0x5F, 0x41, 0x6C, 0x70, 0x68, 0x61, 0x21,
];

pub fn execute_debug(raw_token: &str) {
    print!("\x1b[1;94m  --> \x1b[0m\x1b[1mEnter Developer Passcode:\x1b[0m ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        eprintln!("\n\x1b[1;91merror\x1b[0m: failed to read input.");
        std::process::exit(1);
    }

    let entered_pin = input.trim();

    let mut hasher = Sha256::new();
    hasher.update(entered_pin.as_bytes());
    hasher.update(SALT.as_bytes());
    let computed_hash = format!("{:x}", hasher.finalize());

    if computed_hash != MASTER_PIN_HASH {
        eprintln!("\n\x1b[1;91merror\x1b[0m: access denied: invalid developer passcode.");
        std::process::exit(1);
    }

    let b64_clean: String = raw_token
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with("-----") && !l.is_empty())
        .collect();

    if b64_clean.is_empty() {
        eprintln!("\n\x1b[1;91merror\x1b[0m: empty or invalid crash token payload.");
        std::process::exit(1);
    }

    let mut data = match base64::engine::general_purpose::STANDARD.decode(&b64_clean) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("\n\x1b[1;91merror\x1b[0m: Base64 decoding failed: {e}");
            std::process::exit(1);
        }
    };

    for i in 0..data.len() {
        let b = data[i];
        let unrotated = (b >> 3) | (b << 5);
        let k = PUBLIC_KEY_SEED[i % 32];
        let unxored = unrotated ^ (k.wrapping_add((i & 0xFF) as u8));
        data[i] = unxored;
    }

    let json_str = match String::from_utf8(data) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\n\x1b[1;91merror\x1b[0m: decrypted payload is not valid UTF-8: {e}");
            std::process::exit(1);
        }
    };

    let report: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("\n\x1b[1;91merror\x1b[0m: failed to parse decrypted JSON: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "\x1b[1;32m====================================================================\x1b[0m"
    );
    println!("\x1b[1;32m>>> ENKI PARSU CRASH FLIGHT RECORDER DECRYPTED REPORT <<<\x1b[0m");

    println!(
        "\x1b[1;33mTarget Nam:\x1b[0m        {}",
        report["nam"].as_str().unwrap_or("unknown")
    );
    println!(
        "\x1b[1;33mCompiler Phase:\x1b[0m    {}",
        report["phase"].as_str().unwrap_or("unknown")
    );
    println!(
        "\x1b[1;33mOS & Platform:\x1b[0m     {}",
        report["os"].as_str().unwrap_or("unknown")
    );
    println!(
        "\x1b[1;33mWorkgroup Size:\x1b[0m    {}",
        report["workgroup"]
    );
    println!(
        "\x1b[1;33mArgs Count:\x1b[0m        {}",
        report["args_count"]
    );
    println!(
        "\x1b[1;91mFailed Source:\x1b[0m     {}:{}",
        report["inv_file"].as_str().unwrap_or("?"),
        report["inv_line"]
    );
    println!(
        "\x1b[1;91mFunction:\x1b[0m          {}",
        report["inv_func"].as_str().unwrap_or("?")
    );
    println!(
        "\x1b[1;91mInvariant Message:\x1b[0m {}",
        report["message"].as_str().unwrap_or("?")
    );

    let type_a = report["type_a"].as_str().unwrap_or("none");
    if type_a != "none" {
        println!("\x1b[1;36mType A:\x1b[0m            {type_a}");
    }
    let type_b = report["type_b"].as_str().unwrap_or("none");
    if type_b != "none" {
        println!("\x1b[1;36mType B:\x1b[0m            {type_b}");
    }
    println!(
        "\x1b[1;32m====================================================================\x1b[0m\n"
    );
}
