use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::path::Path;
use std::process::Command;
use crate::interactive;
use crate::git_ops;

pub fn build_package(pkg: &str, clean_build: bool, show_diff: bool) -> Result<()> {
    let home = env::var("HOME").context("Could not find HOME directory")?;
    let cache_dir = format!("{}/.cache/raur/{}", home, pkg);
    let cache_path = Path::new(&cache_dir);

    println!(":: Building {}...", pkg.cyan());

    // 1. Prompt for diff if requested (and if it's an update, which implies directory exists)
    // I hope the directory exists, otherwise this will look silly.
    if show_diff && cache_path.exists() {
        if interactive::prompt_diff(pkg)? {
            match git_ops::get_diff(cache_path) {
                Ok(diff) => {
                    if diff.is_empty() {
                        println!(":: No git changes found.");
                    } else {
                        // Use pager for diff
                        let mut pager = Command::new("less")
                            .arg("-R") // Raw control chars for color
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                            .context("Failed to spawn pager")?;

                        if let Some(mut stdin) = pager.stdin.take() {
                            use std::io::Write;
                            write!(stdin, "{}", diff)?;
                        }
                        pager.wait()?;
                    }
                }
                Err(e) => println!(":: Failed to get diff: {}", e),
            }
        }
    }

    // 2. Prompt for review
    if interactive::prompt_review(pkg)? {
        let editor = env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        let pkgbuild_path = cache_path.join("PKGBUILD");
        
        let status = Command::new(&editor)
            .arg(&pkgbuild_path)
            .status()
            .context(format!("Failed to open {}", editor))?;

        if !status.success() {
             println!("{}", "!! Editor exited with error, aborting build.".red());
             anyhow::bail!("Editor failed");
        }
    }

    // 3. Clean build if requested
    if clean_build {
        println!("{}", ":: Cleaning build directory...".yellow());
        Command::new("git")
            .current_dir(&cache_dir)
            .args(&["clean", "-fdx"])
            .status()
            .context("Failed to clean build directory")?;
    }

    // 4. Run makepkg
    let status = Command::new("makepkg")
        .arg("-si")
        .current_dir(&cache_dir)
        .status()
        .context("Failed to execute makepkg")?;

    if status.success() {
        println!(":: {} {}", pkg.green(), "installed successfully!".green());
        Ok(())
    } else {
        anyhow::bail!("Failed to build {}. Aborting queue.", pkg);
    }
}
