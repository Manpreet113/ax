use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::env;
use std::process::Command;

mod api;
mod arch;
mod git_ops;
mod gpg;
mod parser;
mod resolver;
mod upgrade;

#[derive(Parser)]
#[command(name = "raur")]
#[command(about = "Repo Unified Helper", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Get {
        package: String,
    },
    Upgrade,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Upgrade) => {
            if let Err(e) = upgrade::check_updates().await {
                eprintln!("{} {:#}", "!! Upgrade check failed:".red().bold(), e);
            }
        }
        Some(Commands::Get { package }) => {
            // Initialize ALPM
            let arch_db = arch::ArchDB::new()
                .context("Failed to initialize ALPM database. Is pacman locked?")?;

            let mut visited = Vec::new();
            let mut build_queue = Vec::new();
            let mut repo_queue = Vec::new();

            println!("{}", ":: Calculating dependency tree...".blue().bold());

            resolver::resolve_tree(
                &package,
                &arch_db,
                &mut visited,
                &mut build_queue,
                &mut repo_queue
            ).await?;

            // Phase 1: Install Official Deps
            if !repo_queue.is_empty() {
                println!("\n{}", ":: Installing official dependencies...".yellow().bold());
                println!(":: Targets: {:?}", repo_queue);

                let mut pacman_cmd = Command::new("sudo");
                pacman_cmd.arg("pacman").arg("-S").arg("--needed");

                for dep in repo_queue {
                    pacman_cmd.arg(dep);
                }

                let status = pacman_cmd.status().context("Failed to execute sudo pacman")?;

                if !status.success() {
                    anyhow::bail!("Failed to install official dependencies. Aborting.");
                }
            } else {
                println!("\n:: No official dependencies to install.");
            }

            // Phase 2: Build AUR Deps
            println!("\n:: Starting AUR build process for {} packages...", build_queue.len().to_string().green());

            for pkg in build_queue {
                let home = env::var("HOME").context("Could not find HOME directory")?;
                let cache_dir = format!("{}/.cache/raur/{}", home, pkg);

                println!(":: Building {}...", pkg.cyan());

                let status = Command::new("makepkg")
                    .arg("-si")
                    .current_dir(&cache_dir)
                    .status()
                    .context("Failed to execute makepkg")?; // The '?' unwraps it to ExitStatus

                // Fix: Check status directly, no match Ok() needed
                if status.success() {
                    println!(":: {} {}", pkg.green(), "installed successfully!".green());
                } else {
                    anyhow::bail!("Failed to build {}. Aborting queue.", pkg);
                }
            }
        }
        None => {
            println!("Try 'raur --help'");
        }
    }

    Ok(())
}