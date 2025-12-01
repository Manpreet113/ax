use std::process::Command;
use std::collections::HashSet;
pub fn clean_dependency(dep: &str) -> String {
    if let Some(idx) = dep.find(|c| c == '>' || c == '<' || c == '=') {
        return dep[..idx].to_string();
    }
    dep.to_string()
}

pub struct DepStatus {
    pub repo_install: Vec<String>,
    pub aur_build: Vec<String>,
}

pub fn classify_dependencies(deps: Vec<String>) -> DepStatus {
    let mut cmd = Command::new("pacman");
    cmd.arg("-T");
    for dep in &deps {
        cmd.arg(dep);
    }

    let output = cmd.output().expect("Failed to run pacman -T");
    let missing_raw = String::from_utf8(output.stdout).expect("Invalid UTF8");

    let missing_deps: Vec<String> = missing_raw
        .lines()
        .map(|s| s.trim().to_string())
        .collect();

    if missing_deps.is_empty() {
        return DepStatus { repo_install: vec![], aur_build: vec![] };
    }

    let mut cmd_repo = Command::new("pacman");
    cmd_repo.arg("-Ssq");

    for dep in &missing_deps {
        let clean = clean_dependency(dep);
        cmd_repo.arg(format!("^{}$", clean));
    }

    let output_repo = cmd_repo.output().expect("Failed to run pacman -Ssq");
    let repo_found_raw = String::from_utf8(output_repo.stdout).expect("Invalid UTF8");
    let repo_set: HashSet<String> = repo_found_raw
        .lines()
        .map(|s| s.trim().to_string())
        .collect();

    let mut final_repo = Vec::new();
    let mut final_aur = Vec::new();

    for dep in missing_deps {
        let clean = clean_dependency(&dep);
        if repo_set.contains(&clean) {
            final_repo.push(clean);
        } else {
            final_aur.push(clean);
        }
    }

    DepStatus {
        repo_install: final_repo,
        aur_build: final_aur,
    }
}