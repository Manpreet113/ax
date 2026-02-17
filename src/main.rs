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
            } else if let Some(proj_dirs) =
                directories::ProjectDirs::from("com", "manpreet113", "ax")
            {
                proj_dirs.cache_dir().to_path_buf()
            } else {
                std::env::var("HOME")
                    .ok()
                    .map(|h| std::path::PathBuf::from(format!("{}/.cache/ax", h)))
                    .unwrap_or_else(|| std::path::PathBuf::from(".cache/ax"))
            };

            let pkg_cache = cache_base.join(&pkgbase);

            // Read dir
            let mut overrides = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&pkg_cache) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let fname = path.file_name().unwrap().to_string_lossy();

                    // Support multiple compression formats: .zst, .xz, .gz, and uncompressed
                    let is_package = fname.contains(".pkg.tar.") || fname.ends_with(".pkg.tar");

                    if is_package {
                        // Filename format: name-version-arch.pkg.tar.{zst,xz,gz}
                        // Improved matching: check exact package name with version separator
                        let mut should_install = false;
                        for needed in &visited {
                            // Match pattern: "pkgname-" followed by version number
                            // This prevents libfoo matching libfoo-extra
                            if fname.starts_with(&format!("{}-", needed)) {
                                // Additional verification: ensure next char after name is digit or version
                                let after_name = &fname[needed.len() + 1..];
                                if after_name
                                    .chars()
                                    .next()
                                    .map(|c| c.is_ascii_digit())
                                    .unwrap_or(false)
                                {
                                    should_install = true;
                                    break;
                                }
                            }
                        }

                        if should_install {
                            overrides.push(path);
                        }
                    }
                }
            }

            if !overrides.is_empty() {
                println!(
                    ":: Installing built packages: {:?}",
                    overrides
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
