use crate::app::{input::Action, save};

use super::{LoadCityScreen, NewCityScreen, Screen, ScreenTransition, AppContext};

#[derive(Default)]
pub struct StartState {
    pub selected: usize,
    pub menu_areas: [crate::app::ClickArea; 3],
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
    fn on_action(&mut self, action: Action, _context: AppContext) -> Option<ScreenTransition> {
        const N: usize = 3;
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
                for (i, area) in self.state.menu_areas.iter().enumerate() {
                    if area.contains(col, row) {
                        self.state.selected = i;
                        return self.activate_selected();
                    }
                }
                None
            }
            Action::MenuSelect => self.activate_selected(),
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame, _context: AppContext) {
        let area = frame.area();
        crate::ui::screens::start::render_start(frame, area, &mut self.state);
    }
}

impl StartScreen {
    fn activate_selected(&self) -> Option<ScreenTransition> {
        match self.state.selected {
            0 => {
                let saves = save::list_saves();
                Some(ScreenTransition::Push(Box::new(LoadCityScreen {
                    state: super::load_city::LoadCityState { saves, selected: 0 },
                })))
            }
            1 => Some(ScreenTransition::Push(Box::new(NewCityScreen {
                state: super::new_city::NewCityState::new(),
            }))),
            2 => Some(ScreenTransition::Quit),
            _ => None,
        }
    }
}
