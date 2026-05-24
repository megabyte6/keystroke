use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::{APP_NAME, api::typing::TypingClass};

const FILENAME: &str = "settings.toml";

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create config dir at {0}: {1}")]
    CreateDir(PathBuf, #[source] std::io::Error),

    #[error("failed to serialize settings: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("failed to write settings file {0}: {1}")]
    WriteFile(PathBuf, #[source] std::io::Error),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub typing_username: Option<String>,
    pub typing_class: Option<TypingClass>,

    #[serde(skip)]
    loaded_from: Option<PathBuf>,
}

impl Settings {
    pub fn load_or_default() -> Self {
        let path = resolve_config_path();
        let mut settings: Settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|contents| toml::from_str(&contents).ok())
            .unwrap_or_default();
        settings.loaded_from = Some(path);
        settings
    }

    pub fn save(&self) -> Result<()> {
        let path = self.loaded_from.clone().unwrap_or_else(resolve_config_path);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::CreateDir(parent.to_path_buf(), e))?;
        }

        let contents = toml::to_string_pretty(self).map_err(Error::Serialize)?;
        std::fs::write(&path, contents).map_err(|e| Error::WriteFile(path.clone(), e))?;
        Ok(())
    }
}

fn resolve_config_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(FILENAME)))
        .filter(|p| p.is_file())
        .or_else(|| {
            ProjectDirs::from("", "", APP_NAME).map(|dirs| dirs.config_dir().join(FILENAME))
        })
        .unwrap_or_else(|| PathBuf::from(FILENAME))
}
