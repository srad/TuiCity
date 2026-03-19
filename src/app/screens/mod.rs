mod ingame;
mod ingame_budget;
mod ingame_interaction;
mod ingame_menu;
mod load_city;
mod new_city;
mod settings;
mod start;
mod theme_settings;

use std::any::Any;
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use crate::{
    app::input::Action,
    core::engine::{EngineCommand, SimulationEngine},
    ui::view::ScreenView,
};

pub use ingame::InGameScreen;
pub use ingame_budget::BudgetFocus;
pub use ingame_menu::{menu_rows, MENU_TITLES};
pub use load_city::{LoadCityScreen, LoadCityState};
pub use new_city::{NewCityField, NewCityScreen, NewCityState};
pub use settings::{SettingsScreen, SettingsState};
pub use start::{StartScreen, StartState};
pub use theme_settings::{ThemeSettingsScreen, ThemeSettingsState};

pub enum ScreenTransition {
    Push(Box<dyn Screen>),
    Pop,
    Replace(Box<dyn Screen>),
    Quit,
}

pub struct AppContext<'a> {
    pub engine: &'a Arc<RwLock<SimulationEngine>>,
    pub cmd_tx: &'a Option<Sender<EngineCommand>>,
    #[allow(dead_code)]
    pub running: &'a mut bool,
}

pub trait Screen {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn on_event(
        &mut self,
        _event: &crossterm::event::Event,
        _context: AppContext,
    ) -> Option<ScreenTransition> {
        None
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition>;

    fn on_tick(&mut self, _context: AppContext) {}

    fn build_view(&self, context: AppContext<'_>) -> ScreenView;
}
