use anyhow::Result;
use directories::ProjectDirs;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub build_dir: Option<String>,
    pub editor: Option<String>,
    pub clean_build: bool,
    pub show_news: bool,
    pub diff_viewer: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            build_dir: None,
            editor: None,
            clean_build: false,
            show_news: true,
            diff_viewer: true,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "manpreet113", "ax") {
            let config_dir = proj_dirs.config_dir();
            let config_path = config_dir.join("config.toml");

            // Check for old config location and migrate
            if !config_path.exists()
                && let Some(old_dirs) = ProjectDirs::from("com", "ax", "ax")
            {
                let old_path = old_dirs.config_dir().join("config.toml");
                if old_path.exists() {
                    eprintln!(":: Migrating config from old location...");
                    fs::create_dir_all(config_dir)?;
                    fs::copy(&old_path, &config_path)?;
                }
            }

            if config_path.exists() {
                let content = fs::read_to_string(config_path)?;
                let config: Config = toml::from_str(&content)?;
                return Ok(config);
            }
        }
        Ok(Self::default())
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "manpreet113", "ax") {
            let config_dir = proj_dirs.config_dir();
            fs::create_dir_all(config_dir)?;

            let config_path = config_dir.join("config.toml");
            let content = toml::to_string_pretty(self)?;

            // Use file locking to prevent corruption from concurrent saves
            let mut file = File::create(&config_path)?;
            file.lock_exclusive()?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
            file.unlock()?;
        }
        Ok(())
    }
}
