pub mod camera;
pub mod config;
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

use crate::app::screens::{AppContext, InGameScreen, Screen, ScreenTransition, StartScreen};
use crate::audio::{MusicCue, MusicManager};
use crate::core::{engine::EngineCommand, map::Map, sim::SimState, tool::Tool};
pub use crate::ui::runtime::{ClickArea, DesktopState, MapUiAreas, UiAreas, WindowId};

pub struct AppState {
    pub screens: Vec<Box<dyn Screen>>,
    pub engine: Arc<RwLock<crate::core::engine::SimulationEngine>>,
    pub cmd_tx: Option<Sender<EngineCommand>>,
    pub running: bool,
    pub music: MusicManager,
    pub textgen: crate::textgen::TextGenService,
}

impl AppState {
    pub fn new() -> Self {
        config::apply_user_config();
        let model_dir = crate::textgen::default_model_dir();
        let model_present = crate::textgen::download::model_files_present(&model_dir);
        if let Err(error) = config::persist_default_llm_preference_if_model_present(model_present) {
            log::error!("[config] failed to persist default LLM preference: {error}");
        }
        let textgen = crate::textgen::TextGenService::start(model_dir);
        let mut app = Self {
            screens: vec![Box::new(StartScreen::new())],
            engine: Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
                Map::new(128, 128),
                SimState::default(),
            ))),
            cmd_tx: None,
            running: true,
            music: MusicManager::new(),
            textgen,
        };
        app.sync_music();
        app
    }

    pub fn on_tick(&mut self) {
        let transition = {
            let context = AppContext {
                engine: &self.engine,
                cmd_tx: &self.cmd_tx,
                textgen: &self.textgen,
            };
            if let Some(screen) = self.screens.last_mut() {
                screen.on_tick(context)
            } else {
                None
            }
        };
        if let Some(transition) = transition {
            self.apply_transition(transition);
        }
        self.sync_music();
    }

    pub fn on_event(&mut self, event: &input::UiEvent) -> bool {
        let transition = {
            let context = AppContext {
                engine: &self.engine,
                cmd_tx: &self.cmd_tx,
                textgen: &self.textgen,
            };
            if let Some(screen) = self.screens.last_mut() {
                screen.on_event(event, context)
            } else {
                None
            }
        };
        self.sync_music();

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
        let is_quit = matches!(action, Action::Quit);
        let suppress_quit_fallback = is_quit
            && self
                .screens
                .last_mut()
                .map(|screen| screen.as_any_mut().is::<InGameScreen>())
                .unwrap_or(false);
        let transition = {
            let context = AppContext {
                engine: &self.engine,
                cmd_tx: &self.cmd_tx,
                textgen: &self.textgen,
            };
            if let Some(screen) = self.screens.last_mut() {
                screen.on_action(action, context)
            } else {
                None
            }
        };
        self.sync_music();

        if let Some(transition) = transition {
            self.apply_transition(transition);
        } else if is_quit && !suppress_quit_fallback {
            self.running = false;
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
            ScreenTransition::Quit => {
                self.running = false;
                self.music.stop_all();
            }
            ScreenTransition::ReinitTextGen => {
                let model_dir = crate::textgen::default_model_dir();
                let old = std::mem::replace(
                    &mut self.textgen,
                    crate::textgen::TextGenService::reinitializing(),
                );
                drop(old);
                self.textgen = crate::textgen::TextGenService::start(model_dir);
            }
        }
        self.sync_music();
    }

    fn sync_music(&mut self) {
        if !crate::app::config::is_music_enabled() {
            self.music.sync(MusicCue::None);
            return;
        }
        let cue = if let Some(screen) = self.screens.last_mut() {
            if screen.as_any_mut().is::<StartScreen>() {
                MusicCue::StartTheme
            } else if screen.as_any_mut().is::<InGameScreen>() {
                MusicCue::Gameplay
            } else {
                MusicCue::None
            }
        } else {
            MusicCue::None
        };
        self.music.sync(cue);
    }

    pub fn attach_engine_channel(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.cmd_tx = Some(tx);

        let engine_arc = self.engine.clone();
        std::thread::spawn(move || {
            while let Ok(cmd) = rx.recv() {
                let mut engine = engine_arc.write().unwrap();
                let _ = engine.execute_command(cmd);
                while let Ok(cmd) = rx.try_recv() {
                    let _ = engine.execute_command(cmd);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_action_does_not_immediately_exit_ingame() {
        let mut app = AppState::new();
        app.screens = vec![Box::new(InGameScreen::new())];

        app.on_action(Action::Quit);

        assert!(app.running);
        let screen = app.screens[0]
            .as_any_mut()
            .downcast_mut::<InGameScreen>()
            .expect("ingame screen should downcast");
        assert!(screen.is_confirm_prompt_open());
    }
}
