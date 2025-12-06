use anyhow::{Context, Result};
use clap::{Parser, CommandFactory};
use colored::*;
use std::process::Command;

mod api;
mod builder;
mod arch;
mod git_ops;
mod gpg;
mod parser;
mod resolver;
mod upgrade;
mod interactive;
mod config;
mod news;

mod cli;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = config::Config::load()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Sync { refresh: _, sysupgrade, cleanbuild, packages }) => {
            if cleanbuild {
                config.clean_build = true;
            }

            if sysupgrade {
                if config.show_news {
                    if let Err(e) = news::check_news().await {
                        eprintln!("{} {}", "!! Failed to fetch news:".red(), e);
                    }
                }

                println!("{}", ":: Starting system upgrade...".blue().bold());
                let status = Command::new("sudo")
                    .arg("pacman")
                    .arg("-Syu")
                    .status()
                    .context("Failed to execute sudo pacman -Syu")?;

                if !status.success() {
                    anyhow::bail!("System upgrade failed");
                }

                println!("{}", ":: Checking for AUR updates...".blue().bold());
                match upgrade::check_updates().await {
                    Ok(updates) => {
                        if !updates.is_empty() {
                            install_packages(&updates, &config).await?;
                        }
                    }
                    Err(e) => eprintln!("{} {:#}", "!! Upgrade check failed:".red().bold(), e),
                }
            }

            if !packages.is_empty() {
                install_packages(&packages, &config).await?;
            }
        }
        Some(Commands::Remove { packages }) => {
            if !packages.is_empty() {
                let mut cmd = Command::new("sudo");
                cmd.arg("pacman").arg("-R").arg("-s").args(&packages);
                cmd.status().context("Failed to execute sudo pacman -R")?;
            }
        }
        None => {
            if !cli.query.is_empty() {
                let query = cli.query.join(" ");
                search_and_install(&query, &config).await?;
            } else {
                Cli::command().print_help()?;
            }
        }
    }

    Ok(())
}

async fn search_and_install(query: &str, config: &config::Config) -> Result<()> {
    let arch_db = arch::ArchDB::new().context("Failed to initialize ALPM")?;
    
    println!("{}", ":: Searching...".blue().bold());

    let repo_results = arch_db.search(query)?;
    let aur_results = api::search(query).await?;

    let mut results = Vec::new();
    for r in repo_results {
        results.push(interactive::SearchResult::Repo(r));
    }
    for r in aur_results {
        results.push(interactive::SearchResult::Aur(r));
    }

    if results.is_empty() {
        println!("No results found for '{}'", query);
        return Ok(());
    }

    interactive::show_results(&results);

    let selection = interactive::get_user_selection(results.len())?;
    if selection.is_empty() {
        return Ok(());
    }

    let mut packages_to_install = Vec::new();
    for idx in selection {
        packages_to_install.push(results[idx].name().to_string());
    }

    // Default to no cleanbuild for interactive search for now, or we could prompt?
    // For now, let's assume false because I'm too lazy to add another prompt.
    install_packages(&packages_to_install, config).await
}

async fn install_packages(packages: &[String], config: &config::Config) -> Result<()> {
    let arch_db = arch::ArchDB::new().context("Failed to initialize ALPM")?;

    let mut visited = Vec::new();
    let mut build_queue = Vec::new();
    let mut repo_queue = Vec::new();

    println!("{}", ":: Calculating dependency tree...".blue().bold());

    for pkg in packages {
        resolver::resolve_tree(
            pkg,
            &arch_db,
            &mut visited,
            &mut build_queue,
            &mut repo_queue
        ).await?;
    }

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
    }

    // Phase 2: Build AUR Deps
    if !build_queue.is_empty() {
        println!("\n:: Starting AUR build process for {} packages...", build_queue.len().to_string().green());

        for pkg in build_queue {
            builder::build_package(&pkg, config.clean_build, config.diff_viewer)?;
        }
    }

    Ok(())
}