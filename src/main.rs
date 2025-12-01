mod args;
mod aur;
mod core;
mod pacman;
mod git_ops;
mod parser;
mod dependency;
mod resolver;

use anyhow::Result;
use args::{Cli, Commands};
use clap::Parser;
use colored::*;
use std::{ env, process::Command };

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Search { query } => {
            let local_pkgs = pacman::search(query)?;

            let aur_pkgs = aur::search(query).await?;

            // TODO: Refactor to alpm

            if !local_pkgs.is_empty() {
                println!("{}", ":: repo / local".blue().bold());
                for pkg in &local_pkgs {
                    let installed = if pkg.installed {
                        " [installed]".cyan()
                    } else {
                        "".clear()
                    };
                    println!(
                        "{}/{} {}{}",
                        pkg.repo.magenta(),
                        pkg.name.bold(),
                        pkg.version.green(),
                        installed
                    );
                    println!("    {}", pkg.description);
                }
            }
            if !aur_pkgs.is_empty() {
                println!("{}", ":: aur / remote".blue().bold());
                for pkg in &aur_pkgs {
                    println!("aur/{} {} ({})",
                             pkg.name.bold(),
                             pkg.version.green(),
                             format!("+{}", pkg.num_votes).yellow()
                    );

                    if let Some(desc) = &pkg.description {
                        println!("    {}", desc);
                    }
                }
            }

            if local_pkgs.is_empty() && aur_pkgs.is_empty() {
                println!("No packages found for '{}'", query);
            }
        }
        Commands::Get { package } => {
            let mut visited = Vec::new();
            let mut build_queue = Vec::new();
            let mut repo_queue = Vec::new();

            println!(":: Calculating dependency tree...");

            resolver::resolve_tree(&package, &mut visited, &mut build_queue, &mut repo_queue);

            if !repo_queue.is_empty() {
                println!("\n:: Installing {} official dependencies...", repo_queue.len());
                println!(":: Targets: {:?}", repo_queue);

                let mut pacman_cmd = Command::new("sudo");
                pacman_cmd.arg("pacman").arg("-S").arg("--needed");

                for dep in repo_queue {
                    pacman_cmd.arg(dep);
                }

                let status = pacman_cmd.status().expect("Failed to run pacman");

                if !status.success() {
                    eprintln!("!! Failed to install dependencies. Aborting.");
                    return Ok(());
                }
            } else {
                println!("\n:: No official dependencies to install.");
            }

            println!("\n:: Starting AUR build process for {} packages...", build_queue.len());

            for pkg in build_queue {
                let home = env::var("HOME").expect("No HOME");
                let cache_dir = format!("{}/.cache/raur/{}", home, pkg);

                println!(":: Building {}...", pkg);

                let status = Command::new("makepkg")
                    .arg("-si")
                    .current_dir(&cache_dir)
                    .status();

                match status {
                    Ok(s) if s.success() => println!(":: {} installed successfully!", pkg),
                    _ => {
                        eprintln!("!! Failed to build {}. Aborting queue.", pkg);
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
