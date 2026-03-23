use crate::{
    app::{config, input::Action, ClickArea},
    ui::theme,
};

use super::{AppContext, LlmSetupScreen, Screen, ScreenTransition, ThemeSettingsScreen};

pub struct SettingsState {
    pub selected: usize,
    pub row_areas: Vec<ClickArea>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            row_areas: Vec::new(),
        }
    }
}

pub struct SettingsScreen {
    pub state: SettingsState,
}

impl SettingsScreen {
    pub fn new() -> Self {
        Self {
            state: SettingsState::new(),
        }
    }

    fn item_count(&self) -> usize {
        5 // 4 options + back
    }

    fn activate_selected(&mut self) -> Option<ScreenTransition> {
        match self.state.selected {
            0 => Some(ScreenTransition::Push(Box::new(ThemeSettingsScreen::new()))),
            1 => {
                let next = theme::cycle_theme();
                let _ = config::persist_theme_preference(next);
                None
            }
            2 => {
                let current = config::is_music_enabled();
                let _ = config::persist_music_preference(!current);
                None
            }
            3 => Some(ScreenTransition::Push(Box::new(LlmSetupScreen::new()))),
            4 => Some(ScreenTransition::Pop), // Back (auto-appended by renderer)
            _ => None,
        }
    }

    pub fn view_model(&self, context: AppContext<'_>) -> crate::ui::view::SettingsViewModel {
        let music_label = if config::is_music_enabled() {
            "Disable Music"
        } else {
            "Enable Music"
        };
        let llm_status = if context.textgen.has_model() {
            crate::ui::view::LlmStatus::Active
        } else if cfg!(feature = "llm") {
            crate::ui::view::LlmStatus::Unavailable
        } else {
            crate::ui::view::LlmStatus::Disabled
        };

        crate::ui::view::SettingsViewModel {
            options: vec![
                "Theme Settings".to_string(),
                "Cycle Theme".to_string(),
                music_label.to_string(),
                "LLM Setup".to_string(),
            ],
            selected: self.state.selected,
            current_theme_label: theme::current_theme().label().to_string(),
            llm_status,
        }
    }
}

impl Screen for SettingsScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_action(&mut self, action: Action, _context: AppContext) -> Option<ScreenTransition> {
        let count = self.item_count();
        match action {
            Action::MenuBack => Some(ScreenTransition::Pop),
            Action::MoveCursor(_, dy) => {
                if dy > 0 {
                    self.state.selected = (self.state.selected + 1) % count;
                } else if dy < 0 {
                    self.state.selected = self.state.selected.checked_sub(1).unwrap_or(count - 1);
                }
                None
            }
            Action::MouseClick { col, row } => {
                if let Some(idx) = self
                    .state
                    .row_areas
                    .iter()
                    .position(|area| area.contains(col, row))
                {
                    self.state.selected = idx;
                    return self.activate_selected();
                }
                None
            }
            Action::MenuSelect => self.activate_selected(),
            Action::CharInput('P') => {
                let next = theme::cycle_theme();
                let _ = config::persist_theme_preference(next);
                None
            }
            Action::CharInput('M') | Action::CharInput('m') => {
                let current = config::is_music_enabled();
                let _ = config::persist_music_preference(!current);
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    fn test_context() -> (
        Arc<RwLock<crate::core::engine::SimulationEngine>>,
        crate::textgen::TextGenService,
    ) {
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(4, 4),
            crate::core::sim::SimState::default(),
        )));
        let textgen =
            crate::textgen::TextGenService::start(std::path::PathBuf::from("/nonexistent"));
        (engine, textgen)
    }

    #[test]
    fn settings_view_has_four_options_plus_auto_back() {
        let screen = SettingsScreen::new();
        let (engine, textgen) = test_context();
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            textgen: &textgen,
        };
        let view = screen.view_model(context);
        assert_eq!(view.options.len(), 4);
        assert!(view.options.iter().all(|option| !option.contains("Frontend")));
        // Back button is auto-appended by the renderer
    }

    #[test]
    fn llm_setup_pushes_new_screen() {
        let mut screen = SettingsScreen::new();
        screen.state.selected = 3;
        let (engine, textgen) = test_context();
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            textgen: &textgen,
        };
        let result = screen.on_action(Action::MenuSelect, context);
        assert!(matches!(result, Some(ScreenTransition::Push(_))));
    }
}
