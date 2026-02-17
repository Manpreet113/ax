use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::git_ops;
use crate::interactive;

pub fn build_package(
    pkg: &str,
    config: &crate::config::Config,
    show_diff: bool,
) -> Result<Vec<PathBuf>> {
    // Resolve Cache Dir: Config > XDG > HOME
    let cache_base = if let Some(ref dir) = config.build_dir {
        std::path::PathBuf::from(dir)
    } else if let Some(proj_dirs) = directories::ProjectDirs::from("com", "manpreet113", "ax") {
        proj_dirs.cache_dir().to_path_buf()
    } else {
        // Safe fallback without unwrap
        env::var("HOME")
            .ok()
            .map(|h| std::path::PathBuf::from(format!("{}/.cache/ax", h)))
            .unwrap_or_else(|| std::path::PathBuf::from(".cache/ax"))
    };

    let cache_dir = cache_base.join(pkg);
    let cache_path = Path::new(&cache_dir);

    println!(":: Building {}...", pkg.cyan());

    // 1. Prompt for diff if requested (and if it's an update, which implies directory exists)
    // I hope the directory exists, otherwise this will look silly.
    if show_diff && cache_path.exists() && interactive::prompt_diff(pkg)? {
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
                                write!(stdin, "{}", diff)?;
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

    // 2. Prompt for review
    if interactive::prompt_review(pkg)? {
        let editor = config
            .editor
            .as_deref()
            .map(|s| s.to_string())
            .or_else(|| env::var("EDITOR").ok())
            .unwrap_or_else(|| "nano".to_string());

        let pkgbuild_path = cache_path.join("PKGBUILD");
        let pkgbuild_str = pkgbuild_path.to_string_lossy();

        // Use sh -c to allow arguments in EDITOR (e.g., "code --wait")
        let cmd_str = format!("{} \"{}\"", editor, pkgbuild_str);

        let status = Command::new("sh")
            .arg("-c")
            .arg(&cmd_str)
            .status()
            .context(format!("Failed to open editor: {}", editor))?;

        if !status.success() {
            println!("{}", "!! Editor exited with error.".red());
            anyhow::bail!("Editor failed with status: {}", status);
        }

        // Post-edit confirmation (fixes issue where editors return immediately)
        if !crate::interactive::prompt_continue()? {
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

    // 5. Run makepkg
    let status = Command::new("makepkg")
        .arg("-sf") // Sync deps, Force build (overwrite), DO NOT install (-i)
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
