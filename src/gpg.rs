use colored::*;
use std::process::{Command, Stdio};

/// Ensure required PGP keys are available in the local keyring
/// Returns a list of keys that failed to be retrieved
pub fn ensure_keys(keys: &[String]) -> anyhow::Result<Vec<String>> {
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    println!("{} Checking {} PGP key(s)...", "::".blue().bold(), keys.len());

    let mut failed_keys = Vec::new();

    for key in keys {
        // Check if key already exists
        let check_status = Command::new("gpg")
            .arg("--list-keys")
            .arg(key)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match check_status {
            Ok(status) if status.success() => {
                println!("   {} Key {} already present", "✓".green(), key);
                continue;
            }
            _ => {
                println!("   {} Fetching key {}...", "→".yellow(), key);
            }
        }

        // Try to fetch the key from keyserver
        let fetch_status = Command::new("gpg")
            .arg("--keyserver")
            .arg("keyserver.ubuntu.com")
            .arg("--recv-keys")
            .arg(key)
            .status();

        match fetch_status {
            Ok(status) if status.success() => {
                println!("   {} Key {} imported successfully", "✓".green(), key);
            }
            Ok(status) => {
                eprintln!(
                    "   {} Failed to fetch key {} (exit code: {})",
                    "✗".red(),
                    key,
                    status.code().unwrap_or(-1)
                );
                failed_keys.push(key.clone());
            }
            Err(e) => {
                eprintln!("   {} Failed to run gpg: {}", "✗".red(), e);
                failed_keys.push(key.clone());
            }
        }
    }

    Ok(failed_keys)
}
