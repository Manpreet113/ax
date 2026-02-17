use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use colored::*;
use std::process::Command;

mod api;
mod arch;
mod builder;
mod config;
mod git_ops;
mod gpg;
mod interactive;
mod lock;
mod news;
mod parser;
mod resolver;
mod upgrade;

mod cli;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Phase 12: Single Instance Lock
    // We bind it to a variable so it stays alive until end of main
    let _lock = lock::Lock::acquire()?;

    let mut config = config::Config::load()?;
    check_tools()?;
    check_interactive()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Sync {
            refresh: _,
            sysupgrade,
            cleanbuild,
            packages,
        }) => {
            // Separate pacman flags from package names
            let (pacman_flags, pkg_names): (Vec<String>, Vec<String>) =
                packages.into_iter().partition(|arg| arg.starts_with('-'));

            if cleanbuild {
                config.clean_build = true;
            }

            if sysupgrade {
                if config.show_news
                    && let Err(e) = news::check_news().await
                {
                    eprintln!("{} {}", "!! Failed to fetch news:".red(), e);
                }

                println!("{}", ":: Starting system upgrade...".blue().bold());
                let mut cmd = Command::new("sudo");
                cmd.arg("pacman").arg("-Syu");

                // Forward user-provided pacman flags
                for flag in &pacman_flags {
                    cmd.arg(flag);
                }

                let status = cmd.status().context("Failed to execute sudo pacman -Syu")?;

                if !status.success() {
                    anyhow::bail!("System upgrade failed");
                }

                println!("{}", ":: Checking for AUR updates...".blue().bold());
                match upgrade::check_updates(&config).await {
                    Ok(updates) => {
                        if !updates.is_empty() {
                            install_packages(&updates, &config, &pacman_flags).await?;
                        }
                    }
                    Err(e) => eprintln!("{} {:#}", "!! Upgrade check failed:".red().bold(), e),
                }
            }

            if !pkg_names.is_empty() {
                install_packages(&pkg_names, &config, &pacman_flags).await?;
            }
        }
        Some(Commands::Remove { packages }) => {
            // Separate pacman flags from package names
            let (pacman_flags, pkg_names): (Vec<String>, Vec<String>) =
                packages.into_iter().partition(|arg| arg.starts_with('-'));

            if !pkg_names.is_empty() {
                let mut cmd = Command::new("sudo");
                cmd.arg("pacman").arg("-R").arg("-s");

                // Forward user-provided flags
                for flag in pacman_flags {
                    cmd.arg(flag);
                }

                cmd.args(&pkg_names);
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
    install_packages(&packages_to_install, config, &[]).await
}

async fn install_packages(
    packages: &[String],
    config: &config::Config,
    pacman_flags: &[String],
) -> Result<()> {
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
            config,
        )
        .await?;
    }

    // Phase 1: Install Official Deps
    if !repo_queue.is_empty() {
        println!(
            "\n{}",
            ":: Installing official dependencies...".yellow().bold()
        );
        println!(":: Targets: {:?}", repo_queue);

        let mut pacman_cmd = Command::new("sudo");
        pacman_cmd.arg("pacman").arg("-S").arg("--needed");

        for dep in repo_queue {
            pacman_cmd.arg(dep);
        }

        // Forward user-provided pacman flags
        for flag in pacman_flags {
            pacman_cmd.arg(flag);
        }

        let status = pacman_cmd
            .status()
            .context("Failed to execute sudo pacman")?;

        if !status.success() {
            anyhow::bail!("Failed to install official dependencies. Aborting.");
        }
    }

    // Phase 2: Build AUR Deps
    if !build_queue.is_empty() {
        println!(
            "\n:: Starting AUR build process for {} packages...",
            build_queue.len().to_string().green()
        );

        for pkgbase in build_queue {
            // build_package now returns the exact paths of packages to install
            loop {
                match builder::build_package(&pkgbase, config, config.diff_viewer) {
                    Ok(package_paths) => {
                        // Install the built packages using exact paths from makepkg --packagelist
                        if !package_paths.is_empty() {
                            println!(
                                ":: Installing built packages: {:?}",
                                package_paths
                                    .iter()
                                    .map(|p| p.file_name().unwrap())
                                    .collect::<Vec<_>>()
                            );
                            let mut cmd = Command::new("sudo");
                            cmd.arg("pacman").arg("-U"); // No --noconfirm: Allow interactive conflict resolution (Phase 10)

                            // Forward user-provided pacman flags
                            for flag in pacman_flags {
                                cmd.arg(flag);
                            }

                            for p in package_paths {
                                cmd.arg(p);
                            }

                            let status = cmd.status().context("Failed to install AUR package")?;
                            if !status.success() {
                                eprintln!("{} Failed to install {}", "!!".red(), pkgbase);

                                // Prompt for action on install failure
                                match interactive::prompt_on_error(
                                    &format!("Installation of {} failed", pkgbase),
                                    false, // No retry for pacman -U failures
                                )? {
                                    interactive::ErrorAction::Skip => {
                                        println!("{}", ":: Skipping package...".yellow());
                                        break;
                                    }
                                    interactive::ErrorAction::Abort
                                    | interactive::ErrorAction::Retry => {
                                        anyhow::bail!("Aborting due to installation failure");
                                    }
                                }
                            } else {
                                break; // Success, move to next package
                            }
                        } else {
                            println!("!! No packages were built for {}", pkgbase);
                            break;
                        }
                    }
                    Err(e) => {
                        // Build failed, prompt user
                        eprintln!(
                            "{} Build failed for {}: {:#}",
                            "!!".red().bold(),
                            pkgbase,
                            e
                        );

                        match interactive::prompt_on_error(
                            &format!("Build of {} failed", pkgbase),
                            true, // Allow retry for build failures
                        )? {
                            interactive::ErrorAction::Retry => {
                                println!("{}", ":: Retrying build...".yellow());
                                continue; // Retry loop
                            }
                            interactive::ErrorAction::Skip => {
                                println!("{}", ":: Skipping package...".yellow());
                                break; // Skip to next package
                            }
                            interactive::ErrorAction::Abort => {
                                anyhow::bail!("Aborting due to build failure");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn check_tools() -> Result<()> {
    let tools = ["git", "pacman", "makepkg"];
    for tool in tools {
        // Use 'command -v' instead of 'which' for better reliability
        // Works with shell aliases and is POSIX-compliant
        let status = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", tool))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if status.map(|s| !s.success()).unwrap_or(true) {
            anyhow::bail!(
                "Required tool '{}' not found. Please install it:\n  sudo pacman -S {}",
                tool,
                match tool {
                    "makepkg" => "base-devel",
                    _ => tool,
                }
            );
        }
    }
    Ok(())
}

fn check_interactive() -> Result<()> {
    // Check if we're running in an interactive terminal
    // This prevents sudo from hanging in non-interactive environments
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        eprintln!(
            "{}",
            "WARNING: ax is running in a non-interactive environment.".yellow()
        );
        eprintln!("Sudo commands may fail or hang. Consider running in an interactive terminal.");
        eprintln!();
    }

    Ok(())
}
