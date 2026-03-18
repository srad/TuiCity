use crate::{
    app::{input::Action, ClickArea},
    core::{
        engine::EngineCommand,
        map::{gen, Map},
    },
};

use super::{AppContext, InGameScreen, Screen, ScreenTransition};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    pub city_name: String,
    pub seed_input: String,
    pub water_pct: usize,
    pub trees_pct: usize,
    pub field_areas: [ClickArea; 7],
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

        Self {
            seed,
            preview_map,
            focused_field: NewCityField::CityName,
            city_name: String::new(),
            seed_input: format!("{seed:016X}"),
            water_pct: params.water_pct as usize,
            trees_pct: params.trees_pct as usize,
            field_areas: [ClickArea::default(); 7],
        }
    }

    pub fn regenerate(&mut self) {
        let raw_seed = rand::random::<u64>() & 0x0000_FFFF_FFFF_FFFF;
        let w = self.water_pct as u8;
        let t = self.trees_pct as u8;
        self.seed = Self::pack_seed(w, t, raw_seed);
        self.seed_input = format!("{:016X}", self.seed);
        self.rebuild_map();
    }

    pub fn apply_seed_input(&mut self) {
        let text = self.seed_input.clone();
        let raw = text.trim().trim_start_matches("0x").trim_start_matches("0X");
        let parsed = u64::from_str_radix(raw, 16).or_else(|_| text.trim().parse::<u64>());
        if let Ok(new_seed) = parsed {
            self.seed = new_seed;
            let (w, t, _) = Self::unpack_seed(self.seed);
            self.water_pct = w as usize;
            self.trees_pct = t as usize;
        }
        self.seed_input = format!("{:016X}", self.seed);
        self.rebuild_map();
    }

    pub fn sync_sliders_to_seed(&mut self) {
        let w = self.water_pct as u8;
        let t = self.trees_pct as u8;
        let (_, _, raw) = Self::unpack_seed(self.seed);
        self.seed = Self::pack_seed(w, t, raw);
        self.seed_input = format!("{:016X}", self.seed);
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
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
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
            Action::MouseClick { col, row } => {
                for (idx, area) in self.state.field_areas.iter().enumerate() {
                    if area.contains(col, row) {
                        let field = NewCityField::ALL[idx];
                        self.state.focused_field = field;
                        return match field {
                            NewCityField::RegenerateBtn => {
                                self.state.regenerate();
                                None
                            }
                            NewCityField::StartBtn => self.start_city(context),
                            NewCityField::BackBtn => Some(ScreenTransition::Pop),
                            NewCityField::WaterSlider => {
                                let relative = col.saturating_sub(area.x).min(area.width.saturating_sub(1));
                                self.state.water_pct = ((relative as u32 * 100) / area.width.max(1) as u32) as usize;
                                self.state.sync_sliders_to_seed();
                                self.state.rebuild_map();
                                None
                            }
                            NewCityField::TreesSlider => {
                                let relative = col.saturating_sub(area.x).min(area.width.saturating_sub(1));
                                self.state.trees_pct = ((relative as u32 * 100) / area.width.max(1) as u32) as usize;
                                self.state.sync_sliders_to_seed();
                                self.state.rebuild_map();
                                None
                            }
                            _ => None,
                        };
                    }
                }
                None
            }
            Action::DeleteChar => {
                match self.state.focused_field {
                    NewCityField::CityName => {
                        self.state.city_name.pop();
                    }
                    NewCityField::SeedInput => {
                        self.state.seed_input.pop();
                        self.state.apply_seed_input();
                    }
                    _ => {}
                }
                None
            }
            Action::CharInput(c) => {
                match self.state.focused_field {
                    NewCityField::CityName => {
                        if !c.is_control() {
                            self.state.city_name.push(c);
                        }
                    }
                    NewCityField::SeedInput => {
                        if c.is_ascii_hexdigit() {
                            self.state.seed_input.push(c.to_ascii_uppercase());
                            self.state.apply_seed_input();
                        }
                    }
                    NewCityField::WaterSlider => {
                        if c == '-' {
                            self.state.water_pct = self.state.water_pct.saturating_sub(1);
                            self.state.sync_sliders_to_seed();
                            self.state.rebuild_map();
                        } else if c == '+' {
                            self.state.water_pct = (self.state.water_pct + 1).min(100);
                            self.state.sync_sliders_to_seed();
                            self.state.rebuild_map();
                        }
                    }
                    NewCityField::TreesSlider => {
                        if c == '-' {
                            self.state.trees_pct = self.state.trees_pct.saturating_sub(1);
                            self.state.sync_sliders_to_seed();
                            self.state.rebuild_map();
                        } else if c == '+' {
                            self.state.trees_pct = (self.state.trees_pct + 1).min(100);
                            self.state.sync_sliders_to_seed();
                            self.state.rebuild_map();
                        }
                    }
                    _ => {}
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
            _ => None,
        }
    }

    fn build_view(&self, _context: AppContext<'_>) -> crate::ui::view::ScreenView {
        crate::ui::view::ScreenView::NewCity(crate::ui::view::NewCityViewModel {
            preview_map: self.state.preview_map.clone(),
            focused_field: self.state.focused_field,
            city_name: self.state.city_name.clone(),
            seed_text: self.state.seed_input.clone(),
            water_pct: self.state.water_pct,
            trees_pct: self.state.trees_pct,
        })
    }
}

impl NewCityScreen {
    fn start_city(&mut self, context: AppContext) -> Option<ScreenTransition> {
        let name = if self.state.city_name.trim().is_empty() {
            "New City".to_string()
        } else {
            self.state.city_name.clone()
        };
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::ReplaceState {
                map: self.state.preview_map.clone(),
                sim: crate::core::sim::SimState::default(),
            });
            let _ = tx.send(EngineCommand::SetCityName(name));
        }
        if self.state.city_name.trim().is_empty() {
            None
        } else {
            Some(ScreenTransition::Replace(Box::new(InGameScreen::new())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let transition = screen.on_action(Action::MenuSelect, context);
        assert!(transition.is_none());
    }

    #[test]
    fn test_new_city_screen_fields_and_start_flow() {
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

        screen.state.city_name = "Test City".to_string();
        screen.state.focused_field = NewCityField::SeedInput;
        screen.state.seed_input.clear();

        screen.on_action(Action::CharInput('1'), context);
        assert_eq!(screen.state.seed_input, "0000000000000001");

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
        let transition_start = screen.on_action(Action::MenuSelect, AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        });
        assert!(transition_start.is_some());
    }
}
