use std::process::Command;
use anyhow::{Result, Context};
use crate::core::Package;

pub fn search(query: &str) -> Result<Vec<Package>> {
    let output = Command::new("pacman")
        .arg("-Ss")
        .arg("--color")
        .arg("never")
        .arg(query)
        .output()
        .context("Failed to execute pacman command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Pacman error: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout)
        .context("Pacman stdout was not valid UTF-8")?;

    parse_search_output(&stdout)
}

fn parse_search_output(raw: &str) -> Result<Vec<Package>> {
    let mut packages = Vec::new();
    let lines: Vec<&str> = raw.lines().collect();

    // TODO: write a more robust state machine parser

    for chunk in lines.chunks(2){
        if chunk.len() < 2 {continue;}

        let header = chunk[0];
        let desc = chunk[1].trim();
        let parts: Vec<&str> = header.split_whitespace().collect();

        if parts.len() < 2 { continue; }

        let Some((repo, name)) = parts[0].split_once('/') else {
            continue;
        };
        let version = parts[1];
        let installed = parts.last().map(|s| *s == "[installed]").unwrap_or(false);

        packages.push(Package {
            repo: repo.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            description: desc.to_string(),
            installed,
        });
    }
    Ok(packages)
}