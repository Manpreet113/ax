use crate::api;
use crate::arch;
use anyhow::{Context, Result};
use colored::*;
use std::env;

pub async fn check_updates(config: &crate::config::Config) -> Result<Vec<String>> {
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
        if let Some(local_ver) = local_map.get(&remote.name)
            && alpm::vercmp(local_ver.as_str(), remote.version.as_str()) == std::cmp::Ordering::Less
        {
            updates.push((remote.name.clone(), local_ver.clone(), remote.version));
            update_names.push(remote.name);
        }
    }

    // Check VCS packages
    // Include -git, -hg, -svn, -bzr, -darcs, -cvs, -nightly, -dev (common suffixes)
    let vcs_suffixes = [
        "-git", "-hg", "-svn", "-bzr", "-darcs", "-cvs", "-nightly", "-dev",
    ];

    // Respect XDG_CACHE_HOME if possible, but for now we follow the existing pattern
    // or use the config (if we had access to it).
    // The previous code hardcoded .cache/ax. We should try to use the same logic as builder/resolver.
    // For now, let's keep the hardcoded path but make it slightly more robust effectively.
    // TODO: Centralize cache path logic.

    // Resolve Cache Dir: Config > XDG > HOME
        let cache_base = if let Some(ref dir) = config.build_dir {
            std::path::PathBuf::from(dir)
        } else if let Some(proj_dirs) = directories::ProjectDirs::from("com", "manpreet113", "ax") {
            proj_dirs.cache_dir().to_path_buf()
        } else {
            env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(format!("{}/.cache/ax", h)))
                .unwrap_or_else(|| std::path::PathBuf::from(".cache/ax"))
        };

    for pkg in &foreign_pkgs {
        if vcs_suffixes.iter().any(|s| pkg.name.ends_with(s)) {
            // Check if we already added it (e.g. AUR version bump)
            if update_names.contains(&pkg.name) {
                continue;
            }

            let cache_path = cache_base.join(&pkg.name);
            if cache_path.exists() {
                if let Ok(true) = crate::git_ops::check_vcs_update(&cache_path) {
                    updates.push((
                        pkg.name.clone(),
                        pkg.version.clone(),
                        "latest-commit".to_string(),
                    ));
                    update_names.push(pkg.name.clone());
                }
            } else {
                // Determine if we should warn about missing cache for VCS
                // For now, let's just print a debug/warning line or maybe add a special status?
                // "Cache missing" is hard to represent in the current Tuple return.
                // We'll skip it for now to avoid spamming the user, but maybe we could add logic later.
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
