use crate::arch::ArchDB;
use crate::parser::PackageMetaData;
use anyhow::Result;
use colored::*;


// ========== NEW: DAG-Based Batch Resolution ==========

use crate::graph::DependencyGraph;
use std::collections::{HashMap, HashSet};
use log::debug;

#[derive(Debug)]
pub struct ResolutionPlan {
    pub repo_deps: Vec<String>,
    pub build_order: Vec<String>,
}

/// Phase 1: Collect all package names that need resolution
async fn collect_all_packages(
    packages: &[String],
    arch_db: &ArchDB,
    config: &crate::config::Config,
) -> Result<(HashMap<String, PackageMetaData>, HashSet<String>)> {
    let mut aur_packages = HashMap::new();
    let mut repo_packages = HashSet::new();
    let mut to_process = packages.to_vec();
    let mut processed = HashSet::new();

    while let Some(pkg) = to_process.pop() {
        if processed.contains(&pkg) {
            continue;
        }
        processed.insert(pkg.clone());

        // Check if in repo
        if arch_db.exists_in_repo(&pkg) {
            debug!("Found {} in official repository", pkg);
            repo_packages.insert(pkg);
            continue;
        }

        debug!("{} not in repo, checking AUR", pkg);

        // Must be in AUR - don't insert now, allow metadata parsing to drive insertion

        // Clone and parse PKGBUILD to get dependencies
        // Clone and parse PKGBUILD to get dependencies
        let cache_base = config.get_cache_dir();

        let cache_path = cache_base.join(&pkg);
        if !cache_path.exists() {
            let aur_url = format!("https://aur.archlinux.org/{}.git", pkg);
            crate::git_ops::clone_repo(&aur_url, &cache_path)?;
        } else {
            crate::git_ops::pull_repo(&cache_path)?;
        }

        // Parse .SRCINFO for dependencies
        if let Ok(metadata) = crate::parser::parse_srcinfo(&cache_path) {
            aur_packages.insert(pkg.clone(), metadata.clone());
            for dep in metadata.depends.iter().chain(metadata.make_depends.iter()) {
                let clean_dep = crate::parser::clean_dependency(dep);
                if !processed.contains(&clean_dep) {
                    to_process.push(clean_dep);
                }
            }
        }
    }

    Ok((aur_packages, repo_packages))
}

/// Phase 2: Build dependency graph and get topological order
pub async fn resolve_with_dag(
    packages: &[String],
    arch_db: &ArchDB,
    config: &crate::config::Config,
) -> Result<ResolutionPlan> {
    println!(
        "{}",
        ":: Phase 1: Collecting all dependencies...".blue().bold()
    );
    let (aur_packages, repo_packages) = collect_all_packages(packages, arch_db, config).await?;

    println!(
        ":: Found {} AUR packages and {} repo packages",
        aur_packages.len(),
        repo_packages.len()
    );

    // Build dependency graph
    println!(
        "{}",
        ":: Phase 2: Building dependency graph...".blue().bold()
    );
    let mut graph = DependencyGraph::new();
    let mut pkgbase_map: HashMap<String, String> = HashMap::new();

    // Add all AUR packages to graph and map pkgnames -> pkgbase
    // Pass 1: Add all nodes and populate pkgbase_map
    for metadata in aur_packages.values() {
        let pkgbase = &metadata.pkgbase;
        graph.add_node(pkgbase);

        for pkgname in &metadata.pkgnames {
            pkgbase_map.insert(pkgname.clone(), pkgbase.clone());
        }
    }

    // Pass 2: Add edges for dependencies
    for metadata in aur_packages.values() {
        let pkgbase = &metadata.pkgbase;
        for dep in metadata.depends.iter().chain(metadata.make_depends.iter()) {
            let clean_dep = crate::parser::clean_dependency(dep);

            // Only add edge if dependency is an AUR package
            // But we must resolve the dependency name to its pkgbase if possible
            // We check if clean_dep is in pkgbase_map, which implies it's in AUR packages we found
            if let Some(dep_base) = pkgbase_map.get(&clean_dep) {
                 graph.add_edge(pkgbase, dep_base);
            } else if aur_packages.contains_key(&clean_dep) {
                 // Fallback: if we found it in aur_packages but somehow missed mapping (shouldn't happen)
                 // or if it's a direct match
                 graph.add_edge(pkgbase, &clean_dep);
            }
        }
    }

    println!(":: Graph has {} nodes", graph.node_count());

    // Get topological order
    println!("{}", ":: Phase 3: Computing build order...".blue().bold());
    let build_order = match graph.topological_order() {
        Ok(order) => order,
        Err(e) => {
            eprintln!("{} {}", "!! Warning:".yellow(), e);
            eprintln!(
                "{}",
                ":: Falling back to discovery order due to circular dependencies".yellow()
            );
            // Fallback to simple ordering (deterministic)
            let mut pkgs: Vec<_> = aur_packages
                .keys()
                .filter_map(|pkg| pkgbase_map.get(pkg).cloned())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            pkgs.sort();
            pkgs
        }
    };

    // Filter out packages that are already installed and up-to-date
    let final_build_order: Vec<String> = build_order
        .into_iter()
        .filter(|pkgbase| {
            if let Some(metadata) = aur_packages.get(pkgbase) {
                // Check if any package in the split package set is installed
                // Usually check the main package or all of them.
                // Simplified: Check if *all* pkgnames in this base are installed and up to date?
                // Or just if ANY is outdated?
                // Conservative approach: If ANY pkgname in the base is NOT installed or OUTDATED, build.
                // If ALL represent packages are installed AND up to date, skip.

                let is_vcs = pkgbase.ends_with("-git")
                    || pkgbase.ends_with("-hg")
                    || pkgbase.ends_with("-svn")
                    || pkgbase.ends_with("-bzr")
                    || pkgbase.ends_with("-cvs")
                    || pkgbase.ends_with("-darcs")
                    || pkgbase.ends_with("-fossil");

                let mut all_up_to_date = true;
                for pkgname in &metadata.pkgnames {
                    match arch_db.get_installed_version(pkgname) {
                        Some(ver) => {
                            // If VCS package, just being installed is enough
                            if !is_vcs
                                && crate::arch::ArchDB::vercmp(&ver, &metadata.version)
                                    != std::cmp::Ordering::Equal
                            {
                                all_up_to_date = false;
                                break;
                            }
                        }
                        None => {
                            all_up_to_date = false;
                            break;
                        }
                    }
                }

                if all_up_to_date {
                    debug!("Skipping {} (up to date)", pkgbase);
                    println!(
                        "{} {} {}",
                        ":: Skipping".yellow(),
                        pkgbase.bold(),
                        format!("(up to date: {})", metadata.version).green()
                    );
                    return false;
                }
            }
            true
        })
        .collect();

    Ok(ResolutionPlan {
        repo_deps: repo_packages.into_iter().collect(),
        build_order: final_build_order,
    })
}
