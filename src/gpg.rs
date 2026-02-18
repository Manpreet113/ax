use colored::*;
use std::process::{Command, Stdio};

/// Ensure required PGP keys are available in the local keyring.
/// Returns true if all keys are available, false if some failed
/// (caller should use --skippgpcheck as fallback).
pub fn ensure_keys(keys: &[String]) -> anyhow::Result<bool> {
    if keys.is_empty() {
        return Ok(true);
    }

    println!(
        "{} Checking {} PGP key(s)...",
        "::".blue().bold(),
        keys.len()
    );

    let mut all_ok = true;

    for key in keys {
        // Check if key already exists in local keyring
        let already_present = Command::new("gpg")
            .args(["--batch", "--list-keys", key])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());

        if already_present {
            println!("   {} Key {} already present", "✓".green(), key);
            continue;
        }

        println!("   {} Fetching key {}...", "→".yellow(), key);

        // First attempt
        if fetch_key(key) {
            println!("   {} Key {} imported successfully", "✓".green(), key);
            continue;
        }

        // First attempt failed — kill ALL gpg daemons aggressively
        eprintln!(
            "   {} First attempt failed, restarting GPG daemons...",
            "→".yellow()
        );
        let _ = Command::new("gpgconf").args(["--kill", "all"]).status();

        // Small delay to let daemons fully shut down
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Retry
        if fetch_key(key) {
            println!("   {} Key {} imported on retry", "✓".green(), key);
        } else {
            eprintln!("   {} Failed to fetch key {}", "✗".red(), key);
            all_ok = false;
        }
    }

    Ok(all_ok)
}

/// Try to fetch a key from a keyserver. Returns true on success.
fn fetch_key(key: &str) -> bool {
    Command::new("gpg")
        .args([
            "--batch",
            "--yes",
            "--keyserver",
            "keyserver.ubuntu.com",
            "--recv-keys",
            key,
        ])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
