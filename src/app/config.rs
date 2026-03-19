use std::{fs, io, path::PathBuf};

use crate::ui::theme::{self, ThemePreset};

use super::save;

const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub theme: Option<ThemePreset>,
}

pub fn apply_user_config() {
    if let Some(theme) = load_user_config().theme {
        theme::set_theme(theme);
    }
}

pub fn persist_theme_preference(theme: ThemePreset) -> io::Result<()> {
    let mut config = load_user_config();
    config.theme = Some(theme);
    save_user_config(&config)
}

fn load_user_config() -> UserConfig {
    let Ok(json) = fs::read_to_string(config_path()) else {
        return UserConfig::default();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

fn save_user_config(config: &UserConfig) -> io::Result<()> {
    let dir = save::app_data_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(config).map_err(io::Error::other)?;
    fs::write(config_path(), json)
}

fn config_path() -> PathBuf {
    save::app_data_dir().join(CONFIG_FILE_NAME)
}
