pub mod camera;
pub mod input;
pub mod line_drag;
pub mod rect_drag;
pub mod save;
pub mod screens;

use line_drag::LineDrag;
use rect_drag::RectDrag;

use camera::Camera;
use input::Action;
use crate::core::{
    map::{gen, Map},
    sim::SimState,
    tool::Tool,
};

// ── Click-area type (no ratatui dependency) ──────────────────────────────────

#[derive(Clone, Copy, Default, Debug)]
pub struct ClickArea {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl ClickArea {
    pub fn contains(&self, col: u16, row: u16) -> bool {
        self.width > 0
            && self.height > 0
            && col >= self.x
            && col < self.x + self.width
            && row >= self.y
            && row < self.y + self.height
    }
}

// ── Floating window ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct FloatingWindow {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl FloatingWindow {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    /// True when (col, row) is on the top border row (the draggable title bar).
    pub fn title_bar_contains(&self, col: u16, row: u16) -> bool {
        self.width > 0
            && row == self.y
            && col >= self.x
            && col < self.x + self.width
    }

    /// True when (col, row) is anywhere inside the window (including border).
    pub fn contains(&self, col: u16, row: u16) -> bool {
        self.width > 0
            && self.height > 0
            && col >= self.x
            && col < self.x + self.width
            && row >= self.y
            && row < self.y + self.height
    }
}

// ── Window drag state ─────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum WindowDrag {
    Map(u16, u16),
    Panel(u16, u16),
    Budget(u16, u16),
    Inspect(u16, u16),
}

// ── UI Areas (written by ui::render each frame) ───────────────────────────────

#[derive(Default)]
pub struct UiAreas {
    pub map: ClickArea,
    pub minimap: ClickArea,
    pub pause_btn: ClickArea,
    pub toolbar_buttons: Vec<(Tool, ClickArea)>,
    pub menu_bar_y: u16,
}

// ── Screen-specific state ─────────────────────────────────────────────────────

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
        let idx = Self::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

pub struct StartState {
    pub selected: usize,
    pub menu_areas: [ClickArea; 3],
}

impl Default for StartState {
    fn default() -> Self {
        Self {
            selected: 0,
            menu_areas: [ClickArea::default(); 3],
        }
    }
}

pub struct NewCityState {
    pub seed: u64, // The packed seed (water, trees, raw seed)
    pub preview_map: Map,
    pub focused_field: NewCityField,
    
    // rat-widget states
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
        let params = gen::GenParams { seed: raw_seed, ..Default::default() };
        let preview_map = gen::generate(&params);
        
        let seed = Self::pack_seed(params.water_pct, params.trees_pct, raw_seed);

        let city_name = rat_widget::text_input::TextInputState::default();
        let mut seed_input = rat_widget::text_input::TextInputState::default();
        seed_input.set_text(&format!("{:016X}", seed));
        
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

    /// Randomise seed and rebuild map.
    pub fn regenerate(&mut self) {
        let raw_seed = rand::random::<u64>() & 0x0000_FFFF_FFFF_FFFF;
        let w = self.water_slider.value() as u8;
        let t = self.trees_slider.value() as u8;
        self.seed = Self::pack_seed(w, t, raw_seed);
        self.seed_input.set_text(&format!("{:016X}", self.seed));
        self.rebuild_map();
    }

    /// Try to parse `seed_input` as a hex (or decimal) u64, then rebuild.
    pub fn apply_seed_input(&mut self) {
        let text = self.seed_input.text();
        let raw = text.trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let parsed = u64::from_str_radix(raw, 16)
            .or_else(|_| text.trim().parse::<u64>());
        if let Ok(new_seed) = parsed {
            self.seed = new_seed;
            let (w, t, _) = Self::unpack_seed(self.seed);
            self.water_slider.set_value(w as usize);
            self.trees_slider.set_value(t as usize);
        }
        // Always reformat to canonical hex so the field stays clean
        self.seed_input.set_text(&format!("{:016X}", self.seed));
        self.rebuild_map();
    }

    pub fn sync_sliders_to_seed(&mut self) {
        let w = self.water_slider.value() as u8;
        let t = self.trees_slider.value() as u8;
        let (_, _, raw) = Self::unpack_seed(self.seed);
        self.seed = Self::pack_seed(w, t, raw);
        self.seed_input.set_text(&format!("{:016X}", self.seed));
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

pub struct LoadCityState {
    pub saves: Vec<save::SaveEntry>,
    pub selected: usize,
}

use std::sync::{Arc, RwLock};
use std::sync::mpsc::Sender;
use crate::core::engine::EngineCommand;

// ── AppState ──────────────────────────────────────────────────────────────────

use crate::app::screens::{Screen, ScreenTransition, AppContext, StartScreen};

pub struct AppState {
    pub screens: Vec<Box<dyn Screen>>,
    pub engine: Arc<RwLock<crate::core::engine::SimulationEngine>>,
    pub cmd_tx: Option<Sender<EngineCommand>>,
    pub running: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            screens: vec![Box::new(StartScreen::new())],
            engine: Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
                Map::new(128, 128),
                SimState::default(),
            ))),
            cmd_tx: None,
            running: true,
        }
    }

    pub fn on_tick(&mut self) {
        let mut running = self.running;
        {
            let context = AppContext {
                engine: &self.engine,
                cmd_tx: &self.cmd_tx,
                running: &mut running,
            };
            if let Some(screen) = self.screens.last_mut() {
                screen.on_tick(context);
            }
        }
        self.running = running;
    }

    pub fn on_event(&mut self, event: &crossterm::event::Event) -> bool {
        let mut running = self.running;
        let transition = {
            let context = AppContext {
                engine: &self.engine,
                cmd_tx: &self.cmd_tx,
                running: &mut running,
            };
            if let Some(screen) = self.screens.last_mut() {
                screen.on_event(event, context)
            } else {
                None
            }
        };
        self.running = running;

        let handled = transition.is_some();
        if let Some(t) = transition {
            match t {
                ScreenTransition::Push(s) => self.screens.push(s),
                ScreenTransition::Pop => { self.screens.pop(); }
                ScreenTransition::Replace(s) => { self.screens.pop(); self.screens.push(s); }
                ScreenTransition::Quit => self.running = false,
            }
        }
        
        if self.screens.is_empty() {
            self.running = false;
        }

        handled
    }

    pub fn on_action(&mut self, action: Action) {
        if matches!(action, Action::Quit) {
            self.running = false;
            return;
        }

        let mut running = self.running;
        let transition = {
            let context = AppContext {
                engine: &self.engine,
                cmd_tx: &self.cmd_tx,
                running: &mut running,
            };
            if let Some(screen) = self.screens.last_mut() {
                screen.on_action(action, context)
            } else {
                None
            }
        };
        self.running = running;

        if let Some(t) = transition {
            match t {
                ScreenTransition::Push(s) => self.screens.push(s),
                ScreenTransition::Pop => { self.screens.pop(); }
                ScreenTransition::Replace(s) => { self.screens.pop(); self.screens.push(s); }
                ScreenTransition::Quit => self.running = false,
            }
        }
        
        if self.screens.is_empty() {
            self.running = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_packing() {
        let water_pct = 40;
        let trees_pct = 60;
        let raw_seed = 0x123456789ABC;

        let packed = NewCityState::pack_seed(water_pct, trees_pct, raw_seed);
        let (w, t, r) = NewCityState::unpack_seed(packed);

        assert_eq!(w, water_pct);
        assert_eq!(t, trees_pct);
        assert_eq!(r, raw_seed);
    }
}
