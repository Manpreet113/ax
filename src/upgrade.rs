use anyhow::Result;
use std::process::Command;
use colored::*;
use crate::api;

pub async fn check_updates() -> Result<()> {
    println!("{}", ":: Searching for AUR updates...".blue().bold());

    let output = Command::new("pacman").arg("-Qm").output()?;
    let stdout = String::from_utf8(output.stdout)?;

    let mut local_map = std::collections::HashMap::new();
    let mut pkg_names = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            local_map.insert(parts[0].to_string(), parts[1].to_string());
            pkg_names.push(parts[0].to_string());
        }
    }

    if pkg_names.is_empty() {
        println!(":: No foreign packages installed.");
        return Ok(());
    }

    let remote_pkgs = api::get_info(&pkg_names).await?;
    let mut updates = Vec::new();

    for remote in remote_pkgs {
        if let Some(local_ver) = local_map.get(&remote.name) {
            // TODO: Use 'alpm-rs' or 'vercmp' crate for rigorous comparison
            if local_ver != &remote.version {
                updates.push((remote.name, local_ver.clone(), remote.version));
            }
        }
    }

    if updates.is_empty() {
        println!("{}", ":: System is up to date.".green());
    } else {
        println!("\n{}", ":: Updates Available:".yellow().bold());
        for (name, old, new) in updates {
            println!("   {} : {} -> {}", name.cyan(), old.red(), new.green());
        }
    }

    Ok(())
}