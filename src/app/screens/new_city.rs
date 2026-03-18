use crate::{
    app::input::Action,
    core::{
        engine::EngineCommand,
        map::{gen, Map},
    },
};

use super::{AppContext, InGameScreen, Screen, ScreenTransition};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NewCityField {
    CityName,
    SeedInput,
    WaterSlider,
    TreesSlider,
    RegenerateBtn,
    StartBtn,
    BackBtn,
}

impl NewCityField {
    const ALL: [NewCityField; 7] = [
        NewCityField::CityName,
        NewCityField::SeedInput,
        NewCityField::WaterSlider,
        NewCityField::TreesSlider,
        NewCityField::RegenerateBtn,
        NewCityField::StartBtn,
        NewCityField::BackBtn,
    ];

    pub fn next(self) -> Self {
        crate::ui::runtime::cycle_next(self, &Self::ALL)
    }

    pub fn prev(self) -> Self {
        crate::ui::runtime::cycle_prev(self, &Self::ALL)
    }
}

pub struct NewCityState {
    pub seed: u64,
    pub preview_map: Map,
    pub focused_field: NewCityField,
    pub city_name: rat_widget::text_input::TextInputState,
    pub seed_input: rat_widget::text_input::TextInputState,
    pub water_slider: rat_widget::slider::SliderState,
    pub trees_slider: rat_widget::slider::SliderState,
    pub regen_btn: rat_widget::button::ButtonState,
    pub start_btn: rat_widget::button::ButtonState,
    pub back_btn: rat_widget::button::ButtonState,
}

impl NewCityState {
    pub fn pack_seed(water_pct: u8, trees_pct: u8, raw_seed: u64) -> u64 {
        let w = (water_pct.min(100) as u64) << 56;
        let t = (trees_pct.min(100) as u64) << 48;
        let s = raw_seed & 0x0000_FFFF_FFFF_FFFF;
        w | t | s
    }

    pub fn unpack_seed(packed: u64) -> (u8, u8, u64) {
        let w = ((packed >> 56) & 0xFF) as u8;
        let t = ((packed >> 48) & 0xFF) as u8;
        let s = packed & 0x0000_FFFF_FFFF_FFFF;
        (w.min(100), t.min(100), s)
    }

    pub fn new() -> Self {
        let raw_seed = rand::random::<u64>() & 0x0000_FFFF_FFFF_FFFF;
        let params = gen::GenParams {
            seed: raw_seed,
            ..Default::default()
        };
        let preview_map = gen::generate(&params);
        let seed = Self::pack_seed(params.water_pct, params.trees_pct, raw_seed);

        let city_name = rat_widget::text_input::TextInputState::default();
        let mut seed_input = rat_widget::text_input::TextInputState::default();
        seed_input.set_text(format!("{seed:016X}"));

        let mut water_slider = rat_widget::slider::SliderState::default();
        water_slider.set_value(params.water_pct as usize);
        let mut trees_slider = rat_widget::slider::SliderState::default();
        trees_slider.set_value(params.trees_pct as usize);

        Self {
            seed,
            preview_map,
            focused_field: NewCityField::CityName,
            city_name,
            seed_input,
            water_slider,
            trees_slider,
            regen_btn: rat_widget::button::ButtonState::default(),
            start_btn: rat_widget::button::ButtonState::default(),
            back_btn: rat_widget::button::ButtonState::default(),
        }
    }

    pub fn regenerate(&mut self) {
        let raw_seed = rand::random::<u64>() & 0x0000_FFFF_FFFF_FFFF;
        let w = self.water_slider.value() as u8;
        let t = self.trees_slider.value() as u8;
        self.seed = Self::pack_seed(w, t, raw_seed);
        self.seed_input.set_text(format!("{:016X}", self.seed));
        self.rebuild_map();
    }

    pub fn apply_seed_input(&mut self) {
        let text = self.seed_input.text();
        let raw = text.trim().trim_start_matches("0x").trim_start_matches("0X");
        let parsed = u64::from_str_radix(raw, 16).or_else(|_| text.trim().parse::<u64>());
        if let Ok(new_seed) = parsed {
            self.seed = new_seed;
            let (w, t, _) = Self::unpack_seed(self.seed);
            self.water_slider.set_value(w as usize);
            self.trees_slider.set_value(t as usize);
        }
        self.seed_input.set_text(format!("{:016X}", self.seed));
        self.rebuild_map();
    }

    pub fn sync_sliders_to_seed(&mut self) {
        let w = self.water_slider.value() as u8;
        let t = self.trees_slider.value() as u8;
        let (_, _, raw) = Self::unpack_seed(self.seed);
        self.seed = Self::pack_seed(w, t, raw);
        self.seed_input.set_text(format!("{:016X}", self.seed));
    }

    pub fn rebuild_map(&mut self) {
        let (w, t, raw_seed) = Self::unpack_seed(self.seed);
        let params = gen::GenParams {
            water_pct: w,
            trees_pct: t,
            seed: raw_seed,
            ..Default::default()
        };
        self.preview_map = gen::generate(&params);
    }
}

pub struct NewCityScreen {
    pub state: NewCityState,
}

impl Screen for NewCityScreen {
    fn on_event(&mut self, event: &crossterm::event::Event, context: AppContext) -> Option<ScreenTransition> {
        use rat_widget::event::{ButtonOutcome, SliderOutcome, TextOutcome};

        let focus_city = self.state.focused_field == NewCityField::CityName;
        let focus_seed = self.state.focused_field == NewCityField::SeedInput;
        let focus_water = self.state.focused_field == NewCityField::WaterSlider;
        let focus_trees = self.state.focused_field == NewCityField::TreesSlider;
        let focus_regen = self.state.focused_field == NewCityField::RegenerateBtn;
        let focus_start = self.state.focused_field == NewCityField::StartBtn;
        let focus_back = self.state.focused_field == NewCityField::BackBtn;

        let _out_city = rat_widget::text_input::handle_events(&mut self.state.city_name, focus_city, event);
        let out_seed = rat_widget::text_input::handle_events(&mut self.state.seed_input, focus_seed, event);
        let out_water = rat_widget::slider::handle_events(&mut self.state.water_slider, focus_water, event);
        let out_trees = rat_widget::slider::handle_events(&mut self.state.trees_slider, focus_trees, event);
        let out_regen = rat_widget::button::handle_events(&mut self.state.regen_btn, focus_regen, event);
        let out_start = rat_widget::button::handle_events(&mut self.state.start_btn, focus_start, event);
        let out_back = rat_widget::button::handle_events(&mut self.state.back_btn, focus_back, event);

        if out_seed == TextOutcome::TextChanged {
            self.state.apply_seed_input();
        }

        if out_water == SliderOutcome::Changed || out_trees == SliderOutcome::Changed {
            self.state.sync_sliders_to_seed();
            self.state.rebuild_map();
        }

        if out_regen == ButtonOutcome::Pressed {
            self.state.regenerate();
        }

        if out_start == ButtonOutcome::Pressed {
            return self.start_city(context);
        }

        if out_back == ButtonOutcome::Pressed {
            return Some(ScreenTransition::Pop);
        }

        None
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition> {
        match action {
            Action::MenuBack => Some(ScreenTransition::Pop),
            Action::MoveCursor(_dx, dy) => {
                if dy != 0 {
                    if self.state.focused_field == NewCityField::SeedInput {
                        self.state.apply_seed_input();
                    }
                    self.state.focused_field = if dy > 0 {
                        self.state.focused_field.next()
                    } else {
                        self.state.focused_field.prev()
                    };
                }
                None
            }
            Action::MenuSelect => match self.state.focused_field {
                NewCityField::SeedInput => {
                    self.state.apply_seed_input();
                    None
                }
                NewCityField::RegenerateBtn => {
                    self.state.regenerate();
                    None
                }
                NewCityField::StartBtn => self.start_city(context),
                NewCityField::BackBtn => Some(ScreenTransition::Pop),
                _ => None,
            },
            Action::MouseClick { col, row } => {
                if col >= 30 {
                    self.state.focused_field = if row == 2 || row == 3 {
                        NewCityField::CityName
                    } else if row == 5 || row == 6 {
                        NewCityField::SeedInput
                    } else if row == 8 || row == 9 {
                        NewCityField::WaterSlider
                    } else if row == 11 || row == 12 {
                        NewCityField::TreesSlider
                    } else if row == 15 {
                        NewCityField::RegenerateBtn
                    } else if row == 17 {
                        NewCityField::StartBtn
                    } else if row == 19 {
                        NewCityField::BackBtn
                    } else {
                        self.state.focused_field
                    };
                }
                None
            }
            _ => None,
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame, _context: AppContext) {
        let area = frame.area();
        crate::ui::screens::new_city::render_new_city(frame, area, &mut self.state);
    }
}

impl NewCityScreen {
    fn start_city(&mut self, context: AppContext) -> Option<ScreenTransition> {
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::ReplaceState {
                map: self.state.preview_map.clone(),
                sim: crate::core::sim::SimState::default(),
            });
            let name = if self.state.city_name.text().is_empty() {
                "New City".to_string()
            } else {
                self.state.city_name.text().to_string()
            };
            let _ = tx.send(EngineCommand::SetCityName(name));
        }
        if self.state.city_name.text().is_empty() {
            None
        } else {
            Some(ScreenTransition::Replace(Box::new(InGameScreen::new())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_new_city_screen_requires_name() {
        let mut screen = NewCityScreen { state: NewCityState::new() };
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

        screen.state.focused_field = NewCityField::StartBtn;
        let ev_enter = Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        let transition = screen.on_event(&ev_enter, context);
        assert!(transition.is_none());
    }

    #[test]
    fn test_new_city_screen_rat_widget() {
        let mut screen = NewCityScreen { state: NewCityState::new() };
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

        screen.state.city_name.set_text("Test City");
        screen.state.focused_field = NewCityField::SeedInput;
        screen.state.seed_input.set_text("");

        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        screen.on_event(&ev, context);
        assert_eq!(screen.state.seed_input.text(), "0000000000000001");

        screen.state.apply_seed_input();
        assert_eq!(screen.state.seed, 1);

        let context_action = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let transition = screen.on_action(Action::MenuSelect, context_action);
        assert!(transition.is_none());

        screen.state.focused_field = NewCityField::StartBtn;
        let ev_enter = Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        let transition_start = screen.on_event(&ev_enter, AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        });
        assert!(transition_start.is_some());
    }
}
