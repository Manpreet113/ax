use alpm::{Alpm, SigLevel};
use anyhow::{Context, Result};
use std::cmp::Ordering;
use std::rc::Rc;

pub struct ArchDB {
    handle: Rc<Alpm>,
}

pub struct RepoPackage {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub db: String,
}

impl ArchDB {
    pub fn new() -> Result<Self> {
        let handle = Alpm::new("/", "/var/lib/pacman")?;
        let sync_dbs = vec!["core", "extra", "multilib"];

        for db_name in sync_dbs {
            handle
                .register_syncdb(db_name, SigLevel::USE_DEFAULT)
                .with_context(|| format!("Failed to register DB: {}", db_name))?;
        }

        Ok(Self {
            handle: Rc::new(handle),
        })
    }

    pub fn exists_in_repo(&self, pkg_name: &str) -> bool {
        let dbs = self.handle.syncdbs();
        for db in dbs {
            if db.pkgs().find_satisfier(pkg_name).is_some() {
                return true;
            }
        }
        false
    }

    pub fn search(&self, query: &str) -> Result<Vec<RepoPackage>> {
        let mut results = Vec::new();
        let dbs = self.handle.syncdbs();

        for db in dbs {
            let pkgs = db.search([query].iter())?;
            for pkg in pkgs {
                results.push(RepoPackage {
                    name: pkg.name().to_string(),
                    version: pkg.version().to_string(),
                    description: pkg.desc().map(|s| s.to_string()),
                    db: db.name().to_string(),
                });
            }
        }
        Ok(results)
    }

    pub fn get_foreign_packages(&self) -> Result<Vec<RepoPackage>> {
        let local_db = self.handle.localdb();
        let sync_dbs = self.handle.syncdbs();
        let mut foreign_pkgs = Vec::new();

        for pkg in local_db.pkgs() {
            let pkg_name = pkg.name();
            let mut found = false;

            for db in sync_dbs {
                if db.pkg(pkg_name).is_ok() {
                    found = true;
                    break;
                }
            }

            if !found {
                foreign_pkgs.push(RepoPackage {
                    name: pkg.name().to_string(),
                    version: pkg.version().to_string(),
                    description: pkg.desc().map(|s| s.to_string()),
                    db: "local".to_string(),
                });
            }
        }

        Ok(foreign_pkgs)
    }
    pub fn get_installed_version(&self, pkg_name: &str) -> Option<String> {
        let local_db = self.handle.localdb();
        local_db.pkg(pkg_name).ok().map(|p| p.version().to_string())
    }

    pub fn vercmp(v1: &str, v2: &str) -> Ordering {
        alpm::vercmp(v1, v2)
    }
}
