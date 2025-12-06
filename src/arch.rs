use alpm::{Alpm, SigLevel};
use anyhow::{Context, Result};
use std::sync::Arc;


pub struct ArchDB {
    handle: Arc<Alpm>,
}

impl ArchDB {
    pub fn new() -> Result<Self> {
        let handle = Alpm::new("/", "/var/lib/pacman")?;
        let sync_dbs = vec!["core", "extra", "multilib"];

        for db_name in sync_dbs {
            handle.register_syncdb(db_name, SigLevel::USE_DEFAULT)
                .with_context(|| format!("Failed to register DB: {}", db_name))?;
        }

        Ok(Self {
            handle: Arc::new(handle),
        })
    }

    pub fn is_installed(&self, dep_string: &str) -> bool {
        let local_db = self.handle.localdb();

        local_db.pkgs().find_satisfier(dep_string).is_some()
    }

    pub fn exists_in_repo(&self, pkg_name: &str) -> bool {
        let dbs = self.handle.syncdbs();
        for db in dbs {
            if db.pkg(pkg_name).is_ok() {
                return true;
            }
        }
        false
    }
}