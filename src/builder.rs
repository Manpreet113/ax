use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use log::debug;

use crate::git_ops;
use crate::interactive;

pub fn build_package(
    pkg: &str,
    config: &crate::config::Config,
    show_diff: bool,
) -> Result<Vec<PathBuf>> {
    let cache_base = config.get_cache_dir();
    let cache_dir = cache_base.join(pkg);
    let cache_path = Path::new(&cache_dir);

    println!(":: Building {}...", pkg.cyan());

    // 1. Prompt for diff if requested (and if it's an update, which implies directory exists)
    // I hope the directory exists, otherwise this will look silly.
    if show_diff && !config.no_confirm && cache_path.exists() && interactive::prompt_diff(pkg)? {
        match git_ops::get_diff(cache_path) {
            Ok(diff) => {
                if diff.is_empty() {
                    println!(":: No git changes found.");
                } else {
                    // Use pager for diff
                    match Command::new("less")
                        .arg("-R") // Raw control chars for color
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                    {
                        Ok(mut pager) => {
                            if let Some(mut stdin) = pager.stdin.take() {
                                use std::io::Write;
                                let _ = write!(stdin, "{}", diff);
                            }
                            pager.wait()?;
                        }
                        Err(_) => {
                            println!(":: (Pager failed, showing raw diff)");
                            println!("{}", diff);
                        }
                    }
                }
            }
            Err(e) => println!(":: Failed to get diff: {}", e),
        }
    }

    // 2. Prompt for review (skip in --noconfirm mode)
    if !config.no_confirm && interactive::prompt_review(pkg)? {
        let editor = config
            .editor
            .as_deref()
            .map(|s| s.to_string())
            .or_else(|| env::var("EDITOR").ok())
            .unwrap_or_else(|| "nano".to_string());

        let pkgbuild_path = cache_path.join("PKGBUILD");
        // let pkgbuild_str = pkgbuild_path.to_string_lossy();

        // Use sh -c to allow arguments in EDITOR (e.g., "code --wait")
        // Pass the file path as an argument to sh to prevent shell injection
        let status = Command::new("sh")
            .arg("-c")
            .arg(format!("{} \"$1\"", editor))
            .arg("--")
            .arg(&pkgbuild_path)
            .status()
            .context(format!("Failed to open editor: {}", editor))?;

        if !status.success() {
            println!("{}", "!! Editor exited with error.".red());
            anyhow::bail!("Editor failed with status: {}", status);
        }

        // Post-edit confirmation (fixes issue where editors return immediately)
        if !config.no_confirm && !crate::interactive::prompt_continue()? {
            anyhow::bail!("Build aborted by user.");
        }
    }

    // 3. Get exact list of packages that will be built BEFORE building
    println!(":: Determining package list...");
    let packagelist_output = Command::new("makepkg")
        .arg("--packagelist")
        .current_dir(&cache_dir)
        .output()
        .context("Failed to run makepkg --packagelist")?;

    if !packagelist_output.status.success() {
        anyhow::bail!("makepkg --packagelist failed");
    }

    let package_files: Vec<PathBuf> = String::from_utf8_lossy(&packagelist_output.stdout)
        .lines()
        .map(|line| PathBuf::from(line.trim()))
        .collect();

    if package_files.is_empty() {
        anyhow::bail!("makepkg --packagelist returned no packages");
    }

    println!(":: Will build: {}", package_files.len());
    for pf in &package_files {
        if let Some(fname) = pf.file_name() {
            println!("   - {}", fname.to_string_lossy());
        }
    }

    // 4. Clean build if requested
    if config.clean_build {
        println!("{}", ":: Cleaning build directory...".yellow());
        Command::new("git")
            .current_dir(&cache_dir)
            .args(["clean", "-fdx"])
            .status()
            .context("Failed to clean build directory")?;
    }

    // 5. Fetch required PGP keys from .SRCINFO
    let mut skip_pgp = false;
    if let Ok(metadata) = crate::parser::parse_srcinfo(cache_path)
        && !metadata.validpgpkeys.is_empty()
    {
        let keys_ok = crate::gpg::ensure_keys(&metadata.validpgpkeys)?;
        if !keys_ok {
            eprintln!(
                "{} GPG key fetch failed — falling back to --skippgpcheck",
                "!!".yellow().bold()
            );
            skip_pgp = true;
        }
    }

    // 6. Run makepkg
    debug!("Starting makepkg for {}", pkg);
    let mut makepkg = Command::new("makepkg");
    makepkg.arg("-srf"); // Sync deps, Remove deps, Force build
    if skip_pgp {
        makepkg.arg("--skippgpcheck");
    }
    let status = makepkg
        .current_dir(&cache_dir)
        .status()
        .context("Failed to execute makepkg")?;

    if status.success() {
        println!(":: {} {}", pkg.green(), "built successfully!".green());

        // Return the exact package files that were built
        Ok(package_files)
    } else {
        anyhow::bail!("Failed to build {}. Aborting queue.", pkg);
    }
}
