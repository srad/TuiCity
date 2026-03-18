pub mod camera;
pub mod input;
pub mod line_drag;
pub mod rect_drag;
pub mod save;
pub mod screens;

use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};

use camera::Camera;
use input::Action;
use line_drag::LineDrag;
use rect_drag::RectDrag;

use crate::core::{
    engine::EngineCommand,
    map::Map,
    sim::SimState,
    tool::Tool,
};
use crate::app::screens::{AppContext, Screen, ScreenTransition, StartScreen};
pub use crate::ui::runtime::{
    ClickArea, DesktopState, MapUiAreas, UiAreas, UiRect, WindowId,
};

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
        if let Some(transition) = transition {
            self.apply_transition(transition);
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

        if let Some(transition) = transition {
            self.apply_transition(transition);
        }

        if self.screens.is_empty() {
            self.running = false;
        }
    }

    fn apply_transition(&mut self, transition: ScreenTransition) {
        match transition {
            ScreenTransition::Push(screen) => self.screens.push(screen),
            ScreenTransition::Pop => {
                self.screens.pop();
            }
            ScreenTransition::Replace(screen) => {
                self.screens.pop();
                self.screens.push(screen);
            }
            ScreenTransition::Quit => self.running = false,
        }
    }
}
