use std::process::{Command, Stdio};
use colored::*;

pub fn ensure_keys(keys: &[String]) {
    if keys.is_empty() {
        return;
    }

    println!(":: Checking {} PGP keys...", keys.len());

    for key in keys {
        let status = Command::new("gpg")
            .arg("--list-keys")
            .arg(key)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("Failed to check gpg");

        if !status.success() {
            println!("   -> Key {} missing. Fetching...", key.yellow());

            let fetch_status = Command::new("gpg")
                .arg("--recv-keys")
                .arg(key)
                .status()
                .expect("Failed to run gpg recv-keys");

            if !fetch_status.success() {
                eprintln!("!! Warning: Failed to import key {}. Build might fail.", key.red());
            }
        }
    }
}