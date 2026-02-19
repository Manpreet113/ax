use anyhow::{Context, Result};
use colored::*;
use git2::{FetchOptions, RemoteCallbacks, build::RepoBuilder};
use std::path::Path;

pub fn clone_repo(url: &str, path: &Path) -> Result<()> {
    println!(":: Downloading {}...", url.cyan());

    let callbacks = RemoteCallbacks::new();
    // Progress bar removed for cleaner logs

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    fetch_options.depth(1); // Shallow clone for speed

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);

    builder.clone(url, path)?;
    Ok(())
}

pub fn pull_repo(path: &Path) -> Result<()> {
    // 1. Open Repository
    let repo = git2::Repository::open(path)
        .context("Failed to open repository")?;

    // 2. Find Remote
    let mut remote = repo.find_remote("origin")
        .context("Failed to find remote 'origin'")?;

    // 3. Fetch
    let mut fetch_options = FetchOptions::new();
    remote.fetch(&[] as &[&str], Some(&mut fetch_options), None)
        .context("Failed to fetch from remote")?;

    // 4. Find FETCH_HEAD
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

    // 5. Merge Analysis
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_up_to_date() {
        Ok(())
    } else if analysis.is_fast_forward() {
        // Fast-forward
        let head = repo.head()?;
        let refname = head.name().ok_or_else(|| anyhow::anyhow!("HEAD reference name invalid"))?;

        let mut reference = repo.find_reference(refname)?;
        reference.set_target(fetch_commit.id(), "Fast-Forward")?;
        repo.set_head(refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    } else {
        // Diverged or conflict
        anyhow::bail!("Repository has diverged or requires merge. Manual intervention required.")
    }
}

pub fn get_diff(path: &Path) -> Result<String> {
    // 0. Fetch first to ensure FETCH_HEAD is valid
    let fetch_status = std::process::Command::new("git")
        .current_dir(path)
        .arg("fetch")
        .output()?;

    if !fetch_status.status.success() {
        return Ok("(Failed to fetch updates, cannot show diff)".to_string());
    }

    // Show diff between HEAD and the upstream we just fetched
    // Usually 'git diff HEAD..FETCH_HEAD' works after a fetch
    let status = std::process::Command::new("git")
        .current_dir(path)
        .args(["diff", "HEAD..FETCH_HEAD", "--color=always"])
        .output()?;

    Ok(String::from_utf8_lossy(&status.stdout).to_string())
}

pub fn check_vcs_update(path: &Path) -> Result<bool> {
    // 1. Fetch
    let status = std::process::Command::new("git")
        .current_dir(path)
        .arg("fetch")
        .output()?;

    if !status.status.success() {
        return Ok(false); // If fetch fails, assume no update or offline
    }

    // 2. Compare HEAD and @{upstream}
    let output = std::process::Command::new("git")
        .current_dir(path)
        .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output format: "0 5" (0 ahead, 5 behind)
    let parts: Vec<&str> = stdout.split_whitespace().collect();

    if parts.len() >= 2 {
        let behind: usize = parts[1]
            .parse()
            .context("Failed to parse git rev-list output")?;
        return Ok(behind > 0);
    }

    Ok(false)
}
