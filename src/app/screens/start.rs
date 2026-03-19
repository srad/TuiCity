use crate::app::{input::Action, save, ClickArea};

use super::{AppContext, LoadCityScreen, NewCityScreen, Screen, ScreenTransition, SettingsScreen};

pub struct StartState {
    pub selected: usize,
    pub menu_areas: [ClickArea; 4],
}

impl Default for StartState {
    fn default() -> Self {
        Self {
            selected: 0,
            menu_areas: [ClickArea::default(); 4],
        }
    }
}

pub struct StartScreen {
    pub state: StartState,
}

impl StartScreen {
    pub fn new() -> Self {
        Self {
            state: StartState::default(),
        }
    }
}

impl Screen for StartScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_action(&mut self, action: Action, _context: AppContext) -> Option<ScreenTransition> {
        const N: usize = 4;
        match action {
            Action::Quit | Action::CharInput('q') => Some(ScreenTransition::Quit),
            Action::MoveCursor(_, dy) => {
                if dy > 0 {
                    self.state.selected = (self.state.selected + 1) % N;
                } else if dy < 0 {
                    self.state.selected = self.state.selected.checked_sub(1).unwrap_or(N - 1);
                }
                None
            }
            Action::MouseClick { col, row } => {
                for (idx, area) in self.state.menu_areas.iter().enumerate() {
                    if area.contains(col, row) {
                        self.state.selected = idx;
                        return self.activate_selected();
                    }
                }
                None
            }
            Action::MenuSelect => self.activate_selected(),
            _ => None,
        }
    }

    fn build_view(&self, _context: AppContext<'_>) -> crate::ui::view::ScreenView {
        crate::ui::view::ScreenView::Start(crate::ui::view::StartViewModel {
            selected: self.state.selected,
            options: [
                "Load Existing City",
                "Create New City",
                "Settings",
                "Quit",
            ],
        })
    }
}

impl StartScreen {
    fn activate_selected(&self) -> Option<ScreenTransition> {
        match self.state.selected {
            0 => {
                let saves = save::list_saves();
                Some(ScreenTransition::Push(Box::new(LoadCityScreen {
                    state: super::load_city::LoadCityState {
                        saves,
                        selected: 0,
                        row_areas: Vec::new(),
                    },
                })))
            }
            1 => Some(ScreenTransition::Push(Box::new(NewCityScreen {
                state: super::new_city::NewCityState::new(),
            }))),
            2 => Some(ScreenTransition::Push(Box::new(SettingsScreen::new()))),
            3 => Some(ScreenTransition::Quit),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ClickArea;
    use std::sync::{Arc, RwLock};

    #[test]
    fn mouse_click_on_menu_item_activates_selection() {
        let mut screen = StartScreen::new();
        screen.state.menu_areas[1] = ClickArea {
            x: 10,
            y: 5,
            width: 20,
            height: 1,
        };

        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = None;
        let mut running = true;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };

        let transition = screen.on_action(Action::MouseClick { col: 12, row: 5 }, context);
        assert!(matches!(transition, Some(ScreenTransition::Push(_))));
        assert_eq!(screen.state.selected, 1);
    }
}
