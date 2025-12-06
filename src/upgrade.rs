use anyhow::{Context, Result};
use colored::*;
use crate::api;
use crate::arch;

pub async fn check_updates() -> Result<Vec<String>> {
    println!("{}", ":: Searching for AUR updates...".blue().bold());

    let arch_db = arch::ArchDB::new().context("Failed to initialize ALPM")?;
    let foreign_pkgs = arch_db.get_foreign_packages()?;

    if foreign_pkgs.is_empty() {
        println!(":: No foreign packages installed.");
        return Ok(vec![]);
    }

    let pkg_names: Vec<String> = foreign_pkgs.iter().map(|p| p.name.clone()).collect();
    let remote_pkgs = api::get_info(&pkg_names).await?;
    
    let mut updates = Vec::new();
    let mut update_names = Vec::new();

    // Create a map for faster lookup
    let local_map: std::collections::HashMap<String, String> = foreign_pkgs
        .iter()
        .map(|p| (p.name.clone(), p.version.clone()))
        .collect();

    for remote in remote_pkgs {
        if let Some(local_ver) = local_map.get(&remote.name) {
            // TODO: Use 'alpm-rs' or 'vercmp' crate for rigorous comparison.
            // String comparison is bad but I'm too scared to touch C bindings again.
            if local_ver != &remote.version {
                updates.push((remote.name.clone(), local_ver.clone(), remote.version));
                update_names.push(remote.name);
            }
        }
    }

    // Check VCS packages (ending in -git, -bin is NOT VCS, -hg, etc.)
    // Note: -bin packages are usually static versions, not VCS.
    let vcs_suffixes = ["-git", "-hg", "-svn", "-bzr"];
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

    for pkg in &foreign_pkgs {
        if vcs_suffixes.iter().any(|s| pkg.name.ends_with(s)) {
            // Check if we already added it (e.g. AUR version bump)
            if update_names.contains(&pkg.name) {
                continue;
            }

            let cache_path = std::path::Path::new(&home).join(".cache/raur").join(&pkg.name);
            if cache_path.exists() {
                if let Ok(true) = crate::git_ops::check_vcs_update(&cache_path) {
                    updates.push((pkg.name.clone(), pkg.version.clone(), "latest-commit".to_string()));
                    update_names.push(pkg.name.clone());
                }
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

    Ok(update_names)
}