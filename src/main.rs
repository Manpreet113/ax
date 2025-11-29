mod args;
mod core;
mod pacman;

use clap::Parser;
use args::{ Commands, Cli };
use anyhow::Result;
use colored::*;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Search { query } => {
            let packages = pacman::search(query)?;

            if packages.is_empty() {
                println!("No packages found for {}", query);
                return Ok(());
            }

            for pkg in packages {
                let repo = format!("{}/", pkg.repo).magenta();
                let name = pkg.name.bold();
                let ver = pkg.version.green();
                let status = if pkg.installed {"[installed]".cyan()} else {"".clear()};

                println!("{}{} {}   {}", repo, name, ver, status);
                println!("    {}", pkg.description);
            }
        }
    }
    Ok(())
}
