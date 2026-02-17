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
mod lock;

mod cli;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Phase 12: Single Instance Lock
    // We bind it to a variable so it stays alive until end of main
    let _lock = lock::Lock::acquire()?;

    let mut config = config::Config::load()?;
    check_tools()?; 
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
                match upgrade::check_updates(&config).await {
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
    let mut cycle_path = Vec::new();
    let mut build_queue = Vec::new();
    let mut repo_queue = Vec::new();

    println!("{}", ":: Calculating dependency tree...".blue().bold());

    for pkg in packages {
        resolver::resolve_tree(
            pkg,
            &arch_db,
            &mut visited,
            &mut cycle_path,
            &mut build_queue,
            &mut repo_queue,
            config
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

        for pkgbase in build_queue {
            builder::build_package(&pkgbase, config, config.diff_viewer)?;
            
            // Post-build: Identify which .pkg.tar.zst files to install.
            // We want to install files that match:
            // 1. The requested 'pkg' (if it was a sub-package of this base)
            // 2. Any dependencies of requested packages that are provided by this base.
            
            // For now, a simple heuristic:
            // If the user requested 'gcc-libs', and we built 'gcc', we look for 'gcc-libs-*.pkg.tar.zst'.
            // If the user requested 'gcc', we look for 'gcc-*.pkg.tar.zst'.
            
            // To do this robustly, we need to know WHICH packages from this base are actually needed.
            // Our 'repo_queue' tracks repo deps, but 'build_queue' just tracks bases.
            // 'packages' arg contains the root requests.
            // We might need to track "aur_deps" separately.
            
            // FOR PHASE 6 MVP:
            // We will list ALL .pkg.tar.zst files in the cache dir.
            // We will filter them: if the filename starts with any string in `visited` (which contains all resolved nodes),
            // we install it.
            
            let cache_base = if let Some(ref dir) = config.build_dir {
                std::path::PathBuf::from(dir)
            } else if let Some(proj_dirs) = directories::ProjectDirs::from("com", "ax", "ax") {
                proj_dirs.cache_dir().to_path_buf()
            } else {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                std::path::PathBuf::from(format!("{}/.cache/ax", home))
            };
            
            let pkg_cache = cache_base.join(&pkgbase);
            
            // Read dir
            let mut overrides = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&pkg_cache) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "zst" { // .pkg.tar.zst
                            let fname = path.file_name().unwrap().to_string_lossy();
                            // Check if this file corresponds to a package in `visited`.
                            // Filename format: name-version-arch.pkg.tar.zst
                            // Heuristic: check if fname starts with "{name}-" for any name in visited.
                            
                            let mut should_install = false;
                            for needed in &visited {
                                if fname.starts_with(&format!("{}-", needed)) {
                                    should_install = true;
                                    break;
                                }
                            }
                            
                            if should_install {
                                overrides.push(path);
                            }
                        }
                    }
                }
            }
            
            if !overrides.is_empty() {
                println!(":: Installing built packages: {:?}", overrides.iter().map(|p| p.file_name().unwrap()).collect::<Vec<_>>());
                 let mut cmd = Command::new("sudo");
                 cmd.arg("pacman").arg("-U"); // No --noconfirm: Allow interactive conflict resolution (Phase 10)
                 for p in overrides {
                     cmd.arg(p);
                 }
                 
                 let status = cmd.status().context("Failed to install AUR package")?;
                 if !status.success() {
                     anyhow::bail!("Failed to install {}", pkgbase);
                 }
            } else {
                 println!("!! No matching packages found to install for {}", pkgbase);
            }
        }
    }

    Ok(())
}

fn check_tools() -> Result<()> {
    let tools = ["git", "pacman", "makepkg"];
    for tool in tools {
        if Command::new("which")
            .arg(tool)
            .stdout(std::process::Stdio::null())
            .status()
            .map(|s| !s.success())
            .unwrap_or(true) 
        {
            anyhow::bail!("Required tool '{}' not found in PATH.", tool);
        }
    }
    Ok(())
}