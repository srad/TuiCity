use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
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
            AdvisorViewModel, BudgetViewModel, ConfirmDialogViewModel, NewsTickerViewModel,
            NewspaperViewModel, StatisticsWindowViewModel, TextWindowViewModel,
            ToolChooserViewModel, ToolbarPaletteViewModel,
        },
    },
};

fn to_rect(rect: RuntimeRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width, rect.height)
}

fn month_label(m: u8) -> &'static str {
    match m {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "???",
    }
}

fn to_click_area(rect: RuntimeRect) -> ClickArea {
    ClickArea {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn newspaper_card_inner(rect: Rect) -> Rect {
    Rect::new(
        rect.x.saturating_add(1),
        rect.y.saturating_add(1),
        rect.width.saturating_sub(2),
        rect.height.saturating_sub(2),
    )
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
        self.desktop_layout
            .as_ref()
            .expect("begin_frame must be called first")
    }

    fn window_border_type(&self) -> BorderType {
        if crate::ui::theme::is_pixel_style() {
            BorderType::Double
        } else {
            BorderType::Plain
        }
    }

    fn newspaper_palette(&self) -> (Color, Color, Color, Color, Color, Color, Color) {
        if crate::ui::theme::is_pixel_style() {
            (
                Color::Rgb(224, 220, 182),
                Color::Rgb(28, 22, 16),
                Color::Rgb(94, 84, 58),
                Color::Rgb(96, 32, 16),
                Color::Rgb(128, 116, 80),
                Color::Rgb(204, 194, 152),
                Color::Rgb(214, 204, 174),
            )
        } else {
            (
                Color::Rgb(242, 233, 214),
                Color::Rgb(52, 38, 24),
                Color::Rgb(109, 88, 64),
                Color::Rgb(105, 46, 24),
                Color::Rgb(149, 120, 88),
                Color::Rgb(221, 205, 173),
                Color::Rgb(236, 226, 203),
            )
        }
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
                .border_type(self.window_border_type())
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
                layout.viewport.x,
                layout.viewport.y,
                layout.viewport.width,
                layout.viewport.height,
            )),
            vertical_bar: to_click_area(RuntimeRect::new(
                layout.vertical_bar.x,
                layout.vertical_bar.y,
                layout.vertical_bar.width,
                layout.vertical_bar.height,
            )),
            vertical_dec: to_click_area(RuntimeRect::new(
                layout.vertical_dec.x,
                layout.vertical_dec.y,
                layout.vertical_dec.width,
                layout.vertical_dec.height,
            )),
            vertical_track: to_click_area(RuntimeRect::new(
                layout.vertical_track.x,
                layout.vertical_track.y,
                layout.vertical_track.width,
                layout.vertical_track.height,
            )),
            vertical_thumb: to_click_area(RuntimeRect::new(
                layout.vertical_thumb.x,
                layout.vertical_thumb.y,
                layout.vertical_thumb.width,
                layout.vertical_thumb.height,
            )),
            vertical_inc: to_click_area(RuntimeRect::new(
                layout.vertical_inc.x,
                layout.vertical_inc.y,
                layout.vertical_inc.width,
                layout.vertical_inc.height,
            )),
            horizontal_bar: to_click_area(RuntimeRect::new(
                layout.horizontal_bar.x,
                layout.horizontal_bar.y,
                layout.horizontal_bar.width,
                layout.horizontal_bar.height,
            )),
            horizontal_dec: to_click_area(RuntimeRect::new(
                layout.horizontal_dec.x,
                layout.horizontal_dec.y,
                layout.horizontal_dec.width,
                layout.horizontal_dec.height,
            )),
            horizontal_track: to_click_area(RuntimeRect::new(
                layout.horizontal_track.x,
                layout.horizontal_track.y,
                layout.horizontal_track.width,
                layout.horizontal_track.height,
            )),
            horizontal_thumb: to_click_area(RuntimeRect::new(
                layout.horizontal_thumb.x,
                layout.horizontal_thumb.y,
                layout.horizontal_thumb.width,
                layout.horizontal_thumb.height,
            )),
            horizontal_inc: to_click_area(RuntimeRect::new(
                layout.horizontal_inc.x,
                layout.horizontal_inc.y,
                layout.horizontal_inc.width,
                layout.horizontal_inc.height,
            )),
            corner: to_click_area(RuntimeRect::new(
                layout.corner.x,
                layout.corner.y,
                layout.corner.width,
                layout.corner.height,
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
            buf.set_string(
                x,
                area.y,
                " ",
                Style::default().fg(ui.menu_fg).bg(ui.menu_bg),
            );
        }

        let mut x = area.x;
        let title = format!(" {GAME_NAME} ");
        if x + title.len() as u16 <= area.x + area.width {
            self.frame.buffer_mut().set_string(
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

        // Left-aligned items (first 4)
        for (idx, menu_title) in crate::app::screens::MENU_TITLES.iter().take(4).enumerate() {
            let text = format!(" {} ", menu_title);
            let style = if menu_active && menu_selected == idx {
                Style::default()
                    .fg(ui.menu_focus_fg)
                    .bg(ui.menu_focus_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ui.menu_fg).bg(ui.menu_bg)
            };
            if x + text.len() as u16 <= area.x + area.width {
                areas.menu_items[idx] = ClickArea {
                    x,
                    y: area.y,
                    width: text.len() as u16,
                    height: 1,
                };
                self.frame.buffer_mut().set_string(x, area.y, &text, style);
            }
            x += text.len() as u16 + 1;
        }

        // Right-aligned items (remaining)
        let mut right_x = area.x + area.width;
        for (idx, menu_title) in crate::app::screens::MENU_TITLES
            .iter()
            .enumerate()
            .skip(4)
            .rev()
        {
            let text = format!(" {} ", menu_title);
            let text_w = text.len() as u16;
            if right_x < area.x + text_w {
                continue;
            }
            right_x = right_x.saturating_sub(text_w);
            let style = if menu_active && menu_selected == idx {
                Style::default()
                    .fg(ui.menu_focus_fg)
                    .bg(ui.menu_focus_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ui.menu_fg).bg(ui.menu_bg)
            };
            areas.menu_items[idx] = ClickArea {
                x: right_x,
                y: area.y,
                width: text_w,
                height: 1,
            };
            self.frame
                .buffer_mut()
                .set_string(right_x, area.y, &text, style);
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
        let popup_x = anchor
            .x
            .min(menu_area.x + menu_area.width.saturating_sub(popup_w.max(8)));
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
                .border_type(self.window_border_type())
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
                .border_type(self.window_border_type())
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
        let full_inner = Rect::new(
            panel_inner.x,
            panel_inner.y,
            panel_inner.width,
            full_inner_h,
        );
        let ph = full_inner.height;
        let desired_toolbar_h = game::toolbar::toolbar_height(toolbar);
        let minimum_toolbar_h = game::toolbar::minimum_toolbar_height(toolbar);
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

        let toolbar_area = panel_vert[0].intersection(self.area);
        let minimap_area = panel_vert[1].intersection(self.area);
        let info_area = panel_vert[2].intersection(self.area);

        if toolbar_area.width > 0 && toolbar_area.height > 0 {
            areas.toolbar_items =
                game::toolbar::render_toolbar(toolbar_area, self.frame.buffer_mut(), toolbar);
        }
        if minimap_area.width > 0 && minimap_area.height > 0 {
            let render_area =
                game::minimap::minimap_render_area(minimap_area, map.width, map.height);
            areas.minimap = to_click_area(RuntimeRect::new(
                render_area.x,
                render_area.y,
                render_area.width,
                render_area.height,
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
        let tile = if map.width > 0 && map.height > 0 {
            map.surface_lot_tile(cx, cy)
        } else {
            Tile::Grass
        };
        let tile_overlay = if map.width > 0 && map.height > 0 {
            map.get_overlay(cx, cy)
        } else {
            crate::core::map::TileOverlay::default()
        };

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

    fn paint_tool_chooser(&mut self, chooser: &ToolChooserViewModel) -> Vec<(ClickArea, Tool)> {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let power_outer = to_rect(dl.window(WindowId::PowerPicker).outer);
        let power_inner = to_rect(dl.window(WindowId::PowerPicker).inner);

        self.frame.render_widget(Clear, power_outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(self.window_border_type())
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
                .border_type(self.window_border_type())
                .title(ingame.desktop.window(WindowId::Budget).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.budget_window_bg)),
            budget_outer,
        );
        render_close_button(self.frame, budget_outer);
        game::budget::render_budget_content(self.frame.buffer_mut(), budget_inner, budget);
    }

    fn paint_statistics_window(
        &mut self,
        stats: &StatisticsWindowViewModel,
        ingame: &InGameScreen,
    ) {
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
                .border_type(self.window_border_type())
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
                        .border_type(self.window_border_type())
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
                    sim.plants
                        .iter()
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
                .border_type(self.window_border_type())
                .title(ingame.desktop.window(window_id).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            outer,
        );
        render_close_button(self.frame, outer);
        render_text_window_content(self.frame, &layout, view, ui.popup_bg, ui.text_primary);
    }

    fn paint_advisor_window(
        &mut self,
        advisor: &AdvisorViewModel,
        ingame: &crate::app::screens::InGameScreen,
    ) {
        let ui = crate::ui::theme::ui_palette();
        let dl = self.dl();
        let layout = dl.window(WindowId::Advisor);
        let outer = to_rect(layout.outer);

        if ingame.desktop.window(WindowId::Advisor).shadowed {
            render_window_shadow(self.frame, outer);
        }
        self.frame.render_widget(Clear, outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(self.window_border_type())
                .title(ingame.desktop.window(WindowId::Advisor).title)
                .title_style(Style::default().fg(ui.window_title))
                .border_style(Style::default().fg(ui.window_border))
                .style(Style::default().bg(ui.popup_bg)),
            outer,
        );
        render_close_button(self.frame, outer);

        let inner = to_rect(layout.padded_inner);
        if inner.width < 4 || inner.height < 4 {
            return;
        }

        let buf = self.frame.buffer_mut();

        // Tab row: [Economy] [City Planning] [Education] [Safety] [Transport]
        let domains = crate::textgen::types::AdvisorDomain::ALL;
        let mut tab_x = inner.x;
        for &domain in &domains {
            let label = domain.label();
            let is_sel = domain == advisor.domain;
            let style = if is_sel {
                Style::default()
                    .fg(ui.button_focus_fg)
                    .bg(ui.button_focus_bg)
                    .add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default().fg(ui.button_fg).bg(ui.button_bg)
            };
            let text = format!(" {} ", label);
            let w = text.len().min((inner.x + inner.width).saturating_sub(tab_x) as usize);
            if w == 0 {
                break;
            }
            buf.set_string(tab_x, inner.y, &text[..w], style);
            tab_x += w as u16 + 1;
        }

        // Body: advice text or "Thinking..."
        let body_y = inner.y + 2;
        let body_h = inner.height.saturating_sub(2);
        let text = if advisor.pending {
            "Thinking..."
        } else if let Some(ref t) = advisor.text {
            t.as_str()
        } else {
            "Press Enter to request advice, or ←/→ to switch domain."
        };

        for (i, line) in text.lines().enumerate() {
            if i as u16 >= body_h {
                break;
            }
            let truncated: String = line.chars().take(inner.width as usize).collect();
            buf.set_string(
                inner.x,
                body_y + i as u16,
                &truncated,
                Style::default().fg(ui.text_primary).bg(ui.popup_bg),
            );
        }
    }

    fn paint_newspaper_window(
        &mut self,
        newspaper: &NewspaperViewModel,
        ingame: &crate::app::screens::InGameScreen,
    ) -> Vec<ClickArea> {
        let dl = self.dl();
        let layout = dl.window(WindowId::Newspaper);
        let outer = to_rect(layout.outer);
        let (paper_bg, paper_fg, ink_dim, ink_accent, rule_fg, highlight_bg, feature_bg) =
            self.newspaper_palette();

        if ingame.desktop.window(WindowId::Newspaper).shadowed {
            render_window_shadow(self.frame, outer);
        }
        self.frame.render_widget(Clear, outer);
        self.frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(rule_fg))
                .style(Style::default().bg(paper_bg)),
            outer,
        );
        render_close_button(self.frame, outer);

        let inner = to_rect(layout.padded_inner);
        if inner.width < 4 || inner.height < 4 {
            return Vec::new();
        }

        let w = inner.width as usize;
        let text_style = Style::default().fg(paper_fg).bg(paper_bg);
        let dim_style = Style::default().fg(ink_dim).bg(paper_bg);
        let bold_style = Style::default()
            .fg(paper_fg)
            .bg(paper_bg)
            .add_modifier(Modifier::BOLD);
        let kicker_style = Style::default()
            .fg(ink_accent)
            .bg(paper_bg)
            .add_modifier(Modifier::BOLD);
        let divider_style = Style::default().fg(rule_fg).bg(paper_bg);
        let box_style = Style::default().fg(rule_fg).bg(paper_bg);
        let highlighted_box_style = Style::default().fg(rule_fg).bg(highlight_bg);

        // ── Masthead ──
        let month_name = month_label(newspaper.month);
        let masthead = format!("THE {} DAILY TRIBUNE", newspaper.city_name.to_uppercase(),);
        let date_line = format!(
            "{} {}  •  Morning Edition  •  City Desk",
            month_name, newspaper.year
        );

        let top_rule: String = "═".repeat(w);
        {
            let buf = self.frame.buffer_mut();
            buf.set_string(inner.x, inner.y, &top_rule, divider_style);
            let mast_row = inner.y + 1;
            let mast_len = masthead.len().min(w);
            let mast_x = inner.x + (inner.width.saturating_sub(mast_len as u16)) / 2;
            buf.set_string(mast_x, mast_row, &masthead[..mast_len], bold_style);
            let date_row = inner.y + 2;
            let date_len = date_line.len().min(w);
            let date_x = inner.x + (inner.width.saturating_sub(date_len as u16)) / 2;
            buf.set_string(date_x, date_row, &date_line[..date_len], dim_style);
            let rule_row = inner.y + 3;
            buf.set_string(inner.x, rule_row, &top_rule, divider_style);
        }

        if newspaper.pending {
            let msg = "The presses are rolling...";
            let body_y = inner.y + 6;
            let mx = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            self.frame.buffer_mut().set_string(
                mx,
                body_y + inner.height.saturating_sub(6) / 2,
                msg,
                dim_style,
            );
            return Vec::new();
        }

        if newspaper.pages.is_empty() {
            self.frame.buffer_mut().set_string(
                inner.x,
                inner.y + 6,
                "No edition available.",
                dim_style,
            );
            return Vec::new();
        }

        let current_page_index = newspaper
            .current_page
            .min(newspaper.pages.len().saturating_sub(1));
        let current_page = &newspaper.pages[current_page_index];
        let page_info = format!(
            "PAGE {}/{}  •  {}",
            current_page_index + 1,
            newspaper.pages.len(),
            current_page.title
        );
        {
            let buf = self.frame.buffer_mut();
            let info_len = page_info.len().min(w);
            let info_x = inner.x + (inner.width.saturating_sub(info_len as u16)) / 2;
            buf.set_string(info_x, inner.y + 4, &page_info[..info_len], kicker_style);

            let mut tab_x = inner.x;
            for (idx, page) in newspaper.pages.iter().enumerate() {
                if tab_x >= inner.x + inner.width {
                    break;
                }
                let short_title = page.title.chars().take(12).collect::<String>();
                let tab_label = format!(" {}:{} ", idx + 1, short_title);
                let visible = tab_label
                    .chars()
                    .take((inner.x + inner.width).saturating_sub(tab_x) as usize)
                    .collect::<String>();
                if visible.is_empty() {
                    break;
                }
                let tab_style = if idx == current_page_index {
                    Style::default()
                        .fg(paper_bg)
                        .bg(ink_accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    dim_style
                };
                buf.set_string(tab_x, inner.y + 5, visible.clone(), tab_style);
                tab_x = tab_x.saturating_add(visible.chars().count() as u16);
            }
            buf.set_string(inner.x, inner.y + 6, &top_rule, divider_style);
        }

        let body_y = inner.y + 7;
        let footer_y = inner.y + inner.height.saturating_sub(1);
        let body_h = inner.height.saturating_sub(8);
        let content_rect = Rect::new(inner.x, body_y, inner.width, body_h);

        if let Some(idx) = newspaper.detail_section_index {
            if let Some(section) = current_page.sections.get(idx) {
                let article_block = Block::default()
                    .title(format!(" {} • {} ", current_page.title, section.title))
                    .title_style(kicker_style)
                    .borders(Borders::ALL)
                    .border_style(box_style);
                self.frame.render_widget(article_block, content_rect);

                let article_inner = newspaper_card_inner(content_rect);
                self.frame.render_widget(
                    Paragraph::new(section.body.clone())
                        .style(text_style)
                        .wrap(Wrap { trim: false }),
                    article_inner,
                );

                let hint = "[Esc] Back  •  [←/→] Turn Page  •  [↑/↓] Pick Story";
                let hx = inner.x + inner.width.saturating_sub(hint.len() as u16);
                self.frame
                    .buffer_mut()
                    .set_string(hx, footer_y, hint, dim_style);
            }
            return Vec::new();
        }

        let mut click_areas = Vec::new();
        let is_feature_panel = |title: &str| {
            matches!(
                title,
                "CITY OWNER'S ADVERTISEMENT"
                    | "CONTACT ADS"
                    | "JOKE CORNER"
                    | "WEATHER DESK"
                    | "COMMUNITY CALENDAR"
            )
        };

        if current_page_index == 0 && !current_page.sections.is_empty() {
            let lead_height = content_rect.height.min(10);
            let lead_rect = Rect::new(
                content_rect.x,
                content_rect.y,
                content_rect.width,
                lead_height,
            );
            if let Some(section) = current_page.sections.first() {
                let selected = newspaper.selected_section_index == 0;
                let lead_bg = if selected { highlight_bg } else { paper_bg };
                let lead_block = Block::default()
                    .title(format!(" {} ", section.title))
                    .title_style(kicker_style)
                    .borders(Borders::ALL)
                    .border_style(if selected {
                        highlighted_box_style
                    } else {
                        box_style
                    })
                    .style(Style::default().bg(lead_bg));
                self.frame.render_widget(lead_block, lead_rect);
                self.frame.render_widget(
                    Paragraph::new(section.body.clone())
                        .style(
                            Style::default()
                                .fg(if selected { ink_accent } else { paper_fg })
                                .bg(lead_bg),
                        )
                        .wrap(Wrap { trim: false }),
                    newspaper_card_inner(lead_rect),
                );
                click_areas.push(ClickArea {
                    x: lead_rect.x,
                    y: lead_rect.y,
                    width: lead_rect.width,
                    height: lead_rect.height,
                });
            }

            let remaining_y = lead_rect.y + lead_rect.height + 1;
            let remaining_h = footer_y.saturating_sub(remaining_y);
            let others = current_page
                .sections
                .iter()
                .enumerate()
                .skip(1)
                .collect::<Vec<_>>();
            if !others.is_empty() && remaining_h >= 4 {
                let rows = ((others.len() + 1) / 2).max(1) as u16;
                let row_height = (remaining_h / rows).max(4);
                for (pos, (idx, section)) in others.into_iter().enumerate() {
                    let row = pos as u16 / 2;
                    let col = pos as u16 % 2;
                    let block_y = remaining_y + row * row_height;
                    if block_y >= footer_y {
                        break;
                    }
                    let block_h = if row + 1 == rows {
                        footer_y.saturating_sub(block_y)
                    } else {
                        row_height.saturating_sub(1)
                    }
                    .max(4);
                    let half_width = content_rect.width / 2;
                    let block_x = if col == 0 {
                        content_rect.x
                    } else {
                        content_rect.x + half_width
                    };
                    let block_w = if col == 0 {
                        half_width.saturating_sub(1)
                    } else {
                        content_rect.width.saturating_sub(half_width)
                    };
                    let rect = Rect::new(block_x, block_y, block_w, block_h);
                    let selected = newspaper.selected_section_index == idx;
                    let card_bg = if selected {
                        highlight_bg
                    } else if is_feature_panel(&section.title) {
                        feature_bg
                    } else {
                        paper_bg
                    };
                    let block = Block::default()
                        .title(format!(" {} ", section.title))
                        .title_style(kicker_style)
                        .borders(Borders::ALL)
                        .border_style(if selected {
                            highlighted_box_style
                        } else {
                            box_style
                        })
                        .style(Style::default().bg(card_bg));
                    self.frame.render_widget(block, rect);
                    self.frame.render_widget(
                        Paragraph::new(section.body.clone())
                            .style(Style::default().fg(paper_fg).bg(card_bg))
                            .wrap(Wrap { trim: false }),
                        newspaper_card_inner(rect),
                    );
                    click_areas.push(ClickArea {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height,
                    });
                }
            }
        } else {
            let rows = ((current_page.sections.len() + 1) / 2).max(1) as u16;
            let row_height = (content_rect.height / rows).max(5);
            for (pos, section) in current_page.sections.iter().enumerate() {
                let row = pos as u16 / 2;
                let col = pos as u16 % 2;
                let block_y = content_rect.y + row * row_height;
                if block_y >= footer_y {
                    break;
                }
                let block_h = if row + 1 == rows {
                    footer_y.saturating_sub(block_y)
                } else {
                    row_height.saturating_sub(1)
                }
                .max(5);
                let half_width = content_rect.width / 2;
                let block_x = if col == 0 {
                    content_rect.x
                } else {
                    content_rect.x + half_width
                };
                let block_w = if col == 0 {
                    half_width.saturating_sub(1)
                } else {
                    content_rect.width.saturating_sub(half_width)
                };
                let rect = Rect::new(block_x, block_y, block_w, block_h);
                let selected = newspaper.selected_section_index == pos;
                let card_bg = if selected {
                    highlight_bg
                } else if is_feature_panel(&section.title) {
                    feature_bg
                } else {
                    paper_bg
                };
                let block = Block::default()
                    .title(format!(" {} ", section.title))
                    .title_style(kicker_style)
                    .borders(Borders::ALL)
                    .border_style(if selected {
                        highlighted_box_style
                    } else {
                        box_style
                    })
                    .style(Style::default().bg(card_bg));
                self.frame.render_widget(block, rect);
                self.frame.render_widget(
                    Paragraph::new(section.body.clone())
                        .style(Style::default().fg(paper_fg).bg(card_bg))
                        .wrap(Wrap { trim: false }),
                    newspaper_card_inner(rect),
                );
                click_areas.push(ClickArea {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                });
            }
        }

        let hint = "Page with ←/→  •  Select with ↑/↓  •  Enter or click to read  •  Esc closes";
        let hx = inner.x + (inner.width.saturating_sub(hint.len() as u16)) / 2;
        self.frame
            .buffer_mut()
            .set_string(hx, footer_y, hint, dim_style);

        click_areas
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

    // ── Advisor tab-row arithmetic ───────────────────────────────────────────

    /// The tab remaining-width expression in paint_advisor_window is:
    ///   `inner.width - tab_x + inner.x`
    /// Because Rust evaluates left-to-right this computes `(inner.width - tab_x)`
    /// first.  On a 40-column terminal `inner.width` is 34 and after rendering
    /// "Economy" + "City Planning" + the partial "Education" tab, `tab_x`
    /// reaches 38.  `34u16 - 38u16` underflows and panics in debug mode.
    ///
    /// This test directly reproduces that arithmetic to confirm the overflow.
    #[test]
    #[should_panic]
    fn advisor_tab_remaining_overflows_with_buggy_formula() {
        // Values that occur on a 40-column terminal:
        //   padded_inner.x = 3, padded_inner.width = 34
        //   tab_x after Economy(9) + City Planning(15) + partial Education(8):
        //     3 + 9+1 + 15+1 + 8+1 = 38
        //
        // black_box prevents compile-time constant-folding so the overflow
        // manifests as a runtime panic (exactly as it does in the real game).
        let inner_width: u16 = std::hint::black_box(34);
        let inner_x: u16 = std::hint::black_box(3);
        let tab_x: u16 = std::hint::black_box(38);

        // BUG: (inner_width - tab_x) underflows before + inner_x can rescue it.
        let _ = (inner_width - tab_x + inner_x) as usize;
    }

    /// Helper that mirrors what the corrected expression should be.
    /// `(inner_x + inner_width).saturating_sub(tab_x)` evaluates the right-edge
    /// first and can never underflow.
    #[test]
    fn advisor_tab_remaining_correct_formula_returns_zero_when_exhausted() {
        let inner_width: u16 = std::hint::black_box(34);
        let inner_x: u16 = std::hint::black_box(3);
        let tab_x: u16 = std::hint::black_box(38); // beyond right edge

        let remaining = (inner_x + inner_width).saturating_sub(tab_x) as usize;
        assert_eq!(remaining, 0);
    }

    #[test]
    fn advisor_tab_remaining_correct_formula_returns_space_when_within_bounds() {
        let inner_width: u16 = std::hint::black_box(34);
        let inner_x: u16 = std::hint::black_box(3);
        let tab_x: u16 = std::hint::black_box(29); // after Economy + City Planning

        // Right edge = inner_x + inner_width = 37; remaining = 37 - 29 = 8
        let remaining = (inner_x + inner_width).saturating_sub(tab_x) as usize;
        assert_eq!(remaining, 8);
    }

    // ── Full rendering smoke tests ───────────────────────────────────────────

    fn render_advisor_at_size(cols: u16, rows: u16) {
        use ratatui::{backend::TestBackend, Terminal};

        let backend = TestBackend::new(cols, rows);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut ingame = InGameScreen::new();
        ingame.open_advisor_window();

        // Compute the desktop layout before entering the draw closure so that
        // `ingame` can be borrowed immutably inside.
        let area = ratatui::layout::Rect::new(0, 0, cols, rows);
        let full =
            crate::ui::runtime::UiRect::new(area.x, area.y, area.width, area.height);
        let desktop_layout = ingame.desktop.layout(full);

        terminal
            .draw(|frame| {
                let mut painter = TerminalPainter::new(frame, area);
                let frame_layout = FrameLayout {
                    desktop_layout: desktop_layout.clone(),
                    view_w: 0,
                    view_h: 0,
                    col_scale: 1,
                };
                painter.begin_frame(&frame_layout);
                let advisor = AdvisorViewModel {
                    domain: crate::textgen::types::AdvisorDomain::Economy,
                    text: None,
                    pending: false,
                };
                painter.paint_advisor_window(&advisor, &ingame);
            })
            .unwrap();
    }

    /// Clicking "Advisors" on a wide terminal must not crash.
    #[test]
    fn advisor_window_renders_on_wide_terminal() {
        render_advisor_at_size(120, 40);
    }

    /// Clicking "Advisors" on a narrow terminal (40 cols) triggers the u16
    /// underflow in the tab row and panics before the fix is applied.
    #[test]
    fn advisor_window_renders_on_narrow_terminal() {
        render_advisor_at_size(40, 24);
    }

    /// Even a borderline width just under the default window size must work.
    #[test]
    fn advisor_window_renders_on_medium_terminal() {
        render_advisor_at_size(60, 30);
    }
}
