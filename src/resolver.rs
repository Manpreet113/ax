use crate::arch::ArchDB;
use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::Path;
use std::pin::Pin;
use std::future::Future;

pub fn resolve_tree<'a>(
    pkg: &'a str,
    arch_db: &'a ArchDB,
    visited: &'a mut Vec<String>,
    build_queue: &'a mut Vec<String>,
    repo_queue: &'a mut Vec<String>
) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
    Box::pin(async move {
        if visited.contains(&pkg.to_string()) {
            return Ok(());
        }
        visited.push(pkg.to_string());

        let home = env::var("HOME").context("No HOME var")?;
        let cache_dir = format!("{}/.cache/raur/{}", home, pkg);
        let aur_url = format!("https://aur.archlinux.org/{}.git", pkg);
        let path = Path::new(&cache_dir);

        // 1. Git Ops (with Self-Healing)
        if path.exists() {
            println!(":: Checking for updates in {}...", pkg.cyan());
            if let Err(e) = crate::git_ops::pull_repo(path) {
                eprintln!("!! Update failed ({}). Nuking directory...", e);
                if let Err(fs_err) = std::fs::remove_dir_all(path) {
                    anyhow::bail!("FATAL: Could not delete corrupted dir: {}", fs_err);
                }
            }
        }

        if !path.exists() {
            if let Err(e) = crate::git_ops::clone_repo(&aur_url, path) {
                anyhow::bail!("Failed to clone {}: {}", pkg, e);
            }
        }

        // 2. Parse
        let meta = match crate::parser::parse_srcinfo(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("!! Error parsing .SRCINFO for {}: {}", pkg, e);
                // We add it to queue anyway so makepkg can fail visibly
                build_queue.push(pkg.to_string());
                return Ok(());
            }
        };

        // 3. GPG
        crate::gpg::ensure_keys(&meta.validpgpkeys);

        // 4. Analyze Dependencies
        let mut aur_candidates = Vec::new();

        for dep in meta.depends {
            if arch_db.is_installed(&dep) {
                continue;
            }

            let clean_name = crate::parser::clean_dependency(&dep);

            if arch_db.exists_in_repo(&clean_name) {
                if !repo_queue.contains(&clean_name) {
                    repo_queue.push(clean_name);
                }
            } else {
                aur_candidates.push(clean_name);
            }
        }

        // 5. Gatekeeper (API Verification)
        if !aur_candidates.is_empty() {
            println!(":: Verifying {} candidates with AUR...", aur_candidates.len());
            let found_pkgs = crate::api::get_info(&aur_candidates).await?;
            let found_names: Vec<String> = found_pkgs.iter().map(|p| p.name.clone()).collect();

            for candidate in &aur_candidates {
                if found_names.contains(candidate) {
                    // Recurse
                    resolve_tree(candidate, arch_db, visited, build_queue, repo_queue).await?;
                } else {
                    eprintln!("{} Dependency '{}' not found in Repo or AUR.", "!! FATAL:".red().bold(), candidate);
                }
            }
        }

        build_queue.push(pkg.to_string());
        Ok(())
    })
}