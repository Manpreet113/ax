use crate::{git_ops, parser, dependency};
use std::{env, path::Path};

pub fn resolve_tree(pkg: &str, visited: &mut Vec<String>, build_queue: &mut Vec<String>, repo_queue: &mut Vec<String>){
    if visited.contains(&pkg.to_string()){
        return;
    }
    visited.push(pkg.to_string());

    println!("Resolving dependencies for:  {}", pkg);

    let home = env::var("HOME").expect("NO HOME");
    let cache_dir = format!("{}/.cache/raur/{}", home, pkg);
    let aur_url = format!("https://aur.archlinux.org/{}.git", pkg);
    let path = Path::new(&cache_dir);

    if !path.exists() {
        if let Err(e) = git_ops::clone_repo(&aur_url, path) {
            eprintln!("!! Failed to clone {}: {}", pkg, e);
            return;
        }
    } else {
        if let Err(e) = git_ops::pull_repo(path) {
            eprintln!("!! Failed to update {}: {}", pkg, e);
        }
    }
    // TODO: handle "Package not found"

    let meta = match parser::parse_srcinfo(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("!! Error parsing .SRCINFO for {}: {}", pkg, e);
            build_queue.push(pkg.to_string());
            return;
        }
    };

    crate::gpg::ensure_keys(&meta.validpgpkeys);

    println!(":: Analyzing {} dependencies...", meta.depends.len());

    let status = dependency::classify_dependencies(meta.depends);

    for repo_dep in status.repo_install {
        if !repo_queue.contains(&repo_dep) {
            println!("   -> Adding {} to repo queue", repo_dep);
            repo_queue.push(repo_dep);
        }
    }

    for aur_dep in status.aur_build {
        println!("   -> AUR Dep found: {}", aur_dep);
        resolve_tree(&aur_dep, visited, build_queue, repo_queue);
    }

    build_queue.push(pkg.to_string());
}