use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;
use directories::ProjectDirs;

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
        if let Some(proj_dirs) = ProjectDirs::from("com", "raur", "raur") {
            let config_dir = proj_dirs.config_dir();
            let config_path = config_dir.join("config.toml");

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
        if let Some(proj_dirs) = ProjectDirs::from("com", "raur", "raur") {
            let config_dir = proj_dirs.config_dir();
            fs::create_dir_all(config_dir)?;
            
            let config_path = config_dir.join("config.toml");
            let content = toml::to_string_pretty(self)?;
            fs::write(config_path, content)?;
        }
        Ok(())
    }
}
