use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::{APP_NAME, api::typing::TypingClass};

const FILENAME: &str = "settings.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub typing_username: Option<String>,
    pub typing_class: Option<TypingClass>,

    #[serde(skip)]
    loaded_from: PathBuf,
}

impl Settings {
    pub fn load_or_default() -> Self {
        let path = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|parent| parent.join(FILENAME)))
            .filter(|path| path.is_file())
            .or_else(|| {
                ProjectDirs::from("", "", APP_NAME).map(|dirs| dirs.config_dir().join(FILENAME))
            })
            .unwrap_or_else(|| PathBuf::from(FILENAME));
        let mut settings: Settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|contents| toml::from_str(&contents).ok())
            .unwrap_or_default();
        settings.loaded_from = path;
        settings
    }

    pub fn save(&self) -> Result<()> {
        let path = if self.loaded_from.as_os_str().is_empty() {
            ProjectDirs::from("", "", APP_NAME).map_or_else(
                || PathBuf::from("settings.toml"),
                |dirs| dirs.config_dir().join("settings.toml"),
            )
        } else {
            self.loaded_from.clone()
        };
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config dir at '{}'", parent.display())
            })?;
        }

        let contents = toml::to_string_pretty(self).context("failed to serialize settings")?;
        std::fs::write(&path, contents)
            .with_context(|| format!("failed to write settings file {}", path.display()))?;
        Ok(())
    }
}
