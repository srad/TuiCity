use std::{fs, io, path::PathBuf};

use crate::textgen::models::{LlmExecutionMode, LlmModelId};
use crate::ui::theme::{self, ThemePreset};

use super::save;

const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontendKind {
    #[default]
    #[serde(alias = "terminal")]
    TerminalAscii,
    #[serde(alias = "pixels_gui")]
    TerminalVga,
}

impl FrontendKind {
    pub fn label(self) -> &'static str {
        match self {
            FrontendKind::TerminalAscii => "Terminal ASCII",
            FrontendKind::TerminalVga => "Terminal VGA",
        }
    }

    pub fn next(self) -> Self {
        match self {
            FrontendKind::TerminalAscii => FrontendKind::TerminalVga,
            FrontendKind::TerminalVga => FrontendKind::TerminalAscii,
        }
    }

    pub fn is_vga(self) -> bool {
        matches!(self, FrontendKind::TerminalVga)
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

pub fn persist_default_llm_preference_if_model_present(model_present: bool) -> io::Result<()> {
    let mut config = load_user_config();
    if apply_default_llm_preference(&mut config, model_present) {
        save_user_config(&config)?;
    }
    Ok(())
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

fn apply_default_llm_preference(config: &mut UserConfig, model_present: bool) -> bool {
    if model_present && config.llm_enabled.is_none() {
        config.llm_enabled = Some(true);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_default_llm_preference, FrontendKind, UserConfig};

    #[test]
    fn frontend_kind_labels_match_terminal_modes() {
        assert_eq!(FrontendKind::TerminalAscii.label(), "Terminal ASCII");
        assert_eq!(FrontendKind::TerminalVga.label(), "Terminal VGA");
    }

    #[test]
    fn frontend_kind_deserializes_legacy_values() {
        let ascii: FrontendKind = serde_json::from_str("\"terminal\"").expect("legacy terminal");
        let vga: FrontendKind = serde_json::from_str("\"pixels_gui\"").expect("legacy pixels");
        assert_eq!(ascii, FrontendKind::TerminalAscii);
        assert_eq!(vga, FrontendKind::TerminalVga);
    }

    #[test]
    fn model_presence_persists_default_llm_enablement_when_unset() {
        let mut config = UserConfig::default();

        let changed = apply_default_llm_preference(&mut config, true);

        assert!(changed);
        assert_eq!(config.llm_enabled, Some(true));
    }

    #[test]
    fn explicit_llm_disable_is_preserved_even_if_model_exists() {
        let mut config = UserConfig {
            llm_enabled: Some(false),
            ..UserConfig::default()
        };

        let changed = apply_default_llm_preference(&mut config, true);

        assert!(!changed);
        assert_eq!(config.llm_enabled, Some(false));
    }
}
