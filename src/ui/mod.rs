pub mod frontends;
pub mod game;
pub mod painter;
pub mod pixel;
pub mod runtime;
pub mod screens;
pub mod theme;
pub mod view;

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

#[cfg(test)]
use ratatui::backend::TestBackend;

use crate::{
    app::{
        config::FrontendKind,
        screens::{
            AppContext, InGameScreen, LlmSetupScreen, LoadCityScreen, NewCityScreen, SettingsScreen,
            StartScreen, ThemeSettingsScreen,
        },
        AppState,
    },
};

pub trait Renderer {
    fn render(&mut self, app: &mut AppState) -> io::Result<()>;
}

pub struct TerminalRenderer {
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
    frontend: FrontendKind,
}

#[cfg(test)]
pub struct OffscreenRenderer {
    terminal: Terminal<TestBackend>,
    frontend: FrontendKind,
}

fn apply_render_style(frontend: FrontendKind) {
    let style = if frontend.is_pixel() {
        theme::RenderStyle::PixelDos
    } else {
        theme::RenderStyle::TerminalAscii
    };
    theme::set_render_style(style);
}

pub(crate) fn render_app_frame(frame: &mut ratatui::Frame<'_>, frontend: FrontendKind, app: &mut AppState) {
    apply_render_style(frontend);
    let context = AppContext {
        engine: &app.engine,
        cmd_tx: &app.cmd_tx,
        textgen: &app.textgen,
    };

    if let Some(screen) = app.screens.last_mut() {
        if let Some(screen) = screen.as_any_mut().downcast_mut::<StartScreen>() {
            let view = screen.view_model();
            screens::start::render_start(frame, frame.area(), &view, &mut screen.state);
        } else if let Some(screen) = screen.as_any_mut().downcast_mut::<LoadCityScreen>() {
            let view = screen.view_model();
            screens::load_city::render_load_city(frame, frame.area(), &view, &mut screen.state);
        } else if let Some(screen) = screen.as_any_mut().downcast_mut::<NewCityScreen>() {
            let view = screen.view_model();
            screens::new_city::render_new_city(frame, frame.area(), &view, &mut screen.state);
        } else if let Some(screen) = screen.as_any_mut().downcast_mut::<SettingsScreen>() {
            let view = screen.view_model(context);
            screens::settings::render_settings(frame, frame.area(), &view, &mut screen.state);
        } else if let Some(screen) = screen.as_any_mut().downcast_mut::<LlmSetupScreen>() {
            let view = screen.view_model(context);
            screens::llm_setup::render_llm_setup(frame, frame.area(), &view, &mut screen.state);
        } else if let Some(screen) = screen.as_any_mut().downcast_mut::<InGameScreen>() {
            let engine = context.engine.read().unwrap();
            let view = screen.view_model(&engine.sim, &engine.map);
            let area = frame.area();
            let total_cols = area.width;
            let total_rows = area.height;
            let desktop_layout = screen
                .desktop
                .layout(crate::ui::runtime::UiRect::new(area.x, area.y, total_cols, total_rows));
            let map_inner = desktop_layout.window(crate::app::WindowId::Map).inner;
            let panel_outer_x = desktop_layout.window(crate::app::WindowId::Panel).outer.x;
            let map_layout = game::map_view::layout_map_chrome(
                ratatui::layout::Rect::new(map_inner.x, map_inner.y, map_inner.width, map_inner.height),
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
            let mut painter = frontends::terminal::ingame::TerminalPainter::new(frame, area);
            crate::ui::painter::orchestrate_ingame(&mut painter, &view, screen, layout);
        } else if let Some(screen) = screen.as_any_mut().downcast_mut::<ThemeSettingsScreen>() {
            let view = screen.view_model();
            screens::theme_settings::render_theme_settings(frame, frame.area(), &view, &mut screen.state);
        }
    }
}

impl TerminalRenderer {
    pub fn new(frontend: FrontendKind) -> io::Result<Self> {
        log::info!("[ui] Initializing TerminalRenderer ({})", frontend.label());
        apply_render_style(frontend);
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend).map_err(|e| {
            log::error!("[ui] Terminal init failed: {e}");
            e
        })?;
        Ok(Self { terminal, frontend })
    }
}

#[cfg(test)]
impl OffscreenRenderer {
    pub fn new(frontend: FrontendKind, cols: u16, rows: u16) -> io::Result<Self> {
        apply_render_style(frontend);
        let terminal = new_offscreen_terminal(cols, rows);
        Ok(Self { terminal, frontend })
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        // Recreate the off-screen terminal on resize instead of mutating the existing
        // TestBackend in place. This keeps the ratatui frame area and backing buffer fully
        // synchronized during live window resizes/maximize events in the pixel frontend.
        self.terminal = new_offscreen_terminal(cols, rows);
        Ok(())
    }

    pub fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.terminal.backend().buffer()
    }
}

impl Renderer for TerminalRenderer {
    fn render(&mut self, app: &mut AppState) -> io::Result<()> {
        log::trace!("[ui] Rendering frame");
        self.terminal
            .draw(|frame| render_app_frame(frame, self.frontend, app))?;
        Ok(())
    }
}

#[cfg(test)]
impl Renderer for OffscreenRenderer {
    fn render(&mut self, app: &mut AppState) -> io::Result<()> {
        self.terminal
            .draw(|frame| render_app_frame(frame, self.frontend, app))
            .expect("test backend terminal should render");
        Ok(())
    }
}

#[cfg(test)]
fn new_offscreen_terminal(cols: u16, rows: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(cols.max(1), rows.max(1)))
        .expect("test backend terminal should initialize")
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{config::FrontendKind, screens::InGameScreen, AppState},
        ui::{render_app_frame, Renderer},
    };
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_start_screen_to_buffer() {
        let mut app = AppState::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render_app_frame(frame, FrontendKind::TerminalAscii, &mut app))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut text_found = false;
        for y in 0..24 {
            for x in 0..80 {
                let cell = &buffer[(x, y)];
                if !cell.symbol().is_empty() && cell.symbol() != " " {
                    text_found = true;
                }
            }
        }

        assert!(
            text_found,
            "The buffer is empty. The rendering logic did not write any content."
        );
    }

    #[test]
    fn test_render_ingame_screen_at_160x50() {
        let mut app = AppState::new();
        app.screens = vec![Box::new(InGameScreen::new())];
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render_app_frame(frame, FrontendKind::Pixel, &mut app))
            .unwrap();
    }

    #[test]
    fn test_render_start_screen_at_160x50() {
        let mut app = AppState::new();
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render_app_frame(frame, FrontendKind::Pixel, &mut app))
            .unwrap();
    }

    #[test]
    fn test_offscreen_renderer_survives_repeated_resizes() {
        let mut app = AppState::new();
        app.screens = vec![Box::new(InGameScreen::new())];
        let mut renderer = super::OffscreenRenderer::new(FrontendKind::Pixel, 80, 24).unwrap();

        for (cols, rows) in [(120, 40), (160, 50), (121, 38), (160, 50), (80, 24)] {
            renderer.resize(cols, rows).unwrap();
            renderer.render(&mut app).unwrap();
            assert_eq!(renderer.buffer().area.width, cols);
            assert_eq!(renderer.buffer().area.height, rows);
        }
    }
}
