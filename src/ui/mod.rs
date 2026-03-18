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
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Clear};

    // ── Fixed bars: menu (row 0) + status bar (row 1) ────────────────────────
    let menu_area   = Rect::new(area.x, area.y,     area.width, 1);
    let status_area = Rect::new(area.x, area.y + 1, area.width, 1);
    // Desktop: everything below the two fixed bars
    let desktop = Rect::new(area.x, area.y + 2, area.width, area.height.saturating_sub(2));

    // ── Clamp a FloatingWindow, return its outer Rect ────────────────────────
    // Windows may be dragged partially off-screen. The only hard rules are:
    //   • Title bar row must stay within the desktop (so it can be grabbed).
    //   • At least 4 columns of the title bar must remain visible on screen.
    //   • The rendered rect is clipped to the buffer boundary so nothing panics.
    let clamp_win = |win: &mut crate::app::FloatingWindow| -> Rect {
        let h = win.height.max(4);
        let w = win.width.max(6);
        // Sentinel: u16::MAX means "not yet placed" → right-align fully visible.
        if win.x == u16::MAX {
            win.x = desktop.x + desktop.width.saturating_sub(w);
        }
        // x: keep at least 4 columns of the title bar visible; window stays within
        // desktop width so it never writes past the buffer's right edge.
        let min_x = (desktop.x + 4).saturating_sub(w);
        let max_x = desktop.x + desktop.width.saturating_sub(4).min(desktop.width.saturating_sub(w));
        let x = win.x.clamp(min_x, max_x);
        // y: only the title bar row needs to be on screen (free vertical movement).
        let y = win.y.clamp(desktop.y, desktop.y + desktop.height.saturating_sub(1));
        win.x = x; win.y = y;
        // Clip both dimensions to the buffer boundary so nothing writes outside.
        // Content layout uses win.width/win.height (fixed), not these clipped values.
        let right   = (x + w).min(desktop.x + desktop.width);
        let actual_w = right.saturating_sub(x).max(1);
        let bottom  = (y + h).min(desktop.y + desktop.height);
        let actual_h = bottom.saturating_sub(y).max(1);
        Rect::new(x, y, actual_w, actual_h)
    };

    let map_rect     = clamp_win(&mut screen.map_win);
    let panel_rect   = clamp_win(&mut screen.panel_win);
    let budget_rect  = clamp_win(&mut screen.budget_win);
    let inspect_rect = clamp_win(&mut screen.inspect_win);

    // Inner content areas (subtract 1-cell border on all sides)
    let inner = |r: Rect| Rect::new(
        r.x + 1, r.y + 1,
        r.width.saturating_sub(2),
        r.height.saturating_sub(2),
    );
    let map_inner     = inner(map_rect);
    let panel_inner   = inner(panel_rect);
    let budget_inner  = inner(budget_rect);
    let inspect_inner = inner(inspect_rect);

    // ── Update camera viewport ───────────────────────────────────────────────
    // Use the *exposed* (panel-free) portion of the map window so that
    // scroll_to_cursor keeps the cursor in the actually visible area.
    let exposed_map_w = if panel_rect.x > map_inner.x {
        (panel_rect.x - map_inner.x) as usize
    } else {
        map_inner.width as usize
    }
    .min(map_inner.width as usize)
    .max(1);
    screen.camera.view_w = exposed_map_w;
    screen.camera.view_h = map_inner.height as usize;

    // ── Update click areas ────────────────────────────────────────────────────
    screen.ui_areas.menu_bar_y = menu_area.y;
    screen.ui_areas.map        = to_click_area(map_inner);
    // minimap + toolbar_buttons updated when panel content is rendered below

    // ── Background ────────────────────────────────────────────────────────────
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(6, 6, 14))),
        area,
    );

    // ── Status bar ────────────────────────────────────────────────────────────
    {
        let engine = app.engine.read().unwrap();
        let pause_area = game::statusbar::render_statusbar(
            status_area,
            frame.buffer_mut(),
            &engine.sim,
            screen.paused,
            screen.status_message(),
        );
        screen.ui_areas.pause_btn = pause_area;
    }

    // ── Map window ────────────────────────────────────────────────────────────
    frame.render_widget(Clear, map_rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" MAP ")
            .title_style(Style::default().fg(Color::Rgb(150, 180, 255)))
            .border_style(Style::default().fg(Color::Rgb(50, 70, 110)))
            .style(Style::default().bg(Color::Rgb(8, 12, 8))),
        map_rect,
    );

    if map_inner.width > 0 && map_inner.height > 0 {
        use crate::core::tool::Tool;
        use crate::ui::game::map_view::PreviewKind;
        let engine = app.engine.read().unwrap();

        let footprint_tiles: Vec<(usize, usize)> =
            if screen.rect_drag.is_none() && screen.line_drag.is_none()
                && Tool::uses_footprint_preview(screen.current_tool)
            {
                let (fw, fh) = screen.current_tool.footprint();
                let (cx, cy) = (screen.camera.cursor_x, screen.camera.cursor_y);
                let ax = cx.saturating_sub(fw / 2).min(engine.map.width.saturating_sub(fw));
                let ay = cy.saturating_sub(fh / 2).min(engine.map.height.saturating_sub(fh));
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
                (screen.rect_preview(), PreviewKind::Rect(screen.current_tool))
            } else if let Some(ref _d) = screen.line_drag {
                (screen.line_preview(), PreviewKind::Line(screen.current_tool))
            } else if !footprint_tiles.is_empty() {
                (&footprint_tiles, PreviewKind::Footprint(screen.current_tool, footprint_all_valid))
            } else {
                (&[], PreviewKind::None)
            };
        let preview_kind = if let Some(ref d) = screen.rect_drag { PreviewKind::Rect(d.tool) }
                           else if let Some(ref d) = screen.line_drag { PreviewKind::Line(d.tool) }
                           else { preview_kind };

        frame.render_widget(
            game::map_view::MapView {
                map: &engine.map,
                camera: &screen.camera,
                line_preview: preview_tiles,
                preview_kind,
                overlay_mode: screen.overlay_mode,
            },
            map_inner,
        );
    }

    // ── Panel window ──────────────────────────────────────────────────────────
    frame.render_widget(Clear, panel_rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" TOOLS ")
            .title_style(Style::default().fg(Color::Rgb(150, 180, 255)))
            .border_style(Style::default().fg(Color::Rgb(50, 70, 110)))
            .style(Style::default().bg(Color::Rgb(12, 12, 28))),
        panel_rect,
    );

    if panel_inner.width > 0 {
        // Layout uses the FULL window height so sub-widget sizes never change
        // as the window is dragged toward the screen edge.  Each sub-area is
        // then intersected with the terminal rect before rendering so nothing
        // writes outside the buffer.
        let full_inner_h = screen.panel_win.height.saturating_sub(2).max(4);
        let full_inner = Rect::new(
            panel_inner.x, panel_inner.y,
            panel_inner.width, full_inner_h,
        );

        let ph = full_inner.height;
        let toolbar_h = game::toolbar::toolbar_height();
        let minimap_h = (ph / 5).max(7).min(ph.saturating_sub(2));
        let info_h    = (ph / 6).max(5).min(ph.saturating_sub(1 + minimap_h));
        let panel_vert = Layout::vertical([
            Constraint::Length(toolbar_h),
            Constraint::Length(minimap_h),
            Constraint::Length(info_h),
        ])
        .split(full_inner);

        // Clip each sub-area to the buffer so off-screen parts just disappear.
        let toolbar_area = panel_vert[0].intersection(area);
        let minimap_area = panel_vert[1].intersection(area);
        let info_area    = panel_vert[2].intersection(area);

        screen.ui_areas.minimap = to_click_area(minimap_area);

        let engine = app.engine.read().unwrap();

        if toolbar_area.width > 0 && toolbar_area.height > 0 {
            game::toolbar::render_toolbar(
                toolbar_area,
                frame.buffer_mut(),
                screen.current_tool,
                &mut screen.ui_areas.toolbar_buttons,
            );
        }
        if minimap_area.width > 0 && minimap_area.height > 0 {
            frame.render_widget(
                game::minimap::MiniMap { map: &engine.map, camera: &screen.camera },
                minimap_area,
            );
        }

        let cx = screen.camera.cursor_x.min(engine.map.width.saturating_sub(1));
        let cy = screen.camera.cursor_y.min(engine.map.height.saturating_sub(1));
        let tile = if engine.map.width > 0 && engine.map.height > 0 {
            engine.map.get(cx, cy)
        } else { crate::core::map::Tile::Grass };
        let tile_overlay = if engine.map.width > 0 && engine.map.height > 0 {
            engine.map.get_overlay(cx, cy)
        } else { crate::core::map::TileOverlay::default() };

        if info_area.width > 0 && info_area.height > 0 {
            frame.render_widget(
                game::infopanel::InfoPanel {
                    tile,
                    overlay: tile_overlay,
                    x: cx, y: cy,
                    current_tool: screen.current_tool,
                    demand_res:  engine.sim.demand_res,
                    demand_comm: engine.sim.demand_comm,
                    demand_ind:  engine.sim.demand_ind,
                    demand_history_res:  &engine.sim.demand_history_res,
                    demand_history_comm: &engine.sim.demand_history_comm,
                    demand_history_ind:  &engine.sim.demand_history_ind,
                },
                info_area,
            );
        }
        // engine lock released here
    }

    // ── Budget window (when open) ─────────────────────────────────────────────
    if screen.is_budget_open {
        let mock_app = crate::app::AppState {
            screens: Vec::new(),
            engine: app.engine.clone(),
            cmd_tx: app.cmd_tx.clone(),
            running: app.running,
        };
        frame.render_widget(Clear, budget_rect);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(" Budget & Taxes ")
                .title_style(Style::default().fg(Color::Yellow))
                .border_style(Style::default().fg(Color::Rgb(180, 160, 40)))
                .style(Style::default().bg(Color::Rgb(20, 20, 30))),
            budget_rect,
        );
        game::budget::render_budget_content(frame.buffer_mut(), budget_inner, &mock_app);
    }

    // ── Inspect window (when open) ────────────────────────────────────────────
    if let Some(inspect_pos) = screen.inspect_pos {
        let engine = app.engine.read().unwrap();
        if inspect_pos.0 < engine.map.width && inspect_pos.1 < engine.map.height {
            let title = format!(" Inspect ({},{}) ", inspect_pos.0, inspect_pos.1);
            frame.render_widget(Clear, inspect_rect);
            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title.as_str())
                    .title_style(Style::default().fg(Color::Cyan))
                    .border_style(Style::default().fg(Color::Rgb(30, 140, 160)))
                    .style(Style::default().bg(Color::Rgb(10, 18, 28))),
                inspect_rect,
            );
            game::inspect_popup::render_inspect_content(
                frame.buffer_mut(), inspect_inner, inspect_pos, &engine.map,
            );
        }
    }

    // ── Menu bar — always on top ──────────────────────────────────────────────
    {
        use tui_menu::Menu;
        use ratatui::style::{Color, Style};
        let buf = frame.buffer_mut();
        for x in menu_area.x..menu_area.x + menu_area.width {
            if let Some(cell) = buf.cell_mut((x, menu_area.y)) {
                cell.set_bg(Color::Rgb(55, 55, 120));
            }
        }
        Menu::<crate::app::screens::MenuAction>::new()
            .default_style(Style::default().fg(Color::White).bg(Color::Rgb(55, 55, 120)))
            .highlight(Style::default().fg(Color::Black).bg(Color::Rgb(160, 160, 255)))
            .dropdown_width(24)
            .dropdown_style(Style::default().bg(Color::Rgb(55, 55, 120)))
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
