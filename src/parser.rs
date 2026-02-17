use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Debug, Default)]
pub struct PackageMetaData {
    pub pkgbase: String,
    pub version: String,
    pub depends: Vec<String>,
    pub make_depends: Vec<String>,
    pub validpgpkeys: Vec<String>,
    pub arch: Vec<String>,
    pub pkgnames: Vec<String>,
}

pub fn clean_dependency(dep: &str) -> String {
    if let Some(idx) = dep.find(['>', '<', '=']) {
        return dep[..idx].to_string();
    }
    dep.to_string()
}

pub fn parse_srcinfo(path: &Path) -> Result<PackageMetaData> {
    let file = File::open(path.join(".SRCINFO")).context("Could not open .SRCINFO")?;
    let reader = io::BufReader::new(file);

    let mut metadata = PackageMetaData::default();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            // Handle inline comments like "value # comment"
            let value = value.split('#').next().unwrap_or("").trim().to_string();

            match key {
                "pkgbase" => metadata.pkgbase = value,
                "pkgver" => metadata.version = value,
                "pkgname" => metadata.pkgnames.push(value),
                "depends" => metadata.depends.push(value),
                "makedepends" => metadata.make_depends.push(value),
                "validpgpkeys" => metadata.validpgpkeys.push(value),
                "arch" => metadata.arch.push(value),
                _ => {}
            }
        }
    }

    Ok(metadata)
}
