use crate::arch::ArchDB;
use anyhow::Result;
use colored::*;
use std::env;
use std::future::Future;
use std::pin::Pin;

pub fn resolve_tree<'a>(
    pkg: &'a str,
    arch_db: &'a ArchDB,
    visited: &'a mut Vec<String>,
    cycle_path: &'a mut Vec<String>,
    build_queue: &'a mut Vec<String>,
    repo_queue: &'a mut Vec<String>,
    config: &'a crate::config::Config,
) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
    Box::pin(async move {
        // Cycle detection
        if cycle_path.contains(&pkg.to_string()) {
            anyhow::bail!("Circular dependency detected: {:?} -> {}", cycle_path, pkg);
        }

        if visited.contains(&pkg.to_string()) {
            return Ok(());
        }

        cycle_path.push(pkg.to_string());

        // 0. Repo Check (Fix for Phase 7)
        // If the package is in the official repos, we prioritize that.
        // We use the full package string (e.g. "lib32-glibc")
        if arch_db.exists_in_repo(pkg) {
            println!(":: {} is in official repo.", pkg.cyan());
            if !repo_queue.contains(&pkg.to_string()) {
                repo_queue.push(pkg.to_string());
            }
            visited.push(pkg.to_string()); // Mark as visited
            cycle_path.pop();
            return Ok(());
        }

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

        let path = cache_base.join(pkg);
        let aur_url = format!("https://aur.archlinux.org/{}.git", pkg);

        // 1. Git Ops (Safer)
        if path.exists() {
            println!(":: Checking for updates in {}...", pkg.cyan());
            if let Err(e) = crate::git_ops::pull_repo(&path) {
                eprintln!("!! Update failed for {}: {}", pkg, e);
                eprintln!("!! You may need to manually fix the git repo at {:?}", path);
                // We do NOT nuke the directory anymore.
                // Depending on severity, we might want to bail, but continuing might use old PKGBUILD.
                // Let's warn and continue, assuming user might have local changes or network is down.
            }
        } else if let Err(e) = crate::git_ops::clone_repo(&aur_url, &path) {
            anyhow::bail!("Failed to clone {}: {}", pkg, e);
        }

        // 2. Parse
        let meta = match crate::parser::parse_srcinfo(&path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("!! Error parsing .SRCINFO for {}: {}", pkg, e);

                // Phase 11: Validation Hardening
                // If .SRCINFO failed, we MUST verify if PKGBUILD exists.
                // If it doesn't, this is an invalid package (or empty repo).
                let pkgbuild_path = path.join("PKGBUILD");
                if !pkgbuild_path.exists() {
                    eprintln!(
                        "!! PKGBUILD not found for {}. removing invalid cache...",
                        pkg
                    );
                    if let Err(rm_err) = std::fs::remove_dir_all(&path) {
                        eprintln!("!! Failed to remove invalid directory: {}", rm_err);
                    }
                    anyhow::bail!("Package {} not found or invalid (no PKGBUILD/SRCINFO)", pkg);
                }

                // Fallback: assume pkg is what we want (only if PKGBUILD exists)
                build_queue.push(pkg.to_string());
                cycle_path.pop();
                visited.push(pkg.to_string());
                return Ok(());
            }
        };

        // If 'pkg' is a sub-package (e.g., 'gcc-libs' requested, but 'pkgbase' is 'gcc')
        // We still need to build 'pkgbase'.
        // The build_queue should ideally store the pkgbase.
        // But main.rs installs based on what's in build_queue.
        // Let's modify build_queue to store pkgbase, but we need to track what we actually want to install.
        //
        // NOTE: For now, we push `pkgbase` to build_queue.
        // `main.rs` will need to look at `repo_queue` and `packages` (user request) to decide which *files* to install.

        let pkgbase = if !meta.pkgbase.is_empty() {
            &meta.pkgbase
        } else {
            // Fallback if pkgbase missing (unlikely in valid SRCINFO)
            pkg
        };

        // Push pkgbase if not already there (deduplication happens later, or check here)
        if !build_queue.contains(&pkgbase.to_string()) {
            build_queue.push(pkgbase.to_string());
        }

        // We assume 'pkg' is satisfied by building 'pkgbase'.
        // We add 'pkg' to visited so we don't recurse on it again.
        if pkg != pkgbase {
            visited.push(pkgbase.to_string());
        }

        // Architecture Check
        let current_arch = std::env::consts::ARCH;
        if !meta.arch.is_empty() && !meta.arch.iter().any(|a| a == "any" || a == current_arch) {
            anyhow::bail!(
                "Architecture mismatch: Package {} supports {:?}, but system is {}",
                pkg,
                meta.arch,
                current_arch
            );
        }

        // 3. GPG
        crate::gpg::ensure_keys(&meta.validpgpkeys)?;

        // 4. Analyze Dependencies
        let mut aur_candidates = Vec::new();

        for dep in meta.depends {
            // Check if installed (handles providers)
            if arch_db.is_installed(&dep) {
                continue;
            }

            // Check if satisfied by repo (handles providers and versions now)
            // We pass the FULL dependency string (e.g. "foo>=1.0") to alpm
            if arch_db.exists_in_repo(&dep) {
                let clean_name = crate::parser::clean_dependency(&dep);
                if !repo_queue.contains(&clean_name) {
                    repo_queue.push(clean_name);
                }
            } else {
                // For AUR, we need the clean name (RPC doesn't handle >=)
                let clean_name = crate::parser::clean_dependency(&dep);
                aur_candidates.push(clean_name);
            }
        }

        // 5. Gatekeeper (API Verification) and Recursion
        if !aur_candidates.is_empty() {
            println!(
                ":: Verifying {} candidates with AUR...",
                aur_candidates.len()
            );
            let found_pkgs = crate::api::get_info(&aur_candidates).await?;
            let found_names: Vec<String> = found_pkgs.iter().map(|p| p.name.clone()).collect();

            for candidate in &aur_candidates {
                if found_names.contains(candidate) {
                    // Recurse
                    resolve_tree(
                        candidate,
                        arch_db,
                        visited,
                        cycle_path,
                        build_queue,
                        repo_queue,
                        config,
                    )
                    .await?;
                } else {
                    // Phase 9 Fix: Abort if dependency is missing
                    anyhow::bail!("Dependency '{}' not found in Repo or AUR.", candidate);
                }
            }
        }

        // build_queue.push(pkg.to_string()); // Moved to earlier step (pkgbase)
        visited.push(pkg.to_string());
        cycle_path.pop();
        Ok(())
    })
}
