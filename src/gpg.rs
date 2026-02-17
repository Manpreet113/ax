use colored::*;
use std::process::{Command, Stdio};

pub fn ensure_keys(keys: &[String]) -> anyhow::Result<Vec<String>> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    println!(":: Checking {} PGP keys...", keys.len());

    let mut failed_keys = Vec::new();

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
                eprintln!(
                    "{} Failed to import key {}. Build may fail if signature verification is required.",
                    "   WARNING:".yellow(),
                    key
                );
                failed_keys.push(key.clone());
            }
        }
    }

    Ok(failed_keys)
}
