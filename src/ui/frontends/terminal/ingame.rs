use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{camera::Camera, screens::InGameScreen, ClickArea, MapUiAreas, WindowId},
    core::{
        map::{Map, Tile, ViewLayer},
        sim::SimState,
        tool::Tool,
    },
    game_info::GAME_NAME,
    ui::{
        frontends::terminal::render_confirm_dialog,
        game,
        painter::{
            FrameLayout, InGamePainter, MapPreview, MenuBarAreas, MenuPopupAreas, PanelAreas,
            StatusBarAreas,
        },
        runtime::{DesktopLayout, UiRect as RuntimeRect},
        theme::OverlayMode,
        view::{
            BudgetViewModel, ConfirmDialogViewModel, NewsTickerViewModel,
            StatisticsWindowViewModel, TextWindowViewModel, ToolChooserViewModel,
            ToolbarPaletteViewModel,
        },
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

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

// ── TerminalPainter ──────────────────────────────────────────────────────────

pub struct TerminalPainter<'a, 'f> {
    frame: &'a mut Frame<'f>,
    area: Rect,
    // Computed in begin_frame
    desktop_layout: Option<DesktopLayout>,
    map_layout: Option<game::map_view::MapChromeLayout>,
}

impl<'a, 'f> TerminalPainter<'a, 'f> {
    pub fn new(frame: &'a mut Frame<'f>, area: Rect) -> Self {
        Self {
            frame,
            area,
            desktop_layout: None,
            map_layout: None,
        }
    }

    fn dl(&self) -> &DesktopLayout {
        self.desktop_layout.as_ref().expect("begin_frame must be called first")
    }
}

impl<'a, 'f> InGamePainter for TerminalPainter<'a, 'f> {
    fn begin_frame(&mut self, layout: &FrameLayout) {
        self.desktop_layout = Some(layout.desktop_layout.clone());
        let ui = crate::ui::theme::ui_palette();
        self.frame.render_widget(
            Block::default().style(Style::default().bg(ui.desktop_bg)),
            self.area,
        );
    }

    fn paint_map(
        &mut self,
        map: &Map,
        camera: &Camera,
        overlay_mode: OverlayMode,
        view_layer: ViewLayer,
        current_tool: Tool,
        preview: MapPreview<'_>,
    ) -> MapUiAreas {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let map_outer = to_rect(dl.window(WindowId::Map).outer);
        let map_inner = to_rect(dl.window(WindowId::Map).inner);

        let layout = game::map_view::layout_map_chrome(
            map_inner,
            map.width,
            map.height,
            camera.offset_x.max(0) as usize,
            camera.offset_y.max(0) as usize,
        );

        // Draw map window chrome
        self.frame.render_widget(Clear, map_outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title("Map")
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.map_window_bg)),
            map_outer,
        );

        if layout.viewport.width > 0 && layout.viewport.height > 0 {
            use game::map_view::PreviewKind;

            let (preview_tiles_owned, preview_kind) = match preview {
                MapPreview::Rect(tiles) => (tiles, PreviewKind::Rect(current_tool)),
                MapPreview::Line(tiles) => (tiles, PreviewKind::Line(current_tool)),
                MapPreview::Footprint(tiles, valid) => {
                    (tiles, PreviewKind::Footprint(current_tool, valid))
                }
                MapPreview::None => (&[] as &[(usize, usize)], PreviewKind::None),
            };

            self.frame.render_widget(
                game::map_view::MapView {
                    map,
                    camera,
                    line_preview: preview_tiles_owned,
                    preview_kind,
                    overlay_mode,
                    view_layer,
                },
                layout.viewport,
            );
            game::map_view::render_scrollbars(&layout, self.frame.buffer_mut());
        }

        let map_areas = MapUiAreas {
            viewport: to_click_area(RuntimeRect::new(
                layout.viewport.x, layout.viewport.y,
                layout.viewport.width, layout.viewport.height,
            )),
            vertical_bar: to_click_area(RuntimeRect::new(
                layout.vertical_bar.x, layout.vertical_bar.y,
                layout.vertical_bar.width, layout.vertical_bar.height,
            )),
            vertical_dec: to_click_area(RuntimeRect::new(
                layout.vertical_dec.x, layout.vertical_dec.y,
                layout.vertical_dec.width, layout.vertical_dec.height,
            )),
            vertical_track: to_click_area(RuntimeRect::new(
                layout.vertical_track.x, layout.vertical_track.y,
                layout.vertical_track.width, layout.vertical_track.height,
            )),
            vertical_thumb: to_click_area(RuntimeRect::new(
                layout.vertical_thumb.x, layout.vertical_thumb.y,
                layout.vertical_thumb.width, layout.vertical_thumb.height,
            )),
            vertical_inc: to_click_area(RuntimeRect::new(
                layout.vertical_inc.x, layout.vertical_inc.y,
                layout.vertical_inc.width, layout.vertical_inc.height,
            )),
            horizontal_bar: to_click_area(RuntimeRect::new(
                layout.horizontal_bar.x, layout.horizontal_bar.y,
                layout.horizontal_bar.width, layout.horizontal_bar.height,
            )),
            horizontal_dec: to_click_area(RuntimeRect::new(
                layout.horizontal_dec.x, layout.horizontal_dec.y,
                layout.horizontal_dec.width, layout.horizontal_dec.height,
            )),
            horizontal_track: to_click_area(RuntimeRect::new(
                layout.horizontal_track.x, layout.horizontal_track.y,
                layout.horizontal_track.width, layout.horizontal_track.height,
            )),
            horizontal_thumb: to_click_area(RuntimeRect::new(
                layout.horizontal_thumb.x, layout.horizontal_thumb.y,
                layout.horizontal_thumb.width, layout.horizontal_thumb.height,
            )),
            horizontal_inc: to_click_area(RuntimeRect::new(
                layout.horizontal_inc.x, layout.horizontal_inc.y,
                layout.horizontal_inc.width, layout.horizontal_inc.height,
            )),
            corner: to_click_area(RuntimeRect::new(
                layout.corner.x, layout.corner.y,
                layout.corner.width, layout.corner.height,
            )),
        };

        self.map_layout = Some(layout);
        map_areas
    }

    fn paint_menu_bar(
        &mut self,
        menu_active: bool,
        menu_selected: usize,
        _menu_item_selected: usize,
    ) -> MenuBarAreas {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let area = to_rect(dl.menu_bar);
        let mut areas = MenuBarAreas::default();

        areas.menu_bar = ClickArea {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
        };

        // Fill background
        let buf = self.frame.buffer_mut();
        for x in area.x..area.x + area.width {
            buf.set_string(x, area.y, " ", Style::default().fg(ui.menu_fg).bg(ui.menu_bg));
        }

        let mut x = area.x;
        let title = format!(" {GAME_NAME} ");
        if x + title.len() as u16 <= area.x + area.width {
            self.frame.buffer_mut().set_string(
                x, area.y, &title,
                Style::default().fg(ui.menu_title).bg(ui.menu_bg).add_modifier(Modifier::BOLD),
            );
            x += title.len() as u16 + 1;
        }

        // Left-aligned items (first 4)
        for (idx, menu_title) in crate::app::screens::MENU_TITLES.iter().take(4).enumerate() {
            let text = format!(" {} ", menu_title);
            let style = if menu_active && menu_selected == idx {
                Style::default().fg(ui.menu_focus_fg).bg(ui.menu_focus_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ui.menu_fg).bg(ui.menu_bg)
            };
            if x + text.len() as u16 <= area.x + area.width {
                areas.menu_items[idx] = ClickArea { x, y: area.y, width: text.len() as u16, height: 1 };
                self.frame.buffer_mut().set_string(x, area.y, &text, style);
            }
            x += text.len() as u16 + 1;
        }

        // Right-aligned items (remaining)
        let mut right_x = area.x + area.width;
        for (idx, menu_title) in crate::app::screens::MENU_TITLES.iter().enumerate().skip(4).rev() {
            let text = format!(" {} ", menu_title);
            let text_w = text.len() as u16;
            if right_x < area.x + text_w { continue; }
            right_x = right_x.saturating_sub(text_w);
            let style = if menu_active && menu_selected == idx {
                Style::default().fg(ui.menu_focus_fg).bg(ui.menu_focus_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ui.menu_fg).bg(ui.menu_bg)
            };
            areas.menu_items[idx] = ClickArea { x: right_x, y: area.y, width: text_w, height: 1 };
            self.frame.buffer_mut().set_string(right_x, area.y, &text, style);
            right_x = right_x.saturating_sub(1);
        }

        areas
    }

    fn paint_menu_popup(
        &mut self,
        menu_selected: usize,
        menu_item_selected: usize,
        anchor: ClickArea,
    ) -> MenuPopupAreas {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let menu_area = to_rect(dl.menu_bar);
        let mut areas = MenuPopupAreas::default();

        let rows = crate::app::screens::menu_rows(menu_selected);
        if rows.is_empty() {
            return areas;
        }

        let popup_w = 28.min(menu_area.width.max(8));
        let popup_x = anchor.x.min(menu_area.x + menu_area.width.saturating_sub(popup_w.max(8)));
        let popup_h = rows.len() as u16 + 2;
        let popup = Rect::new(popup_x, menu_area.y + 1, popup_w.max(8), popup_h);

        areas.menu_popup = ClickArea {
            x: popup.x,
            y: popup.y,
            width: popup.width,
            height: popup.height,
        };

        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ui.menu_fg).bg(ui.menu_bg))
                .style(Style::default().bg(ui.menu_bg)),
            popup,
        );

        let buf = self.frame.buffer_mut();
        for (idx, row) in rows.iter().enumerate() {
            let y = popup.y + 1 + idx as u16;
            areas.menu_popup_items.push(ClickArea {
                x: popup.x + 1,
                y,
                width: popup.width.saturating_sub(2),
                height: 1,
            });
            let selected = idx == menu_item_selected;
            let style = if selected {
                Style::default().fg(ui.menu_focus_fg).bg(ui.menu_focus_bg)
            } else {
                Style::default().fg(ui.menu_fg).bg(ui.menu_bg)
            };
            let content_w = popup.width.saturating_sub(2);
            let mut line = row.label.to_string();
            if !row.right.is_empty() {
                let right_w = row.right.chars().count() as u16;
                let left_w = content_w.saturating_sub(right_w + 1) as usize;
                line = format!(
                    "{:<width$} {}",
                    truncate(&line, left_w),
                    row.right,
                    width = left_w
                );
            }
            buf.set_string(
                popup.x + 1, y,
                format!("{:<width$}", truncate(&line, content_w as usize), width = content_w as usize),
                style,
            );
        }

        areas
    }

    fn paint_status_bar(
        &mut self,
        sim: &SimState,
        paused: bool,
        view_layer: ViewLayer,
        status_message: Option<&str>,
    ) -> StatusBarAreas {
        let dl = self.dl();
        let status_area = to_rect(dl.status_bar);
        game::statusbar::render_statusbar(
            status_area,
            self.frame.buffer_mut(),
            sim,
            paused,
            view_layer,
            status_message,
        )
    }

    fn paint_panel_window(
        &mut self,
        toolbar: &ToolbarPaletteViewModel,
        current_tool: Tool,
        sim: &SimState,
        _inspect_pos: Option<(usize, usize)>,
        map: &Map,
        ingame: &InGameScreen,
    ) -> PanelAreas {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let panel_outer = to_rect(dl.window(WindowId::Panel).outer);
        let panel_inner = to_rect(dl.window(WindowId::Panel).inner);
        let mut areas = PanelAreas::default();

        if panel_outer.width == 0 || panel_outer.height == 0 {
            return areas;
        }

        if ingame.desktop.window(WindowId::Panel).shadowed {
            render_window_shadow(self.frame, panel_outer);
        }
        self.frame.render_widget(Clear, panel_outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(ingame.desktop.window(WindowId::Panel).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.panel_window_bg)),
            panel_outer,
        );
        render_close_button(self.frame, panel_outer);

        if panel_inner.width == 0 {
            return areas;
        }

        let panel_window = ingame.desktop.window(WindowId::Panel);
        let full_inner_h = panel_window.height.saturating_sub(2).max(4);
        let full_inner = Rect::new(panel_inner.x, panel_inner.y, panel_inner.width, full_inner_h);
        let ph = full_inner.height;
        let desired_toolbar_h = game::toolbar::toolbar_height(toolbar);
        let minimum_toolbar_h = game::toolbar::minimum_toolbar_height(toolbar);
        let minimum_minimap_h = 8;
        let minimum_info_h = 5;
        let max_toolbar_h = ph.saturating_sub(minimum_minimap_h + minimum_info_h);
        let toolbar_h = desired_toolbar_h.min(max_toolbar_h.max(minimum_toolbar_h)).min(ph);
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
        ]).split(full_inner);

        let toolbar_area = panel_vert[0].intersection(self.area);
        let minimap_area = panel_vert[1].intersection(self.area);
        let info_area = panel_vert[2].intersection(self.area);

        if toolbar_area.width > 0 && toolbar_area.height > 0 {
            areas.toolbar_items =
                game::toolbar::render_toolbar(toolbar_area, self.frame.buffer_mut(), toolbar);
        }
        if minimap_area.width > 0 && minimap_area.height > 0 {
            let render_area = game::minimap::minimap_render_area(minimap_area, map.width, map.height);
            areas.minimap = to_click_area(RuntimeRect::new(
                render_area.x, render_area.y, render_area.width, render_area.height,
            ));
            self.frame.render_widget(
                game::minimap::MiniMap {
                    map,
                    camera: &ingame.camera,
                    overlay_mode: OverlayMode::None,
                    view_layer: ViewLayer::Surface,
                },
                minimap_area,
            );
        }

        let cx = ingame.camera.cursor_x.min(map.width.saturating_sub(1));
        let cy = ingame.camera.cursor_y.min(map.height.saturating_sub(1));
        let tile = if map.width > 0 && map.height > 0 { map.surface_lot_tile(cx, cy) } else { Tile::Grass };
        let tile_overlay = if map.width > 0 && map.height > 0 { map.get_overlay(cx, cy) } else { crate::core::map::TileOverlay::default() };

        if info_area.width > 0 && info_area.height > 0 {
            self.frame.render_widget(
                game::infopanel::InfoPanel {
                    tile,
                    overlay: tile_overlay,
                    zone: map.effective_zone_kind(cx, cy),
                    x: cx,
                    y: cy,
                    current_tool,
                    demand_res: sim.demand.res,
                    demand_comm: sim.demand.comm,
                    demand_ind: sim.demand.ind,
                    demand_history_res: sim.history.demand_res.clone(),
                    demand_history_comm: sim.history.demand_comm.clone(),
                    demand_history_ind: sim.history.demand_ind.clone(),
                    power_produced: sim.utilities.power_produced_mw,
                    power_consumed: sim.utilities.power_consumed_mw,
                },
                info_area,
            );
        }

        areas
    }

    fn paint_tool_chooser(&mut self, chooser: &ToolChooserViewModel) -> Vec<ClickArea> {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let power_outer = to_rect(dl.window(WindowId::PowerPicker).outer);
        let power_inner = to_rect(dl.window(WindowId::PowerPicker).inner);

        self.frame.render_widget(Clear, power_outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title("Tool Selection")
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            power_outer,
        );
        render_close_button(self.frame, power_outer);
        game::power_popup::render_tool_chooser_content(
            self.frame.buffer_mut(),
            power_inner,
            chooser,
        )
    }

    fn paint_confirm_dialog(&mut self, dialog: &ConfirmDialogViewModel) -> Vec<ClickArea> {
        render_confirm_dialog(self.frame, self.area, dialog)
    }

    fn paint_budget_window(&mut self, budget: &BudgetViewModel, ingame: &InGameScreen) {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let budget_outer = to_rect(dl.window(WindowId::Budget).outer);
        let budget_inner = to_rect(dl.window(WindowId::Budget).inner);

        if ingame.desktop.window(WindowId::Budget).shadowed {
            render_window_shadow(self.frame, budget_outer);
        }
        self.frame.render_widget(Clear, budget_outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(ingame.desktop.window(WindowId::Budget).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.budget_window_bg)),
            budget_outer,
        );
        render_close_button(self.frame, budget_outer);
        game::budget::render_budget_content(self.frame.buffer_mut(), budget_inner, budget);
    }

    fn paint_statistics_window(&mut self, stats: &StatisticsWindowViewModel, ingame: &InGameScreen) {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let statistics_outer = to_rect(dl.window(WindowId::Statistics).outer);
        let statistics_inner = to_rect(dl.window(WindowId::Statistics).inner);

        if ingame.desktop.window(WindowId::Statistics).shadowed {
            render_window_shadow(self.frame, statistics_outer);
        }
        self.frame.render_widget(Clear, statistics_outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(ingame.desktop.window(WindowId::Statistics).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            statistics_outer,
        );
        render_close_button(self.frame, statistics_outer);
        game::statistics::render_statistics_content(self.frame, statistics_inner, stats);
    }

    fn paint_inspect_window(
        &mut self,
        inspect_pos: Option<(usize, usize)>,
        map: &Map,
        sim: &SimState,
        ingame: &InGameScreen,
    ) {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let inspect_outer = to_rect(dl.window(WindowId::Inspect).outer);
        let inspect_inner = to_rect(dl.window(WindowId::Inspect).inner);

        if let Some(pos) = inspect_pos {
            if pos.0 < map.width && pos.1 < map.height {
                let title = format!(" Inspect ({},{}) ", pos.0, pos.1);
                if ingame.desktop.window(WindowId::Inspect).shadowed {
                    render_window_shadow(self.frame, inspect_outer);
                }
                self.frame.render_widget(Clear, inspect_outer);
                self.frame.render_widget(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title.as_str())
                        .title_style(Style::default().fg(ui.window_title))
                        .border_style(Style::default().fg(ui.window_border))
                        .style(Style::default().bg(ui.inspect_window_bg)),
                    inspect_outer,
                );
                render_close_button(self.frame, inspect_outer);

                let plant_info = if matches!(
                    map.surface_lot_tile(pos.0, pos.1),
                    Tile::PowerPlantCoal | Tile::PowerPlantGas
                ) {
                    sim.plants.iter()
                        .find(|(&(px, py), _)| {
                            pos.0 >= px && pos.0 < px + 4 && pos.1 >= py && pos.1 < py + 4
                        })
                        .map(|(_, state)| game::inspect_popup::PlantInfo::from_state(state))
                } else {
                    None
                };

                game::inspect_popup::render_inspect_content(
                    self.frame.buffer_mut(),
                    inspect_inner,
                    pos,
                    map,
                    plant_info,
                );
            }
        }
    }

    fn paint_text_window(
        &mut self,
        window_id: WindowId,
        view: &TextWindowViewModel,
        ingame: &mut crate::app::screens::InGameScreen,
    ) {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let layout = dl.window(window_id);

        if ingame.desktop.window(window_id).shadowed {
            render_window_shadow(self.frame, to_rect(layout.outer));
        }
        let outer = to_rect(layout.outer);
        self.frame.render_widget(Clear, outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(ingame.desktop.window(window_id).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            outer,
        );
        render_close_button(self.frame, outer);
        render_text_window_content(self.frame, &layout, view, ui.popup_bg, ui.text_primary);
    }

    fn paint_news_ticker(&mut self, ticker: &NewsTickerViewModel) {
        let dl = self.dl();
        let news_area = to_rect(dl.news_ticker);
        game::news_ticker::render_news_ticker(news_area, self.frame.buffer_mut(), ticker);
    }

    fn end_frame(&mut self) {
        // No-op — ratatui auto-presents
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
