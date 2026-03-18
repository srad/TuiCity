use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear},
};

use crate::{
    app::{ClickArea, MapUiAreas, UiRect, WindowId},
    ui::{game, runtime::UiRect as RuntimeRect, view::InGameDesktopView},
};

fn to_rect(rect: RuntimeRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width, rect.height)
}

fn to_click_area(rect: RuntimeRect) -> ClickArea {
    ClickArea { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
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
        Style::default().fg(ui.selection_fg).bg(ui.danger).add_modifier(Modifier::BOLD),
    );
}

pub fn render_ingame(
    frame: &mut Frame,
    area: Rect,
    screen: &mut crate::app::screens::InGameScreen,
    view: &InGameDesktopView,
) {
    let ui = crate::ui::theme::ui_palette();
    let desktop_layout = screen.desktop.layout(UiRect::new(area.x, area.y, area.width, area.height));

    let menu_area = to_rect(desktop_layout.menu_bar);
    let status_area = to_rect(desktop_layout.status_bar);
    let map_outer = to_rect(desktop_layout.window(WindowId::Map).outer);
    let map_inner = to_rect(desktop_layout.window(WindowId::Map).inner);
    let panel_outer = to_rect(desktop_layout.window(WindowId::Panel).outer);
    let panel_inner = to_rect(desktop_layout.window(WindowId::Panel).inner);
    let budget_outer = to_rect(desktop_layout.window(WindowId::Budget).outer);
    let budget_inner = to_rect(desktop_layout.window(WindowId::Budget).inner);
    let inspect_outer = to_rect(desktop_layout.window(WindowId::Inspect).outer);
    let inspect_inner = to_rect(desktop_layout.window(WindowId::Inspect).inner);
    let power_outer = to_rect(desktop_layout.window(WindowId::PowerPicker).outer);

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
        viewport: to_click_area(UiRect::new(map_layout.viewport.x, map_layout.viewport.y, map_layout.viewport.width, map_layout.viewport.height)),
        vertical_bar: to_click_area(UiRect::new(map_layout.vertical_bar.x, map_layout.vertical_bar.y, map_layout.vertical_bar.width, map_layout.vertical_bar.height)),
        vertical_dec: to_click_area(UiRect::new(map_layout.vertical_dec.x, map_layout.vertical_dec.y, map_layout.vertical_dec.width, map_layout.vertical_dec.height)),
        vertical_track: to_click_area(UiRect::new(map_layout.vertical_track.x, map_layout.vertical_track.y, map_layout.vertical_track.width, map_layout.vertical_track.height)),
        vertical_thumb: to_click_area(UiRect::new(map_layout.vertical_thumb.x, map_layout.vertical_thumb.y, map_layout.vertical_thumb.width, map_layout.vertical_thumb.height)),
        vertical_inc: to_click_area(UiRect::new(map_layout.vertical_inc.x, map_layout.vertical_inc.y, map_layout.vertical_inc.width, map_layout.vertical_inc.height)),
        horizontal_bar: to_click_area(UiRect::new(map_layout.horizontal_bar.x, map_layout.horizontal_bar.y, map_layout.horizontal_bar.width, map_layout.horizontal_bar.height)),
        horizontal_dec: to_click_area(UiRect::new(map_layout.horizontal_dec.x, map_layout.horizontal_dec.y, map_layout.horizontal_dec.width, map_layout.horizontal_dec.height)),
        horizontal_track: to_click_area(UiRect::new(map_layout.horizontal_track.x, map_layout.horizontal_track.y, map_layout.horizontal_track.width, map_layout.horizontal_track.height)),
        horizontal_thumb: to_click_area(UiRect::new(map_layout.horizontal_thumb.x, map_layout.horizontal_thumb.y, map_layout.horizontal_thumb.width, map_layout.horizontal_thumb.height)),
        horizontal_inc: to_click_area(UiRect::new(map_layout.horizontal_inc.x, map_layout.horizontal_inc.y, map_layout.horizontal_inc.width, map_layout.horizontal_inc.height)),
        corner: to_click_area(UiRect::new(map_layout.corner.x, map_layout.corner.y, map_layout.corner.width, map_layout.corner.height)),
    };

    frame.render_widget(Block::default().style(Style::default().bg(ui.desktop_bg)), area);

    let pause_area = game::statusbar::render_statusbar(
        status_area,
        frame.buffer_mut(),
        &view.sim,
        view.paused,
        view.status_message.as_deref(),
    );
    screen.ui_areas.pause_btn = pause_area;

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

        let footprint_tiles: Vec<(usize, usize)> =
            if view.rect_preview.is_empty() && view.line_preview.is_empty()
                && Tool::uses_footprint_preview(view.current_tool)
            {
                let (fw, fh): (usize, usize) = view.current_tool.footprint();
                let (cx, cy) = (view.camera.cursor_x, view.camera.cursor_y);
                let ax = cx.saturating_sub(fw / 2).min(view.map.width.saturating_sub(fw));
                let ay = cy.saturating_sub(fh / 2).min(view.map.height.saturating_sub(fh));
                (0..fh).flat_map(|dy| (0..fw).map(move |dx| (ax + dx, ay + dy))).collect()
            } else {
                Vec::new()
            };
        let footprint_all_valid = footprint_tiles.iter().all(|&(x, y)| {
            x < view.map.width && y < view.map.height && view.current_tool.can_place(view.map.get(x, y))
        });
        let (preview_tiles, preview_kind): (&[(usize, usize)], PreviewKind) =
            if !view.rect_preview.is_empty() {
                (view.rect_preview.as_slice(), PreviewKind::Rect(view.current_tool))
            } else if !view.line_preview.is_empty() {
                (view.line_preview.as_slice(), PreviewKind::Line(view.current_tool))
            } else if !footprint_tiles.is_empty() {
                (&footprint_tiles, PreviewKind::Footprint(view.current_tool, footprint_all_valid))
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
            },
            map_layout.viewport,
        );
        game::map_view::render_scrollbars(&map_layout, frame.buffer_mut());
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

    if panel_inner.width > 0 {
        let panel_window = screen.desktop.window(WindowId::Panel);
        let full_inner_h = panel_window.height.saturating_sub(2).max(4);
        let full_inner = Rect::new(panel_inner.x, panel_inner.y, panel_inner.width, full_inner_h);
        let ph = full_inner.height;
        let toolbar_h = game::toolbar::toolbar_height();
        let minimap_h = (ph / 5).max(7).min(ph.saturating_sub(2));
        let info_h = (ph / 6).max(5).min(ph.saturating_sub(1 + minimap_h));
        let panel_vert = Layout::vertical([
            Constraint::Length(toolbar_h),
            Constraint::Length(minimap_h),
            Constraint::Length(info_h),
        ])
        .split(full_inner);

        let toolbar_area = panel_vert[0].intersection(area);
        let minimap_area = panel_vert[1].intersection(area);
        let info_area = panel_vert[2].intersection(area);
        screen.ui_areas.minimap = to_click_area(UiRect::new(minimap_area.x, minimap_area.y, minimap_area.width, minimap_area.height));

        if toolbar_area.width > 0 && toolbar_area.height > 0 {
            game::toolbar::render_toolbar(toolbar_area, frame.buffer_mut(), view.current_tool);
        }
        if minimap_area.width > 0 && minimap_area.height > 0 {
            frame.render_widget(
                game::minimap::MiniMap { map: &view.map, camera: &view.camera, overlay_mode: view.overlay_mode },
                minimap_area,
            );
        }

        let cx = view.camera.cursor_x.min(view.map.width.saturating_sub(1));
        let cy = view.camera.cursor_y.min(view.map.height.saturating_sub(1));
        let tile = if view.map.width > 0 && view.map.height > 0 {
            view.map.get(cx, cy)
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

    if screen.is_power_picker_open() {
        game::power_popup::render_power_popup(frame, power_outer);
    }

    if screen.is_budget_open() {
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

    if let Some(inspect_pos) = view.inspect_pos {
        if inspect_pos.0 < view.map.width && inspect_pos.1 < view.map.height {
            let title = format!(" Inspect ({},{}) ", inspect_pos.0, inspect_pos.1);
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
            game::inspect_popup::render_inspect_content(frame.buffer_mut(), inspect_inner, inspect_pos, &view.map);
        }
    }

    render_menu_bar(frame, menu_area, screen, view);
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
    screen.ui_areas.menu_items = [ClickArea::default(); 5];
    screen.ui_areas.menu_popup = ClickArea::default();
    screen.ui_areas.menu_popup_items = [ClickArea::default(); 7];
    {
        let buf = frame.buffer_mut();
        for x in area.x..area.x + area.width {
            buf.set_string(x, area.y, " ", Style::default().fg(ui.menu_fg).bg(ui.menu_bg));
        }
    }

    let mut x = area.x;
    let title = " TuiCity2 ";
    if x + title.len() as u16 <= area.x + area.width {
        frame.buffer_mut().set_string(
            x,
            area.y,
            title,
            Style::default().fg(ui.menu_title).bg(ui.menu_bg).add_modifier(Modifier::BOLD),
        );
        x += title.len() as u16 + 1;
    }

    for (idx, title) in crate::app::screens::MENU_TITLES.iter().enumerate() {
        let text = format!(" {} ", title);
        let style = if view.menu_active && view.menu_selected == idx {
            Style::default().fg(ui.menu_focus_fg).bg(ui.menu_focus_bg).add_modifier(Modifier::BOLD)
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

    if !view.menu_active {
        return;
    }

    let menu = view.menu_selected;
    let rows = crate::app::screens::menu_rows(menu);
    if rows.is_empty() {
        return;
    }
    let popup_x = area.x + 10 + (menu as u16 * 10);
    let popup_w = 28.min(area.width.saturating_sub(popup_x.saturating_sub(area.x)));
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
            if idx < screen.ui_areas.menu_popup_items.len() {
                screen.ui_areas.menu_popup_items[idx] = ClickArea {
                    x: popup.x + 1,
                    y,
                    width: popup.width.saturating_sub(2),
                    height: 1,
                };
            }
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
                line = format!("{:<width$} {}", truncate(&line, left_w), right, width = left_w);
            }
            buf.set_string(popup.x + 1, y, format!("{:<width$}", truncate(&line, content_w as usize), width = content_w as usize), style);
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
