use anyhow::Result;
use srcinfo::Srcinfo;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct PackageMetaData {
    pub pkgbase: String,
    #[allow(dead_code)]
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
    let srcinfo_path = path.join(".SRCINFO");

    let srcinfo = Srcinfo::from_path(&srcinfo_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse .SRCINFO at {:?}: {:?}", srcinfo_path, e))?;

    let mut metadata = PackageMetaData {
        pkgbase: srcinfo.pkgbase().to_string(),
        version: srcinfo.version().to_string(),
        ..Default::default()
    };

    // Collect architectures
    for arch in srcinfo.arch() {
        metadata.arch.push(arch.to_string());
    }

    // Get the current system architecture
    let current_arch = std::env::consts::ARCH;

    // Collect makedepends (global + arch-specific)
    for depends_arch in srcinfo.makedepends() {
        // Check if this is for our arch or global
        if depends_arch.arch().is_none() || depends_arch.arch() == Some(current_arch) {
            for depend in depends_arch.iter() {
                metadata.make_depends.push(depend.to_string());
            }
        }
    }

    // Collect validpgpkeys
    for key in srcinfo.valid_pgp_keys() {
        metadata.validpgpkeys.push(key.to_string());
    }

    // Iterate through all packages (base + split packages)
    for pkg in srcinfo.pkgs() {
        metadata.pkgnames.push(pkg.pkgname().to_string());

        // Collect package-specific depends (global + arch-specific)
        for depends_arch in pkg.depends() {
            if depends_arch.arch().is_none() || depends_arch.arch() == Some(current_arch) {
                for depend in depends_arch.iter() {
                    let depend_str = depend.to_string();
                    if !metadata.depends.contains(&depend_str) {
                        metadata.depends.push(depend_str);
                    }
                }
            }
        }
    }

    Ok(metadata)
}
