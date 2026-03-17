pub mod game;
pub mod screens;
pub mod theme;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::app::{AppState, ClickArea, Screen};

pub fn render(frame: &mut Frame, app: &mut AppState) {
    let area = frame.area();
    match &app.screen {
        Screen::Start(_) => render_start_screen(frame, area, app),
        Screen::NewCity(_) => render_new_city_screen(frame, area, app),
        Screen::LoadCity(_) => render_load_city_screen(frame, area, app),
        Screen::InGame => render_game(frame, area, app),
    }
}

fn render_start_screen(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Screen::Start(ref state) = app.screen {
        screens::start::render_start(frame, area, state);
    }
}

fn render_new_city_screen(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Screen::NewCity(ref state) = app.screen {
        screens::new_city::render_new_city(frame, area, state);
    }
}

fn render_load_city_screen(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Screen::LoadCity(ref state) = app.screen {
        screens::load_city::render_load_city(frame, area, state);
    }
}

fn render_game(frame: &mut Frame, area: Rect, app: &mut AppState) {
    // Layout: status bar (1 row) + main area
    let vert = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(area);

    let status_area = vert[0];
    let main_area = vert[1];

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
    app.camera.view_w = map_area.width as usize;
    app.camera.view_h = map_area.height as usize;

    // Update click areas
    app.ui_areas.map = to_click_area(map_area);
    app.ui_areas.minimap = to_click_area(minimap_area);

    // Render status bar → get pause button area
    let pause_area = game::statusbar::render_statusbar(
        status_area,
        frame.buffer_mut(),
        &app.sim,
        app.paused,
        app.message.as_deref(),
    );
    app.ui_areas.pause_btn = pause_area;

    // Render map view
    use crate::core::tool::Tool;
    use crate::ui::game::map_view::PreviewKind;

    // Pre-compute footprint tiles (owned) so the slice lifetime covers the render call.
    let footprint_tiles: Vec<(usize, usize)> =
        if app.rect_drag.is_none() && app.line_drag.is_none()
            && Tool::uses_footprint_preview(app.current_tool)
        {
            let (fw, fh) = app.current_tool.footprint();
            let (cx, cy) = (app.camera.cursor_x, app.camera.cursor_y);
            let ax = cx.saturating_sub(fw / 2)
                       .min(app.map.width.saturating_sub(fw));
            let ay = cy.saturating_sub(fh / 2)
                       .min(app.map.height.saturating_sub(fh));
            (0..fh).flat_map(|dy| (0..fw).map(move |dx| (ax + dx, ay + dy))).collect()
        } else {
            Vec::new()
        };
    let footprint_all_valid = footprint_tiles.iter().all(|&(x, y)| {
        x < app.map.width && y < app.map.height
            && app.current_tool.can_place(app.map.get(x, y))
    });

    let (preview_tiles, preview_kind): (&[(usize, usize)], PreviewKind) =
        if let Some(ref d) = app.rect_drag {
            (app.rect_preview(), PreviewKind::Rect(d.tool))
        } else if let Some(ref d) = app.line_drag {
            (app.line_preview(), PreviewKind::Line(d.tool))
        } else if !footprint_tiles.is_empty() {
            (&footprint_tiles, PreviewKind::Footprint(app.current_tool, footprint_all_valid))
        } else {
            (&[], PreviewKind::None)
        };
    frame.render_widget(
        game::map_view::MapView {
            map: &app.map,
            camera: &app.camera,
            line_preview: preview_tiles,
            preview_kind,
        },
        map_area,
    );

    // Render toolbar → collect button areas
    game::toolbar::render_toolbar(
        toolbar_area,
        frame.buffer_mut(),
        app.current_tool,
        &mut app.ui_areas.toolbar_buttons,
    );

    // Render minimap
    frame.render_widget(
        game::minimap::MiniMap {
            map: &app.map,
            camera: &app.camera,
        },
        minimap_area,
    );

    // Render info panel
    let cx = app.camera.cursor_x.min(app.map.width.saturating_sub(1));
    let cy = app.camera.cursor_y.min(app.map.height.saturating_sub(1));
    let tile = if app.map.width > 0 && app.map.height > 0 {
        app.map.get(cx, cy)
    } else {
        crate::core::map::Tile::Grass
    };
    let overlay = if app.map.width > 0 && app.map.height > 0 {
        app.map.get_overlay(cx, cy)
    } else {
        crate::core::map::TileOverlay::default()
    };

    frame.render_widget(
        game::infopanel::InfoPanel {
            tile,
            overlay,
            x: cx,
            y: cy,
            current_tool: app.current_tool,
        },
        info_area,
    );
}

fn to_click_area(r: Rect) -> ClickArea {
    ClickArea {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }
}
