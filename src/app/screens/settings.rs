use crate::{
    app::{config, input::Action, ClickArea},
    ui::theme,
};

use super::{AppContext, Screen, ScreenTransition, ThemeSettingsScreen};

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

    fn option_count(&self) -> usize {
        4
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
            3 => Some(ScreenTransition::Pop),
            _ => None,
        }
    }
}

impl Screen for SettingsScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_action(&mut self, action: Action, _context: AppContext) -> Option<ScreenTransition> {
        let count = self.option_count();
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

    fn build_view(&self, _context: AppContext<'_>) -> crate::ui::view::ScreenView {
        let music_label = if config::is_music_enabled() {
            "Disable Music"
        } else {
            "Enable Music"
        };
        crate::ui::view::ScreenView::Settings(crate::ui::view::SettingsViewModel {
            options: vec![
                "Theme Settings".to_string(),
                "Cycle Theme".to_string(),
                music_label.to_string(),
                "Back".to_string(),
            ],
            selected: self.state.selected,
            current_theme_label: theme::current_theme().label().to_string(),
        })
    }
}
