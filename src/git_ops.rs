use anyhow::Result;
use colored::*;
use git2::{build::RepoBuilder, FetchOptions, RemoteCallbacks};
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
    // We use Command for pull because libgit2 merge logic is complex
    let status = std::process::Command::new("git")
        .current_dir(path)
        .arg("pull")
        .output()?;

    if status.status.success() {
        Ok(())
    } else {
        anyhow::bail!("Git pull failed")
    }
}