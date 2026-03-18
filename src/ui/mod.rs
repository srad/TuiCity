pub mod game;
pub mod runtime;
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

use crate::app::{AppState, ClickArea, MapUiAreas, screens::AppContext};
use crate::ui::runtime::{centered_fit_rect, clamp_window_to_desktop};

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
    use ratatui::style::Style;
    use ratatui::widgets::{Block, Borders, Clear};

    let ui = crate::ui::theme::ui_palette();

    let render_close_button = |
        frame: &mut Frame,
        rect: Rect,
        state: &mut rat_widget::button::ButtonState,
    | {
        if rect.width < 5 || rect.height == 0 {
            return;
        }
        let button_area = Rect::new(rect.x + rect.width.saturating_sub(5), rect.y, 5, 1);
        let button = rat_widget::button::Button::new("[X]")
            .styles(rat_widget::button::ButtonStyle {
                style: Style::default().fg(ui.selection_fg).bg(ui.danger),
                focus: Some(Style::default().fg(ui.selection_fg).bg(ui.danger)),
                armed: Some(Style::default().fg(ui.selection_fg).bg(ui.selection_bg)),
                ..Default::default()
            });
        button.render(button_area, frame.buffer_mut(), state);
    };

    // ── Fixed bars: menu (row 0) + status bar (row 1) ────────────────────────
    let menu_area   = Rect::new(area.x, area.y,     area.width, 1);
    let status_area = Rect::new(area.x, area.y + 1, area.width, 1);
    // Desktop: everything below the two fixed bars
    let desktop = Rect::new(area.x, area.y + 2, area.width, area.height.saturating_sub(2));

    if screen.is_budget_open && screen.budget_needs_center {
        let fitted = centered_fit_rect(desktop, 74, 29);
        screen.budget_win.width = fitted.width;
        screen.budget_win.height = fitted.height;
        screen.budget_win.x = fitted.x;
        screen.budget_win.y = fitted.y;
        screen.budget_needs_center = false;
    }

    // ── Clamp a FloatingWindow, return its outer Rect ────────────────────────
    // Windows may be dragged partially off-screen. The only hard rules are:
    //   • Title bar row must stay within the desktop (so it can be grabbed).
    //   • At least 4 columns of the title bar must remain visible on screen.
    //   • The rendered rect is clipped to the buffer boundary so nothing panics.
    let clamp_win = |win: &mut crate::app::FloatingWindow| -> Rect {
        clamp_window_to_desktop(win, desktop)
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

    let map_layout = {
        let engine = app.engine.read().unwrap();
        game::map_view::layout_map_chrome(
            map_inner,
            engine.map.width,
            engine.map.height,
            screen.camera.offset_x.max(0) as usize,
            screen.camera.offset_y.max(0) as usize,
        )
    };

    // ── Update camera viewport ───────────────────────────────────────────────
    // Use the *exposed* portion of the tile viewport so scroll_to_cursor keeps
    // the cursor in the area not hidden behind the tools window.
    let exposed_map_w = if panel_rect.x > map_layout.viewport.x {
        (panel_rect.x - map_layout.viewport.x) as usize
    } else {
        map_layout.viewport.width as usize
    }
    .min(map_layout.viewport.width as usize)
    .max(1);
    screen.camera.view_w = (exposed_map_w / 2).max(1);
    screen.camera.view_h = map_layout.view_tiles_h.max(1);

    // ── Update click areas ────────────────────────────────────────────────────
    screen.ui_areas.map = MapUiAreas {
        viewport: to_click_area(map_layout.viewport),
        vertical_bar: to_click_area(map_layout.vertical_bar),
        vertical_dec: to_click_area(map_layout.vertical_dec),
        vertical_track: to_click_area(map_layout.vertical_track),
        vertical_thumb: to_click_area(map_layout.vertical_thumb),
        vertical_inc: to_click_area(map_layout.vertical_inc),
        horizontal_bar: to_click_area(map_layout.horizontal_bar),
        horizontal_dec: to_click_area(map_layout.horizontal_dec),
        horizontal_track: to_click_area(map_layout.horizontal_track),
        horizontal_thumb: to_click_area(map_layout.horizontal_thumb),
        horizontal_inc: to_click_area(map_layout.horizontal_inc),
        corner: to_click_area(map_layout.corner),
    };
    // minimap updated when panel content is rendered below; toolbar uses rat-widget states.

    // ── Background ────────────────────────────────────────────────────────────
    frame.render_widget(
        Block::default().style(Style::default().bg(ui.desktop_bg)),
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
            .title_style(Style::default().fg(ui.window_title))
            .border_style(Style::default().fg(ui.window_border))
            .style(Style::default().bg(ui.map_window_bg)),
        map_rect,
    );

    if map_layout.viewport.width > 0 && map_layout.viewport.height > 0 {
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
            map_layout.viewport,
        );
        game::map_view::render_scrollbars(&map_layout, frame.buffer_mut());
    }

    // ── Panel window ──────────────────────────────────────────────────────────
    frame.render_widget(Clear, panel_rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" TOOLS ")
            .title_style(Style::default().fg(ui.window_title))
            .border_style(Style::default().fg(ui.window_border))
            .style(Style::default().bg(ui.panel_window_bg)),
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
                &mut screen.toolbar_btn_states,
            );
        }
        if minimap_area.width > 0 && minimap_area.height > 0 {
            frame.render_widget(
                game::minimap::MiniMap {
                    map: &engine.map,
                    camera: &screen.camera,
                    overlay_mode: screen.overlay_mode,
                },
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
                    power_produced: engine.sim.power_produced_mw,
                    power_consumed: engine.sim.power_consumed_mw,
                    },
                    info_area,
                    );        }
        // engine lock released here
    }

    if screen.show_plant_info {
        game::power_popup::render_power_popup(frame, area, screen);
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
                .title(" Budget Control Center ")
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.budget_window_bg)),
            budget_rect,
        );
        render_close_button(frame, budget_rect, &mut screen.budget_close_btn);
        game::budget::render_budget_content(frame.buffer_mut(), budget_inner, &mock_app, &mut screen.budget_ui);
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
                    .title_style(Style::default().fg(ui.window_title))
                    .border_style(Style::default().fg(ui.window_border))
                    .style(Style::default().bg(ui.inspect_window_bg)),
                inspect_rect,
            );
            render_close_button(frame, inspect_rect, &mut screen.inspect_close_btn);
            game::inspect_popup::render_inspect_content(
                frame.buffer_mut(), inspect_inner, inspect_pos, &engine.map,
            );
        }
    }

    // ── Menu bar — always on top ──────────────────────────────────────────────
    {
        use rat_widget::menu::{MenuStyle, Menubar};
        use ratatui::style::Style;
        use ratatui::widgets::Borders;

        let menu_model = {
            let engine = app.engine.read().unwrap();
            crate::app::screens::InGameMenu::from_screen(screen, &engine.sim)
        };
        let menu_style = MenuStyle {
            style: Style::default().fg(ui.menu_fg).bg(ui.menu_bg),
            focus: Some(Style::default().fg(ui.menu_focus_fg).bg(ui.menu_focus_bg)),
            right: Some(Style::default().fg(ui.menu_right).bg(ui.menu_bg)),
            highlight: Some(Style::default().fg(ui.menu_hotkey).bg(ui.menu_bg)),
            popup_style: Some(Style::default().fg(ui.menu_fg).bg(ui.menu_bg)),
            popup_focus: Some(Style::default().fg(ui.menu_focus_fg).bg(ui.menu_focus_bg)),
            popup_right: Some(Style::default().fg(ui.menu_right).bg(ui.menu_bg)),
            popup_highlight: Some(Style::default().fg(ui.menu_hotkey).bg(ui.menu_bg)),
            popup_separator: Some(Style::default().fg(ui.menu_right).bg(ui.menu_bg)),
            popup_block: Some(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ui.menu_fg).bg(ui.menu_bg))
                    .style(Style::default().bg(ui.menu_bg)),
            ),
            ..Default::default()
        };
        let (menu_bar, menu_popup) = Menubar::new(&menu_model)
            .title(" TuiCity2 ")
            .title_style(Style::default().fg(ui.menu_title).bg(ui.menu_bg))
            .popup_width(24)
            .styles(menu_style)
            .into_widgets();
        let buf = frame.buffer_mut();
        for x in menu_area.x..menu_area.x + menu_area.width {
            if let Some(cell) = buf.cell_mut((x, menu_area.y)) {
                cell.set_bg(ui.menu_bg);
                cell.set_fg(ui.menu_fg);
                cell.set_char(' ');
            }
        }
        menu_bar.render(menu_area, buf, &mut screen.menu);
        menu_popup.render(menu_area, buf, &mut screen.menu);
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
