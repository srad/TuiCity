use crate::{
    app::{config, input::Action, ClickArea},
    ui::theme::{self, ALL_THEME_PRESETS},
};

use super::{AppContext, Screen, ScreenTransition};

pub struct ThemeSettingsState {
    pub selected: usize,
    pub row_areas: Vec<ClickArea>,
}

impl ThemeSettingsState {
    pub fn new() -> Self {
        let selected = ALL_THEME_PRESETS
            .iter()
            .position(|theme| *theme == theme::current_theme())
            .unwrap_or(0);
        Self {
            selected,
            row_areas: Vec::new(),
        }
    }
}

pub struct ThemeSettingsScreen {
    pub state: ThemeSettingsState,
}

impl ThemeSettingsScreen {
    pub fn new() -> Self {
        Self {
            state: ThemeSettingsState::new(),
        }
    }

    fn apply_selected_theme(&self) {
        if let Some(theme) = ALL_THEME_PRESETS.get(self.state.selected).copied() {
            theme::set_theme(theme);
            let _ = config::persist_theme_preference(theme);
        }
    }

    fn select_theme(&mut self, selected: usize) {
        self.state.selected = selected.min(ALL_THEME_PRESETS.len().saturating_sub(1));
        self.apply_selected_theme();
    }
}

impl Screen for ThemeSettingsScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_action(&mut self, action: Action, _context: AppContext) -> Option<ScreenTransition> {
        let item_count = ALL_THEME_PRESETS.len() + 1; // themes + back
        match action {
            Action::MenuBack => Some(ScreenTransition::Pop),
            Action::MenuSelect => {
                if self.state.selected >= ALL_THEME_PRESETS.len() {
                    return Some(ScreenTransition::Pop);
                }
                self.apply_selected_theme();
                None
            }
            Action::MoveCursor(_, dy) => {
                if item_count > 0 {
                    let next = if dy > 0 {
                        (self.state.selected + 1) % item_count
                    } else {
                        self.state.selected.checked_sub(1).unwrap_or(item_count - 1)
                    };
                    self.state.selected = next;
                    // Live-preview theme when hovering a theme item
                    if next < ALL_THEME_PRESETS.len() {
                        self.apply_selected_theme();
                    }
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
                    if idx >= ALL_THEME_PRESETS.len() {
                        return Some(ScreenTransition::Pop);
                    }
                    self.select_theme(idx);
                }
                None
            }
            Action::CharInput('P') => {
                let next = theme::cycle_theme();
                let _ = config::persist_theme_preference(next);
                self.state.selected = ALL_THEME_PRESETS
                    .iter()
                    .position(|preset| *preset == next)
                    .unwrap_or(self.state.selected);
                None
            }
            _ => None,
        }
    }

    fn build_view(&self, _context: AppContext<'_>) -> crate::ui::view::ScreenView {
        crate::ui::view::ScreenView::ThemeSettings(crate::ui::view::ThemeSettingsViewModel {
            themes: ALL_THEME_PRESETS.to_vec(),
            selected: self.state.selected,
            active: theme::current_theme(),
        })
    }
}
