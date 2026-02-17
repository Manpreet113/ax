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

    Ok(parse_selection(input, max))
}

fn parse_selection(input: &str, max: usize) -> Vec<usize> {
    let mut selected = Vec::new();

    for part in input.split_whitespace() {
        if part.contains('-') {
            let range: Vec<&str> = part.split('-').collect();
            if range.len() == 2 {
                if let (Ok(start_raw), Ok(end_raw)) = (range[0].parse::<usize>(), range[1].parse::<usize>()) {
                    let start = std::cmp::min(start_raw, end_raw);
                    let end = std::cmp::max(start_raw, end_raw);

                    for i in start..=end {
                        if i > 0 && i <= max {
                            selected.push(i - 1);
                        }
                    }
                }
            }
        } else {
            if let Ok(idx) = part.parse::<usize>() {
                if idx > 0 && idx <= max {
                    selected.push(idx - 1);
                }
            }
        }
    }

    selected.sort();
    selected.dedup();
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_selection() {
        assert_eq!(parse_selection("1-3", 5), vec![0, 1, 2]);
        assert_eq!(parse_selection("1-5", 5), vec![0, 1, 2, 3, 4]);
        assert_eq!(parse_selection("2", 5), vec![1]);
        assert_eq!(parse_selection("1 3 5", 5), vec![0, 2, 4]);
        assert_eq!(parse_selection("1 99", 5), vec![0]); 
        assert_eq!(parse_selection("5-1", 5), vec![0, 1, 2, 3, 4]); // Handles reverse 5-1 -> min(1,5)..max(1,5) -> 1..5
        assert!(parse_selection("0", 5).is_empty());
        assert!(parse_selection("invalid", 5).is_empty());
    }
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

pub fn prompt_continue() -> Result<bool> {
    print!(":: Proceed with build? [Y/n] ");
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
