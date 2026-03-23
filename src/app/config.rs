use std::{fs, io, path::PathBuf};

use crate::textgen::models::{LlmExecutionMode, LlmModelId};
use crate::ui::theme::{self, ThemePreset};

use super::save;

const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontendKind {
    #[default]
    Terminal,
    PixelsGui,
}

impl FrontendKind {
    pub fn label(self) -> &'static str {
        match self {
            FrontendKind::Terminal => "Terminal",
            FrontendKind::PixelsGui => "Pixel GUI",
        }
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub theme: Option<ThemePreset>,
    #[serde(default)]
    pub music_enabled: Option<bool>,
    #[serde(default)]
    pub frontend: Option<FrontendKind>,
    #[serde(default)]
    pub llm_enabled: Option<bool>,
    #[serde(default)]
    pub llm_model: Option<LlmModelId>,
    #[serde(default)]
    pub llm_execution_mode: Option<LlmExecutionMode>,
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

pub fn is_music_enabled() -> bool {
    load_user_config().music_enabled.unwrap_or(true)
}

pub fn persist_music_preference(enabled: bool) -> io::Result<()> {
    let mut config = load_user_config();
    config.music_enabled = Some(enabled);
    save_user_config(&config)
}

pub fn get_frontend_kind() -> FrontendKind {
    load_user_config().frontend.unwrap_or_default()
}

pub fn persist_frontend_preference(kind: FrontendKind) -> io::Result<()> {
    let mut config = load_user_config();
    config.frontend = Some(kind);
    save_user_config(&config)
}

pub fn is_llm_enabled() -> bool {
    load_user_config().llm_enabled.unwrap_or(true)
}

pub fn persist_llm_preference(enabled: bool) -> io::Result<()> {
    let mut config = load_user_config();
    config.llm_enabled = Some(enabled);
    save_user_config(&config)
}

pub fn get_llm_model() -> LlmModelId {
    load_user_config().llm_model.unwrap_or_default()
}

pub fn persist_llm_model_preference(model: LlmModelId) -> io::Result<()> {
    let mut config = load_user_config();
    config.llm_model = Some(model);
    save_user_config(&config)
}

pub fn get_llm_execution_mode() -> LlmExecutionMode {
    load_user_config().llm_execution_mode.unwrap_or_default()
}

pub fn persist_llm_execution_mode_preference(mode: LlmExecutionMode) -> io::Result<()> {
    let mut config = load_user_config();
    config.llm_execution_mode = Some(mode);
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
