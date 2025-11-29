mod args;
mod aur;
mod core;
mod pacman;

use anyhow::Result;
use args::{Cli, Commands};
use clap::Parser;
use colored::*;

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
    }
    Ok(())
}
