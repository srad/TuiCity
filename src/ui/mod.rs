pub mod frontends;
pub mod game;
pub mod painter;
pub mod runtime;
pub mod screens;
pub mod theme;
pub mod view;

use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::{
    app::{
        screens::{
            AppContext, InGameScreen, LlmSetupScreen, LoadCityScreen, NewCityScreen,
            SettingsScreen, StartScreen,
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
        log::info!("[ui] Initializing TerminalRenderer");
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend).map_err(|e| {
            log::error!("[ui] Terminal init failed: {e}");
            e
        })?;
        Ok(Self { terminal })
    }
}

impl Renderer for TerminalRenderer {
    fn render(&mut self, app: &mut AppState) -> io::Result<()> {
        log::trace!("[ui] Rendering frame");
        self.terminal.draw(|frame| {
            let context = AppContext {
                engine: &app.engine,
                cmd_tx: &app.cmd_tx,
                textgen: &app.textgen,
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
                    ScreenView::LlmSetup(view) => {
                        let screen = screen
                            .as_any_mut()
                            .downcast_mut::<LlmSetupScreen>()
                            .expect("active llm-setup screen should downcast");
                        screens::llm_setup::render_llm_setup(
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
                        let area = frame.area();
                        let total_cols = area.width;
                        let total_rows = area.height;
                        let desktop_layout = screen.desktop.layout(
                            crate::ui::runtime::UiRect::new(area.x, area.y, total_cols, total_rows),
                        );
                        // Compute view dimensions for terminal (col_scale=2 because chars are double-wide)
                        let map_inner = desktop_layout.window(crate::app::WindowId::Map).inner;
                        let panel_outer_x =
                            desktop_layout.window(crate::app::WindowId::Panel).outer.x;
                        let map_layout = game::map_view::layout_map_chrome(
                            ratatui::layout::Rect::new(
                                map_inner.x,
                                map_inner.y,
                                map_inner.width,
                                map_inner.height,
                            ),
                            view.map.width,
                            view.map.height,
                            screen.camera.offset_x.max(0) as usize,
                            screen.camera.offset_y.max(0) as usize,
                        );
                        let exposed_map_w = if panel_outer_x > map_layout.viewport.x {
                            (panel_outer_x - map_layout.viewport.x) as usize
                        } else {
                            map_layout.viewport.width as usize
                        }
                        .min(map_layout.viewport.width as usize)
                        .max(1);
                        let layout = crate::ui::painter::FrameLayout {
                            desktop_layout,
                            view_w: (exposed_map_w / 2).max(1),
                            view_h: map_layout.view_tiles_h.max(1),
                            col_scale: 2,
                        };
                        let mut painter =
                            frontends::terminal::ingame::TerminalPainter::new(frame, area);
                        crate::ui::painter::orchestrate_ingame(&mut painter, &view, screen, layout);
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
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::screens::StartScreen;
    use crate::app::AppState;
    use crate::ui::screens::start::render_start;
    use crate::ui::view::ScreenView;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_start_screen_to_buffer() {
        let mut app = AppState::new();
        // Use TestBackend which doesn't require a real terminal.
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let context = crate::app::screens::AppContext {
                    engine: &app.engine,
                    cmd_tx: &app.cmd_tx,
                    textgen: &app.textgen,
                };

                if let Some(screen) = app.screens.last_mut() {
                    let view = screen.build_view(context);
                    if let ScreenView::Start(view) = view {
                        let screen = screen
                            .as_any_mut()
                            .downcast_mut::<StartScreen>()
                            .expect("active start screen should downcast");
                        render_start(frame, frame.area(), &view, &mut screen.state);
                    }
                }
            })
            .unwrap();

        // Now inspect the buffer
        let buffer = terminal.backend().buffer();

        // Check if the buffer is actually being filled
        let mut text_found = false;
        for y in 0..24 {
            for x in 0..80 {
                if !buffer.get(x, y).symbol().is_empty() && buffer.get(x, y).symbol() != " " {
                    text_found = true;
                }
            }
        }

        assert!(
            text_found,
            "The buffer is empty. The rendering logic did not write any content."
        );
    }
}
