use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{ClickArea, MapUiAreas, UiRect, WindowId},
    game_info::GAME_NAME,
    ui::{
        frontends::terminal::render_confirm_dialog,
        game,
        runtime::UiRect as RuntimeRect,
        view::InGameDesktopView,
    },
};

fn to_rect(rect: RuntimeRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width, rect.height)
}

fn to_click_area(rect: RuntimeRect) -> ClickArea {
    ClickArea {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn render_close_button(frame: &mut Frame, rect: Rect) {
    let ui = crate::ui::theme::ui_palette();
    if rect.width < 5 || rect.height == 0 {
        return;
    }
    frame.buffer_mut().set_string(
        rect.x + rect.width - 5,
        rect.y,
        "[X]",
        Style::default()
            .fg(ui.selection_fg)
            .bg(ui.danger)
            .add_modifier(Modifier::BOLD),
    );
}

fn render_window_shadow(frame: &mut Frame, rect: Rect) {
    let ui = crate::ui::theme::ui_palette();
    let buf = frame.buffer_mut();

    if rect.width == 0 || rect.height == 0 {
        return;
    }

    let right_x = rect.x + rect.width;
    if right_x < buf.area.x + buf.area.width {
        for y in rect.y.saturating_add(1)..rect.y + rect.height {
            if y < buf.area.y + buf.area.height {
                if let Some(cell) = buf.cell_mut((right_x, y)) {
                    darken_shadow_cell(cell, ui.window_shadow);
                }
            }
        }
    }

    let bottom_y = rect.y + rect.height;
    if bottom_y < buf.area.y + buf.area.height {
        for x in rect.x.saturating_add(1)..=rect.x + rect.width {
            if x < buf.area.x + buf.area.width {
                if let Some(cell) = buf.cell_mut((x, bottom_y)) {
                    darken_shadow_cell(cell, ui.window_shadow);
                }
            }
        }
    }
}

fn render_text_window_content(
    frame: &mut Frame,
    layout: &crate::ui::runtime::WindowLayout,
    view: &crate::ui::view::TextWindowViewModel,
    bg: ratatui::style::Color,
    fg: ratatui::style::Color,
) {
    let padded_inner = to_rect(layout.padded_inner);

    if padded_inner.width > 0 && padded_inner.height > 0 {
        frame.render_widget(
            Paragraph::new(view.lines.join("\n"))
                .style(Style::default().fg(fg).bg(bg))
                .wrap(Wrap { trim: false })
                .scroll((view.scroll_y, 0)),
            padded_inner,
        );
    }

    // Scrollbar
    if let Some(sb) = &layout.scrollbar {
        let buf = frame.buffer_mut();

        // Top arrow
        if let Some(cell) = buf.cell_mut(to_pos(sb.dec)) {
            cell.set_char('▲');
            cell.set_fg(fg);
            cell.set_bg(bg);
        }

        // Bottom arrow
        if let Some(cell) = buf.cell_mut(to_pos(sb.inc)) {
            cell.set_char('▼');
            cell.set_fg(fg);
            cell.set_bg(bg);
        }

        // Track
        let track = to_rect(sb.track);
        for y in 0..track.height {
            if let Some(cell) = buf.cell_mut((track.x, track.y + y)) {
                cell.set_char('│');
                cell.set_fg(fg);
                cell.set_bg(bg);
            }
        }

        // Thumb
        let thumb = to_rect(sb.thumb);
        for y in 0..thumb.height {
            if let Some(cell) = buf.cell_mut((thumb.x, thumb.y + y)) {
                cell.set_char('█');
                cell.set_fg(fg);
                cell.set_bg(bg);
            }
        }
    }
}

fn to_pos(rect: RuntimeRect) -> (u16, u16) {
    (rect.x, rect.y)
}

fn darken_shadow_cell(cell: &mut ratatui::buffer::Cell, fallback: ratatui::style::Color) {
    cell.set_fg(darken_color(cell.fg, fallback, 0.45));
    cell.set_bg(darken_color(cell.bg, fallback, 0.45));
}

fn darken_color(
    color: ratatui::style::Color,
    fallback: ratatui::style::Color,
    factor: f32,
) -> ratatui::style::Color {
    let (r, g, b) =
        color_to_rgb(color).unwrap_or_else(|| color_to_rgb(fallback).unwrap_or((0, 0, 0)));
    ratatui::style::Color::Rgb(
        ((r as f32) * factor).round() as u8,
        ((g as f32) * factor).round() as u8,
        ((b as f32) * factor).round() as u8,
    )
}

fn color_to_rgb(color: ratatui::style::Color) -> Option<(u8, u8, u8)> {
    use ratatui::style::Color;

    Some(match color {
        Color::Reset => return None,
        Color::Black => (0, 0, 0),
        Color::Red => (128, 0, 0),
        Color::Green => (0, 128, 0),
        Color::Yellow => (128, 128, 0),
        Color::Blue => (0, 0, 128),
        Color::Magenta => (128, 0, 128),
        Color::Cyan => (0, 128, 128),
        Color::Gray => (192, 192, 192),
        Color::DarkGray => (128, 128, 128),
        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (0, 0, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(idx) => indexed_color_to_rgb(idx),
    })
}

fn indexed_color_to_rgb(idx: u8) -> (u8, u8, u8) {
    const ANSI: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];

    match idx {
        0..=15 => ANSI[idx as usize],
        16..=231 => {
            let value = idx - 16;
            let r = value / 36;
            let g = (value % 36) / 6;
            let b = value % 6;
            let level = |n: u8| if n == 0 { 0 } else { 55 + n * 40 };
            (level(r), level(g), level(b))
        }
        232..=255 => {
            let gray = 8 + (idx - 232) * 10;
            (gray, gray, gray)
        }
    }
}

pub fn render_ingame(
    frame: &mut Frame,
    area: Rect,
    screen: &mut crate::app::screens::InGameScreen,
    view: &InGameDesktopView,
) {
    let ui = crate::ui::theme::ui_palette();
    let desktop_layout = screen
        .desktop
        .layout(UiRect::new(area.x, area.y, area.width, area.height));
    screen.ui_areas.desktop = desktop_layout.clone();

    let menu_area = to_rect(desktop_layout.menu_bar);
    let status_area = to_rect(desktop_layout.status_bar);
    let news_area = to_rect(desktop_layout.news_ticker);
    let map_outer = to_rect(desktop_layout.window(WindowId::Map).outer);
    let map_inner = to_rect(desktop_layout.window(WindowId::Map).inner);
    let panel_outer = to_rect(desktop_layout.window(WindowId::Panel).outer);
    let panel_inner = to_rect(desktop_layout.window(WindowId::Panel).inner);
    let budget_outer = to_rect(desktop_layout.window(WindowId::Budget).outer);
    let budget_inner = to_rect(desktop_layout.window(WindowId::Budget).inner);
    let statistics_outer = to_rect(desktop_layout.window(WindowId::Statistics).outer);
    let statistics_inner = to_rect(desktop_layout.window(WindowId::Statistics).inner);
    let inspect_outer = to_rect(desktop_layout.window(WindowId::Inspect).outer);
    let inspect_inner = to_rect(desktop_layout.window(WindowId::Inspect).inner);
    let power_outer = to_rect(desktop_layout.window(WindowId::PowerPicker).outer);
    let power_inner = to_rect(desktop_layout.window(WindowId::PowerPicker).inner);

    let map_layout = game::map_view::layout_map_chrome(
        map_inner,
        view.map.width,
        view.map.height,
        screen.camera.offset_x.max(0) as usize,
        screen.camera.offset_y.max(0) as usize,
    );

    let exposed_map_w = if panel_outer.x > map_layout.viewport.x {
        (panel_outer.x - map_layout.viewport.x) as usize
    } else {
        map_layout.viewport.width as usize
    }
    .min(map_layout.viewport.width as usize)
    .max(1);
    screen.camera.view_w = (exposed_map_w / 2).max(1);
    screen.camera.view_h = map_layout.view_tiles_h.max(1);

    screen.ui_areas.map = MapUiAreas {
        viewport: to_click_area(UiRect::new(
            map_layout.viewport.x,
            map_layout.viewport.y,
            map_layout.viewport.width,
            map_layout.viewport.height,
        )),
        vertical_bar: to_click_area(UiRect::new(
            map_layout.vertical_bar.x,
            map_layout.vertical_bar.y,
            map_layout.vertical_bar.width,
            map_layout.vertical_bar.height,
        )),
        vertical_dec: to_click_area(UiRect::new(
            map_layout.vertical_dec.x,
            map_layout.vertical_dec.y,
            map_layout.vertical_dec.width,
            map_layout.vertical_dec.height,
        )),
        vertical_track: to_click_area(UiRect::new(
            map_layout.vertical_track.x,
            map_layout.vertical_track.y,
            map_layout.vertical_track.width,
            map_layout.vertical_track.height,
        )),
        vertical_thumb: to_click_area(UiRect::new(
            map_layout.vertical_thumb.x,
            map_layout.vertical_thumb.y,
            map_layout.vertical_thumb.width,
            map_layout.vertical_thumb.height,
        )),
        vertical_inc: to_click_area(UiRect::new(
            map_layout.vertical_inc.x,
            map_layout.vertical_inc.y,
            map_layout.vertical_inc.width,
            map_layout.vertical_inc.height,
        )),
        horizontal_bar: to_click_area(UiRect::new(
            map_layout.horizontal_bar.x,
            map_layout.horizontal_bar.y,
            map_layout.horizontal_bar.width,
            map_layout.horizontal_bar.height,
        )),
        horizontal_dec: to_click_area(UiRect::new(
            map_layout.horizontal_dec.x,
            map_layout.horizontal_dec.y,
            map_layout.horizontal_dec.width,
            map_layout.horizontal_dec.height,
        )),
        horizontal_track: to_click_area(UiRect::new(
            map_layout.horizontal_track.x,
            map_layout.horizontal_track.y,
            map_layout.horizontal_track.width,
            map_layout.horizontal_track.height,
        )),
        horizontal_thumb: to_click_area(UiRect::new(
            map_layout.horizontal_thumb.x,
            map_layout.horizontal_thumb.y,
            map_layout.horizontal_thumb.width,
            map_layout.horizontal_thumb.height,
        )),
        horizontal_inc: to_click_area(UiRect::new(
            map_layout.horizontal_inc.x,
            map_layout.horizontal_inc.y,
            map_layout.horizontal_inc.width,
            map_layout.horizontal_inc.height,
        )),
        corner: to_click_area(UiRect::new(
            map_layout.corner.x,
            map_layout.corner.y,
            map_layout.corner.width,
            map_layout.corner.height,
        )),
    };

    frame.render_widget(
        Block::default().style(Style::default().bg(ui.desktop_bg)),
        area,
    );

    let status_areas = game::statusbar::render_statusbar(
        status_area,
        frame.buffer_mut(),
        &view.sim,
        view.paused,
        view.view_layer,
        view.status_message.as_deref(),
    );
    screen.ui_areas.pause_btn = status_areas.pause_btn;
    screen.ui_areas.layer_surface_btn = status_areas.layer_surface_btn;
    screen.ui_areas.layer_underground_btn = status_areas.layer_underground_btn;
    game::news_ticker::render_news_ticker(news_area, frame.buffer_mut(), &view.news_ticker);

    if screen.desktop.window(WindowId::Map).shadowed {
        render_window_shadow(frame, map_outer);
    }
    frame.render_widget(Clear, map_outer);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(screen.desktop.window(WindowId::Map).title)
            .title_style(Style::default().fg(ui.window_title))
            .border_style(Style::default().fg(ui.window_border))
            .style(Style::default().bg(ui.map_window_bg)),
        map_outer,
    );

    if map_layout.viewport.width > 0 && map_layout.viewport.height > 0 {
        use crate::core::tool::Tool;
        use crate::ui::game::map_view::PreviewKind;

        let footprint_tiles: Vec<(usize, usize)> = if view.rect_preview.is_empty()
            && view.line_preview.is_empty()
            && Tool::uses_footprint_preview(view.current_tool)
        {
            let (fw, fh): (usize, usize) = view.current_tool.footprint();
            let (cx, cy) = (view.camera.cursor_x, view.camera.cursor_y);
            let ax = cx
                .saturating_sub(fw / 2)
                .min(view.map.width.saturating_sub(fw));
            let ay = cy
                .saturating_sub(fh / 2)
                .min(view.map.height.saturating_sub(fh));
            (0..fh)
                .flat_map(|dy| (0..fw).map(move |dx| (ax + dx, ay + dy)))
                .collect()
        } else {
            Vec::new()
        };
        let footprint_all_valid = footprint_tiles.iter().all(|&(x, y)| {
            x < view.map.width
                && y < view.map.height
                && view
                    .current_tool
                    .can_place(view.map.view_tile(view.view_layer, x, y))
        });
        let (preview_tiles, preview_kind): (&[(usize, usize)], PreviewKind) =
            if !view.rect_preview.is_empty() {
                (
                    view.rect_preview.as_slice(),
                    PreviewKind::Rect(view.current_tool),
                )
            } else if !view.line_preview.is_empty() {
                (
                    view.line_preview.as_slice(),
                    PreviewKind::Line(view.current_tool),
                )
            } else if !footprint_tiles.is_empty() {
                (
                    &footprint_tiles,
                    PreviewKind::Footprint(view.current_tool, footprint_all_valid),
                )
            } else {
                (&[], PreviewKind::None)
            };

        frame.render_widget(
            game::map_view::MapView {
                map: &view.map,
                camera: &view.camera,
                line_preview: preview_tiles,
                preview_kind,
                overlay_mode: view.overlay_mode,
                view_layer: view.view_layer,
            },
            map_layout.viewport,
        );
        game::map_view::render_scrollbars(&map_layout, frame.buffer_mut());
    }

    screen.ui_areas.toolbar_items.clear();
    screen.ui_areas.tool_chooser_items.clear();
    screen.ui_areas.dialog_items.clear();
    screen.ui_areas.minimap = ClickArea::default();

    if screen.desktop.is_open(WindowId::Panel) && panel_outer.width > 0 && panel_outer.height > 0 {
        if screen.desktop.window(WindowId::Panel).shadowed {
            render_window_shadow(frame, panel_outer);
        }
        frame.render_widget(Clear, panel_outer);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(screen.desktop.window(WindowId::Panel).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.panel_window_bg)),
            panel_outer,
        );
        render_close_button(frame, panel_outer);
    }

    if screen.desktop.is_open(WindowId::Panel) && panel_inner.width > 0 {
        let panel_window = screen.desktop.window(WindowId::Panel);
        let full_inner_h = panel_window.height.saturating_sub(2).max(4);
        let full_inner = Rect::new(
            panel_inner.x,
            panel_inner.y,
            panel_inner.width,
            full_inner_h,
        );
        let ph = full_inner.height;
        let desired_toolbar_h = game::toolbar::toolbar_height(&view.toolbar);
        let minimum_toolbar_h = game::toolbar::minimum_toolbar_height(&view.toolbar);
        let minimum_minimap_h = 8;
        let minimum_info_h = 5;
        let max_toolbar_h = ph.saturating_sub(minimum_minimap_h + minimum_info_h);
        let toolbar_h = desired_toolbar_h
            .min(max_toolbar_h.max(minimum_toolbar_h))
            .min(ph);
        let remaining_h = ph.saturating_sub(toolbar_h);
        let desired_minimap_h = (ph / 3).max(minimum_minimap_h);
        let max_minimap_h = remaining_h.saturating_sub(minimum_info_h);
        let mut minimap_h = desired_minimap_h.min(max_minimap_h);
        if minimap_h < minimum_minimap_h && remaining_h > minimum_info_h {
            minimap_h = minimum_minimap_h.min(max_minimap_h);
        }
        let mut info_h = remaining_h.saturating_sub(minimap_h);
        if info_h < minimum_info_h {
            let needed = minimum_info_h - info_h;
            minimap_h = minimap_h.saturating_sub(needed);
            info_h = remaining_h.saturating_sub(minimap_h);
        }
        let panel_vert = Layout::vertical([
            Constraint::Length(toolbar_h),
            Constraint::Length(minimap_h),
            Constraint::Length(info_h),
        ])
        .split(full_inner);

        let toolbar_area = panel_vert[0].intersection(area);
        let minimap_area = panel_vert[1].intersection(area);
        let info_area = panel_vert[2].intersection(area);

        if toolbar_area.width > 0 && toolbar_area.height > 0 {
            screen.ui_areas.toolbar_items =
                game::toolbar::render_toolbar(toolbar_area, frame.buffer_mut(), &view.toolbar);
        }
        if minimap_area.width > 0 && minimap_area.height > 0 {
            let render_area =
                game::minimap::minimap_render_area(minimap_area, view.map.width, view.map.height);
            screen.ui_areas.minimap = to_click_area(UiRect::new(
                render_area.x,
                render_area.y,
                render_area.width,
                render_area.height,
            ));
            frame.render_widget(
                game::minimap::MiniMap {
                    map: &view.map,
                    camera: &view.camera,
                    overlay_mode: view.overlay_mode,
                    view_layer: view.view_layer,
                },
                minimap_area,
            );
        }

        let cx = view.camera.cursor_x.min(view.map.width.saturating_sub(1));
        let cy = view.camera.cursor_y.min(view.map.height.saturating_sub(1));
        let tile = if view.map.width > 0 && view.map.height > 0 {
            view.map.surface_lot_tile(cx, cy)
        } else {
            crate::core::map::Tile::Grass
        };
        let tile_overlay = if view.map.width > 0 && view.map.height > 0 {
            view.map.get_overlay(cx, cy)
        } else {
            crate::core::map::TileOverlay::default()
        };

        if info_area.width > 0 && info_area.height > 0 {
            frame.render_widget(
                game::infopanel::InfoPanel {
                    tile,
                    overlay: tile_overlay,
                    zone: view.map.effective_zone_kind(cx, cy),
                    x: cx,
                    y: cy,
                    current_tool: view.current_tool,
                    demand_res: view.sim.demand_res,
                    demand_comm: view.sim.demand_comm,
                    demand_ind: view.sim.demand_ind,
                    demand_history_res: &view.sim.demand_history_res,
                    demand_history_comm: &view.sim.demand_history_comm,
                    demand_history_ind: &view.sim.demand_history_ind,
                    power_produced: view.sim.power_produced_mw,
                    power_consumed: view.sim.power_consumed_mw,
                },
                info_area,
            );
        }
    }

    if let Some(chooser) = &view.tool_chooser {
        if screen.desktop.window(WindowId::PowerPicker).shadowed {
            render_window_shadow(frame, power_outer);
        }
        frame.render_widget(Clear, power_outer);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(screen.desktop.window(WindowId::PowerPicker).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            power_outer,
        );
        render_close_button(frame, power_outer);
        screen.ui_areas.tool_chooser_items = game::power_popup::render_tool_chooser_content(
            frame.buffer_mut(),
            power_inner,
            chooser,
        );
    }

    if screen.is_budget_open() {
        if screen.desktop.window(WindowId::Budget).shadowed {
            render_window_shadow(frame, budget_outer);
        }
        frame.render_widget(Clear, budget_outer);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(screen.desktop.window(WindowId::Budget).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.budget_window_bg)),
            budget_outer,
        );
        render_close_button(frame, budget_outer);
        game::budget::render_budget_content(frame.buffer_mut(), budget_inner, &view.budget);
    }

    if let Some(statistics) = &view.statistics {
        if screen.desktop.window(WindowId::Statistics).shadowed {
            render_window_shadow(frame, statistics_outer);
        }
        frame.render_widget(Clear, statistics_outer);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(screen.desktop.window(WindowId::Statistics).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            statistics_outer,
        );
        render_close_button(frame, statistics_outer);
        game::statistics::render_statistics_content(frame, statistics_inner, statistics);
    }

    if screen.is_inspect_open() {
        if let Some(inspect_pos) = view.inspect_pos {
            if inspect_pos.0 < view.map.width && inspect_pos.1 < view.map.height {
                let title = format!(" Inspect ({},{}) ", inspect_pos.0, inspect_pos.1);
                if screen.desktop.window(WindowId::Inspect).shadowed {
                    render_window_shadow(frame, inspect_outer);
                }
                frame.render_widget(Clear, inspect_outer);
                frame.render_widget(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title.as_str())
                        .title_style(Style::default().fg(ui.window_title))
                        .border_style(Style::default().fg(ui.window_border))
                        .style(Style::default().bg(ui.inspect_window_bg)),
                    inspect_outer,
                );
                render_close_button(frame, inspect_outer);
                game::inspect_popup::render_inspect_content(
                    frame.buffer_mut(),
                    inspect_inner,
                    inspect_pos,
                    &view.map,
                );
            }
        }
    }

    if let Some(help) = &view.help {
        let layout = desktop_layout.window(WindowId::Help);
        if screen.desktop.window(WindowId::Help).shadowed {
            render_window_shadow(frame, to_rect(layout.outer));
        }
        let outer = to_rect(layout.outer);
        frame.render_widget(Clear, outer);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(screen.desktop.window(WindowId::Help).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            outer,
        );
        render_close_button(frame, outer);
        render_text_window_content(frame, &layout, help, ui.popup_bg, ui.text_primary);
    }

    if let Some(about) = &view.about {
        let layout = desktop_layout.window(WindowId::About);
        if screen.desktop.window(WindowId::About).shadowed {
            render_window_shadow(frame, to_rect(layout.outer));
        }
        let outer = to_rect(layout.outer);
        frame.render_widget(Clear, outer);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(screen.desktop.window(WindowId::About).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            outer,
        );
        render_close_button(frame, outer);
        render_text_window_content(frame, &layout, about, ui.popup_bg, ui.text_primary);
    }

    if let Some(legend) = &view.legend {
        let layout = desktop_layout.window(WindowId::Legend);
        if screen.desktop.window(WindowId::Legend).shadowed {
            render_window_shadow(frame, to_rect(layout.outer));
        }
        let outer = to_rect(layout.outer);
        frame.render_widget(Clear, outer);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(screen.desktop.window(WindowId::Legend).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            outer,
        );
        render_close_button(frame, outer);
        render_text_window_content(frame, &layout, legend, ui.popup_bg, ui.text_primary);
    }

    render_menu_bar(frame, menu_area, screen, view);

    if let Some(dialog) = &view.confirm_dialog {
        screen.ui_areas.dialog_items = render_confirm_dialog(frame, area, dialog);
    }
}

fn render_menu_bar(
    frame: &mut Frame,
    area: Rect,
    screen: &mut crate::app::screens::InGameScreen,
    view: &InGameDesktopView,
) {
    let ui = crate::ui::theme::ui_palette();
    screen.ui_areas.menu_bar = ClickArea {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height,
    };
    screen.ui_areas.menu_items = [ClickArea::default(); 6];
    screen.ui_areas.menu_popup = ClickArea::default();
    screen.ui_areas.menu_popup_items.clear();
    {
        let buf = frame.buffer_mut();
        for x in area.x..area.x + area.width {
            buf.set_string(
                x,
                area.y,
                " ",
                Style::default().fg(ui.menu_fg).bg(ui.menu_bg),
            );
        }
    }

    let mut x = area.x;
    let title = format!(" {GAME_NAME} ");
    if x + title.len() as u16 <= area.x + area.width {
        frame.buffer_mut().set_string(
            x,
            area.y,
            &title,
            Style::default()
                .fg(ui.menu_title)
                .bg(ui.menu_bg)
                .add_modifier(Modifier::BOLD),
        );
        x += title.len() as u16 + 1;
    }

    for (idx, title) in crate::app::screens::MENU_TITLES.iter().take(4).enumerate() {
        let text = format!(" {} ", title);
        let style = if view.menu_active && view.menu_selected == idx {
            Style::default()
                .fg(ui.menu_focus_fg)
                .bg(ui.menu_focus_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui.menu_fg).bg(ui.menu_bg)
        };
        if x + text.len() as u16 <= area.x + area.width {
            screen.ui_areas.menu_items[idx] = ClickArea {
                x,
                y: area.y,
                width: text.len() as u16,
                height: 1,
            };
            frame.buffer_mut().set_string(x, area.y, &text, style);
        }
        x += text.len() as u16 + 1;
    }

    let mut right_x = area.x + area.width;
    for (idx, title) in crate::app::screens::MENU_TITLES
        .iter()
        .enumerate()
        .skip(4)
        .rev()
    {
        let text = format!(" {} ", title);
        let text_w = text.len() as u16;
        if right_x < area.x + text_w {
            continue;
        }
        right_x = right_x.saturating_sub(text_w);
        let style = if view.menu_active && view.menu_selected == idx {
            Style::default()
                .fg(ui.menu_focus_fg)
                .bg(ui.menu_focus_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ui.menu_fg).bg(ui.menu_bg)
        };
        screen.ui_areas.menu_items[idx] = ClickArea {
            x: right_x,
            y: area.y,
            width: text_w,
            height: 1,
        };
        frame.buffer_mut().set_string(right_x, area.y, &text, style);
        right_x = right_x.saturating_sub(1);
    }

    if !view.menu_active {
        return;
    }

    let menu = view.menu_selected;
    let rows = crate::app::screens::menu_rows(menu);
    if rows.is_empty() {
        return;
    }
    let anchor = screen.ui_areas.menu_items[menu];
    let popup_w = 28.min(area.width.max(8));
    let popup_x = anchor
        .x
        .min(area.x + area.width.saturating_sub(popup_w.max(8)));
    let popup_h = rows.len() as u16 + 2;
    let popup = Rect::new(popup_x, area.y + 1, popup_w.max(8), popup_h);
    screen.ui_areas.menu_popup = ClickArea {
        x: popup.x,
        y: popup.y,
        width: popup.width,
        height: popup.height,
    };
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ui.menu_fg).bg(ui.menu_bg))
            .style(Style::default().bg(ui.menu_bg)),
        popup,
    );
    let buf = frame.buffer_mut();
    for (idx, _) in rows.iter().enumerate() {
        if let Some((label, right, _)) = screen.menu_row(menu, idx, &view.sim) {
            let y = popup.y + 1 + idx as u16;
            screen.ui_areas.menu_popup_items.push(ClickArea {
                x: popup.x + 1,
                y,
                width: popup.width.saturating_sub(2),
                height: 1,
            });
            let selected = idx == view.menu_item_selected;
            let style = if selected {
                Style::default().fg(ui.menu_focus_fg).bg(ui.menu_focus_bg)
            } else {
                Style::default().fg(ui.menu_fg).bg(ui.menu_bg)
            };
            let content_w = popup.width.saturating_sub(2);
            let mut line = label;
            if !right.is_empty() {
                let right_w = right.chars().count() as u16;
                let left_w = content_w.saturating_sub(right_w + 1) as usize;
                line = format!(
                    "{:<width$} {}",
                    truncate(&line, left_w),
                    right,
                    width = left_w
                );
            }
            buf.set_string(
                popup.x + 1,
                y,
                format!(
                    "{:<width$}",
                    truncate(&line, content_w as usize),
                    width = content_w as usize
                ),
                style,
            );
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn darken_color_scales_rgb_values() {
        assert_eq!(
            darken_color(Color::Rgb(200, 100, 50), Color::Black, 0.5),
            Color::Rgb(100, 50, 25)
        );
    }

    #[test]
    fn darken_color_uses_fallback_for_reset() {
        assert_eq!(
            darken_color(Color::Reset, Color::Rgb(20, 40, 60), 0.5),
            Color::Rgb(10, 20, 30)
        );
    }
}
