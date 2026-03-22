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
            let context = AppContext {
                engine: &app.engine,
                cmd_tx: &app.cmd_tx,
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
                        let area = frame.area();
                        let total_cols = area.width;
                        let total_rows = area.height;
                        let desktop_layout = screen
                            .desktop
                            .layout(crate::ui::runtime::UiRect::new(area.x, area.y, total_cols, total_rows));
                        // Compute view dimensions for terminal (col_scale=2 because chars are double-wide)
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
                        }.min(map_layout.viewport.width as usize).max(1);
                        let layout = crate::ui::painter::FrameLayout {
                            desktop_layout,
                            view_w: (exposed_map_w / 2).max(1),
                            view_h: map_layout.view_tiles_h.max(1),
                            col_scale: 2,
                        };
                        let mut painter = frontends::terminal::ingame::TerminalPainter::new(frame, area);
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
