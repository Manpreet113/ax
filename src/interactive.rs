use crate::api::AurPackage;
use crate::arch::RepoPackage;
use colored::*;
use std::io::{self, Write};
use anyhow::Result;

pub enum SearchResult {
    Repo(RepoPackage),
    Aur(AurPackage),
}

impl SearchResult {
    pub fn name(&self) -> &str {
        match self {
            SearchResult::Repo(p) => &p.name,
            SearchResult::Aur(p) => &p.name,
        }
    }
}

pub fn show_results(results: &[SearchResult]) {
    for (i, result) in results.iter().enumerate() {
        let idx = i + 1;
        match result {
            SearchResult::Repo(pkg) => {
                println!(
                    "{} {}/{} {} {}",
                    format!("{}:", idx).magenta(),
                    pkg.db.magenta().bold(),
                    pkg.name.bold(),
                    pkg.version.green(),
                    "(Repo)".cyan()
                );
                if let Some(desc) = &pkg.description {
                    println!("    {}", desc);
                }
            }
            SearchResult::Aur(pkg) => {
                println!(
                    "{} {}/{} {} {} {}",
                    format!("{}:", idx).magenta(),
                    "aur".magenta().bold(),
                    pkg.name.bold(),
                    pkg.version.green(),
                    format!("(+{})", pkg.num_votes.unwrap_or(0)).cyan(),
                    "(AUR)".cyan()
                );
                if let Some(desc) = &pkg.description {
                    println!("    {}", desc);
                }
            }
        }
    }
}

pub fn get_user_selection(max: usize) -> Result<Vec<usize>> {
    print!("{}", ":: Packages to install (eg: 1 2 3, 1-3): ".bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        return Ok(vec![]);
    }

    let mut selected = Vec::new();

    for part in input.split_whitespace() {
        if part.contains('-') {
            let range: Vec<&str> = part.split('-').collect();
            if range.len() == 2 {
                let start: usize = range[0].parse()?;
                let end: usize = range[1].parse()?;
                for i in start..=end {
                    if i > 0 && i <= max {
                        selected.push(i - 1);
                    }
                }
            }
        } else {
            let idx: usize = part.parse()?;
            if idx > 0 && idx <= max {
                selected.push(idx - 1);
            }
        }
    }

    selected.sort();
    selected.dedup();

    Ok(selected)
}

pub fn prompt_review(pkg: &str) -> Result<bool> {
    print!(":: Review PKGBUILD for {}? [Y/n] ", pkg.bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input == "y" || input == "yes" || input.is_empty() {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn prompt_diff(pkg: &str) -> Result<bool> {
    print!(":: View diff for {}? [Y/n] ", pkg.bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input == "y" || input == "yes" || input.is_empty() {
        Ok(true)
    } else {
        Ok(false)
    }
}
