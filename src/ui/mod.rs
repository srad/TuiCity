pub mod frontends;
pub mod game;
pub mod runtime;
pub mod screens;
pub mod theme;
pub mod view;

use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::{
    app::{
        screens::{
            AppContext, InGameScreen, LoadCityScreen, NewCityScreen, SettingsScreen, StartScreen,
        },
        AppState,
    },
    ui::view::ScreenView,
};

pub trait Renderer {
    fn render(&mut self, app: &mut AppState) -> io::Result<()>;
}

pub struct TerminalRenderer {
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalRenderer {
    pub fn new() -> io::Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Renderer for TerminalRenderer {
    fn render(&mut self, app: &mut AppState) -> io::Result<()> {
        self.terminal.draw(|frame| {
            let mut running = app.running;
            let context = AppContext {
                engine: &app.engine,
                cmd_tx: &app.cmd_tx,
                running: &mut running,
            };

            if let Some(screen) = app.screens.last_mut() {
                let view = screen.build_view(context);
                match view {
                    ScreenView::Start(view) => {
                        let screen = screen
                            .as_any_mut()
                            .downcast_mut::<StartScreen>()
                            .expect("active start screen should downcast");
                        screens::start::render_start(frame, frame.area(), &view, &mut screen.state);
                    }
                    ScreenView::LoadCity(view) => {
                        let screen = screen
                            .as_any_mut()
                            .downcast_mut::<LoadCityScreen>()
                            .expect("active load-city screen should downcast");
                        screens::load_city::render_load_city(
                            frame,
                            frame.area(),
                            &view,
                            &mut screen.state,
                        );
                    }
                    ScreenView::NewCity(view) => {
                        let screen = screen
                            .as_any_mut()
                            .downcast_mut::<NewCityScreen>()
                            .expect("active new-city screen should downcast");
                        screens::new_city::render_new_city(
                            frame,
                            frame.area(),
                            &view,
                            &mut screen.state,
                        );
                    }
                    ScreenView::Settings(view) => {
                        let screen = screen
                            .as_any_mut()
                            .downcast_mut::<SettingsScreen>()
                            .expect("active settings screen should downcast");
                        screens::settings::render_settings(
                            frame,
                            frame.area(),
                            &view,
                            &mut screen.state,
                        );
                    }
                    ScreenView::InGame(view) => {
                        let screen = screen
                            .as_any_mut()
                            .downcast_mut::<InGameScreen>()
                            .expect("active in-game screen should downcast");
                        frontends::terminal::render_ingame(frame, frame.area(), screen, &view);
                    }
                    ScreenView::ThemeSettings(view) => {
                        let screen = screen
                            .as_any_mut()
                            .downcast_mut::<crate::app::screens::ThemeSettingsScreen>()
                            .expect("active theme-settings screen should downcast");
                        screens::theme_settings::render_theme_settings(
                            frame,
                            frame.area(),
                            &view,
                            &mut screen.state,
                        );
                    }
                }
            }

            app.running = running;
        })?;
        Ok(())
    }
}
