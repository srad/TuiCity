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
}

impl Default for StartState {
    fn default() -> Self {
        Self { selected: 0 }
    }
}

pub struct NewCityState {
    pub city_name: String,
    pub seed: u64,
    pub seed_input: String,   // editable hex — synced with seed on focus-leave / Enter
    pub water_pct: u8,
    pub trees_pct: u8,
    pub preview_map: Map,
    pub focused_field: NewCityField,
}

impl NewCityState {
    pub fn new() -> Self {
        let seed = rand::random::<u64>();
        let params = gen::GenParams { seed, ..Default::default() };
        let preview_map = gen::generate(&params);
        Self {
            city_name: String::new(),
            seed,
            seed_input: format!("{:016X}", seed),
            water_pct: params.water_pct,
            trees_pct: params.trees_pct,
            preview_map,
            focused_field: NewCityField::CityName,
        }
    }

    /// Randomise seed and rebuild map.
    pub fn regenerate(&mut self) {
        self.seed = rand::random::<u64>();
        self.seed_input = format!("{:016X}", self.seed);
        self.rebuild_map();
    }

    /// Try to parse `seed_input` as a hex (or decimal) u64, then rebuild.
    pub fn apply_seed_input(&mut self) {
        let raw = self.seed_input.trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let parsed = u64::from_str_radix(raw, 16)
            .or_else(|_| self.seed_input.trim().parse::<u64>());
        if let Ok(new_seed) = parsed {
            self.seed = new_seed;
        }
        // Always reformat to canonical hex so the field stays clean
        self.seed_input = format!("{:016X}", self.seed);
        self.rebuild_map();
    }

    pub fn rebuild_map(&mut self) {
        let params = gen::GenParams {
            water_pct: self.water_pct,
            trees_pct: self.trees_pct,
            seed: self.seed,
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
