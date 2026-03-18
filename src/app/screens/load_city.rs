use crate::app::{input::Action, save, ClickArea};

use super::{AppContext, InGameScreen, Screen, ScreenTransition};

pub struct LoadCityState {
    pub saves: Vec<save::SaveEntry>,
    pub selected: usize,
    pub row_areas: Vec<ClickArea>,
}

pub struct LoadCityScreen {
    pub state: LoadCityState,
}

impl Screen for LoadCityScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition> {
        let count = self.state.saves.len();
        match action {
            Action::MenuBack => Some(ScreenTransition::Pop),
            Action::MoveCursor(_, dy) => {
                if count > 0 {
                    self.state.selected = if dy > 0 {
                        (self.state.selected + 1) % count
                    } else {
                        self.state.selected.checked_sub(1).unwrap_or(count.saturating_sub(1))
                    };
                }
                None
            }
            Action::MouseClick { col, row } => {
                for (idx, area) in self.state.row_areas.iter().enumerate() {
                    if area.contains(col, row) {
                        self.state.selected = idx;
                        return None;
                    }
                }
                None
            }
            Action::MenuSelect => {
                if let Some(entry) = self.state.saves.get(self.state.selected) {
                    match save::load_city(&entry.path) {
                        Ok((map, sim)) => {
                            if let Some(tx) = context.cmd_tx {
                                let _ = tx.send(crate::core::engine::EngineCommand::ReplaceState { map, sim });
                            }
                            Some(ScreenTransition::Replace(Box::new(InGameScreen::new())))
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn build_view(&self, _context: AppContext<'_>) -> crate::ui::view::ScreenView {
        crate::ui::view::ScreenView::LoadCity(crate::ui::view::LoadCityViewModel {
            saves: self.state.saves.clone(),
            selected: self.state.selected,
        })
    }
}
