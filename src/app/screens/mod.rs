mod ingame;
mod ingame_budget;
mod ingame_interaction;
mod ingame_menu;
mod ingame_widgets;
mod load_city;
mod new_city;
mod start;

use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use crate::{
    app::input::Action,
    core::engine::{EngineCommand, SimulationEngine},
};
use ratatui::Frame;

pub use ingame::InGameScreen;
pub use ingame_budget::{BudgetFocus, BudgetUiState};
pub use ingame_menu::InGameMenu;
pub use load_city::{LoadCityScreen, LoadCityState};
pub use new_city::{NewCityField, NewCityScreen, NewCityState};
pub use start::{StartScreen, StartState};

pub enum ScreenTransition {
    Push(Box<dyn Screen>),
    Pop,
    Replace(Box<dyn Screen>),
    Quit,
}

pub struct AppContext<'a> {
    pub engine: &'a Arc<RwLock<SimulationEngine>>,
    pub cmd_tx: &'a Option<Sender<EngineCommand>>,
    pub running: &'a mut bool,
}

pub trait Screen {
    fn on_event(&mut self, _event: &crossterm::event::Event, _context: AppContext) -> Option<ScreenTransition> {
        None
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition>;

    fn on_tick(&mut self, _context: AppContext) {}

    fn render(&mut self, frame: &mut Frame, context: AppContext);
}
