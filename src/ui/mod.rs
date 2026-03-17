pub mod game;
pub mod screens;
pub mod theme;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::StatefulWidget,
    Frame,
    Terminal,
    backend::CrosstermBackend,
};
use std::io;

use crate::app::{AppState, ClickArea, screens::AppContext};

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
            {
                let context = AppContext {
                    engine: &app.engine,
                    cmd_tx: &app.cmd_tx,
                    running: &mut running,
                };
                if let Some(screen) = app.screens.last_mut() {
                    screen.render(frame, context);
                }
            }
            app.running = running;
        })?;
        Ok(())
    }
}

pub fn render_game_v2(frame: &mut Frame, area: Rect, app: &AppState, screen: &mut crate::app::screens::InGameScreen) {
    // Layout: menu bar (1 row) + status bar (1 row) + main area
    let vert = Layout::vertical([
        Constraint::Length(1),  // menu bar
        Constraint::Length(1),  // status bar
        Constraint::Fill(1),    // main area
    ])
    .split(area);

    let menu_area   = vert[0];
    let status_area = vert[1];
    let main_area   = vert[2];

    // Main area: map (fill) + right panel (22 cols)
    let horiz = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(22),
    ])
    .split(main_area);

    let map_area = horiz[0];
    let panel_area = horiz[1];

    // Right panel: toolbar (fill) + minimap + info panel
    let minimap_h = (panel_area.height / 5).max(7);
    let info_h = (panel_area.height / 6).max(5);

    let panel_vert = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(minimap_h),
        Constraint::Length(info_h),
    ])
    .split(panel_area);

    let toolbar_area = panel_vert[0];
    let minimap_area = panel_vert[1];
    let info_area = panel_vert[2];

    // Update camera viewport dimensions
    screen.camera.view_w = map_area.width as usize;
    screen.camera.view_h = map_area.height as usize;

    // Update click areas
    screen.ui_areas.menu_bar_y = menu_area.y;
    screen.ui_areas.map = to_click_area(map_area);
    screen.ui_areas.minimap = to_click_area(minimap_area);

    // Render status bar → get pause button area
    let pause_area = {
        let engine = app.engine.read().unwrap();
        game::statusbar::render_statusbar(
            status_area,
            frame.buffer_mut(),
            &engine.sim,
            screen.paused,
            screen.message.as_deref(),
        )
    };
    screen.ui_areas.pause_btn = pause_area;

    // Render map view
    use crate::core::tool::Tool;
    use crate::ui::game::map_view::PreviewKind;

    let engine = app.engine.read().unwrap();

    // Pre-compute footprint tiles (owned) so the slice lifetime covers the render call.
    let footprint_tiles: Vec<(usize, usize)> =
        if screen.rect_drag.is_none() && screen.line_drag.is_none()
            && Tool::uses_footprint_preview(screen.current_tool)
        {
            let (fw, fh) = screen.current_tool.footprint();
            let (cx, cy) = (screen.camera.cursor_x, screen.camera.cursor_y);
            let ax = cx.saturating_sub(fw / 2)
                       .min(engine.map.width.saturating_sub(fw));
            let ay = cy.saturating_sub(fh / 2)
                       .min(engine.map.height.saturating_sub(fh));
            (0..fh).flat_map(|dy| (0..fw).map(move |dx| (ax + dx, ay + dy))).collect()
        } else {
            Vec::new()
        };
    let footprint_all_valid = footprint_tiles.iter().all(|&(x, y)| {
        x < engine.map.width && y < engine.map.height
            && screen.current_tool.can_place(engine.map.get(x, y))
    });

    let (preview_tiles, preview_kind): (&[(usize, usize)], PreviewKind) =
        if let Some(ref _d) = screen.rect_drag {
            (screen.rect_preview(), PreviewKind::Rect(screen.current_tool)) // Note: tool comes from drag in real impl
        } else if let Some(ref _d) = screen.line_drag {
            (screen.line_preview(), PreviewKind::Line(screen.current_tool))
        } else if !footprint_tiles.is_empty() {
            (&footprint_tiles, PreviewKind::Footprint(screen.current_tool, footprint_all_valid))
        } else {
            (&[], PreviewKind::None)
        };
    
    // Quick fix for tool from drag:
    let preview_kind = if let Some(ref d) = screen.rect_drag { PreviewKind::Rect(d.tool) }
                       else if let Some(ref d) = screen.line_drag { PreviewKind::Line(d.tool) }
                       else { preview_kind };

    frame.render_widget(
        game::map_view::MapView {
            map: &engine.map,
            camera: &screen.camera,
            line_preview: preview_tiles,
            preview_kind,
        },
        map_area,
    );

    // Render toolbar → collect button areas
    game::toolbar::render_toolbar(
        toolbar_area,
        frame.buffer_mut(),
        screen.current_tool,
        &mut screen.ui_areas.toolbar_buttons,
    );

    // Render minimap
    frame.render_widget(
        game::minimap::MiniMap {
            map: &engine.map,
            camera: &screen.camera,
        },
        minimap_area,
    );

    // Render info panel
    let cx = screen.camera.cursor_x.min(engine.map.width.saturating_sub(1));
    let cy = screen.camera.cursor_y.min(engine.map.height.saturating_sub(1));
    let tile = if engine.map.width > 0 && engine.map.height > 0 {
        engine.map.get(cx, cy)
    } else {
        crate::core::map::Tile::Grass
    };
    let overlay = if engine.map.width > 0 && engine.map.height > 0 {
        engine.map.get_overlay(cx, cy)
    } else {
        crate::core::map::TileOverlay::default()
    };

    frame.render_widget(
        game::infopanel::InfoPanel {
            tile,
            overlay,
            x: cx,
            y: cy,
            current_tool: screen.current_tool,
            demand_res: engine.sim.demand_res,
            demand_comm: engine.sim.demand_comm,
            demand_ind: engine.sim.demand_ind,
            demand_history_res:  &engine.sim.demand_history_res,
            demand_history_comm: &engine.sim.demand_history_comm,
            demand_history_ind:  &engine.sim.demand_history_ind,
        },
        info_area,
    );
    
    // Explicitly drop the lock before rendering the budget so we don't accidentally hold it across UI bounds
    drop(engine);
    
    let mock_app = crate::app::AppState {
        screens: Vec::new(),
        engine: app.engine.clone(),
        cmd_tx: app.cmd_tx.clone(),
        running: app.running,
    };

    if screen.is_budget_open {
        game::budget::render_budget_v2(frame.buffer_mut(), area, &mock_app, screen);
    }

    // Render the dropdown menubar LAST so it appears on top of everything.
    // The bar occupies menu_area (row 0); dropdowns overlay the content below.
    {
        use tui_menu::Menu;
        use ratatui::style::{Color, Style};
        let buf = frame.buffer_mut();
        Menu::<crate::app::screens::MenuAction>::new()
            .default_style(
                Style::default().fg(Color::White).bg(Color::Rgb(20, 20, 45)),
            )
            .highlight(
                Style::default().fg(Color::Black).bg(Color::Rgb(160, 160, 255)),
            )
            .dropdown_width(24)
            .dropdown_style(Style::default().bg(Color::Rgb(20, 20, 45)))
            .render(menu_area, buf, &mut screen.menu);
    }
}

pub fn to_click_area(r: Rect) -> ClickArea {
    ClickArea {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }
}
