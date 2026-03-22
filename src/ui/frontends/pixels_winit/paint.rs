use crate::{
    app::{camera::Camera, screens::InGameScreen, ClickArea, MapUiAreas},
    core::{
        map::{Map, TileOverlay, ViewLayer},
        sim::SimState,
        tool::Tool,
    },
    ui::{
        painter::{
            FrameLayout, InGamePainter, MapPreview, MenuBarAreas, MenuPopupAreas, PanelAreas,
            StatusBarAreas,
        },
        runtime::{ToolbarHitArea, ToolbarHitTarget, ToolChooserKind, UiRect, WindowId},
        theme::{self, OverlayMode},
        view::{
            BudgetViewModel, ConfirmDialogViewModel, NewsTickerViewModel, ScreenView,
            SettingsViewModel, StartViewModel, StatisticsWindowViewModel, TextWindowViewModel,
            ToolChooserViewModel, ToolbarPaletteViewModel,
        },
    },
};

use super::font::{
    self, cell_h, cell_w, color_to_u32, draw_rect_outline, draw_str, fill_rect, rgb_to_u32,
};
use super::tiles;

// ── Palette constants (0x00RRGGBB) ────────────────────────────────────────────

const BG: u32 = 0x14141e;
const TEXT_FG: u32 = 0xdcd7d2;
const TITLE_FG: u32 = 0xffdd77;
const SELECT_BG: u32 = 0xffdd77;
const SELECT_FG: u32 = 0x1c1c2a;
const BORDER_FG: u32 = 0x6ae2e1;
const STATUS_BG: u32 = 0x1e1c32;
const SIDEBAR_BG: u32 = 0x1a1826;
const DIM_FG: u32 = 0x787370;
const BTN_BG: u32 = 0x2a2440;
const BTN_SEL_BG: u32 = 0x3a3460;
const RES_FG: u32 = 0x66cc44;
const COMM_FG: u32 = 0x4488ff;
const IND_FG: u32 = 0xffcc44;
const POWER_FG: u32 = 0xffa040;
const MINIMAP_FRAME: u32 = 0x3a3460;
const MENU_BG: u32 = 0x28243c;
const MENU_FG: u32 = 0xd0cce0;
const MENU_FOCUS_BG: u32 = 0x6ae2e1;
const MENU_FOCUS_FG: u32 = 0x1a1826;
const WIN_TITLE_BG: u32 = 0x2a2060;
const WIN_TITLE_FG: u32 = 0x6ae2e1;
const POPUP_BG: u32 = 0x1c1828;
const DANGER_FG: u32 = 0xff4040;
const SHADOW_COLOR: u32 = 0x0a0810;

// ── Animation helpers ──────────────────────────────────────────────────────────

fn anim_phase(step_ms: u128, period: u32) -> u32 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        / step_ms
        % period as u128) as u32
}

// ── Top-level dispatch ─────────────────────────────────────────────────────────

/// Paint a complete frame into `buf` (u32 pixels, `0x00RRGGBB`, row-major, `width × height`).
pub fn paint_frame(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    view: &ScreenView,
    ingame: Option<&mut InGameScreen>,
) {
    fill_rect(buf, width, 0, 0, width, height, BG);

    match view {
        ScreenView::Start(v) => paint_start(buf, width, height, scale, v),
        ScreenView::Settings(v) => paint_settings(buf, width, height, scale, v),
        ScreenView::LoadCity(v) => {
            let lines: Vec<String> = v.saves.iter().map(|s| s.city_name.clone()).collect();
            paint_list(buf, width, height, scale, "Load City", &lines, v.selected);
        }
        ScreenView::NewCity(_) => {
            paint_simple_text(
                buf,
                width,
                height,
                scale,
                "New City  --  use Terminal mode to configure",
            );
        }
        ScreenView::ThemeSettings(v) => {
            let labels: Vec<String> = v.themes.iter().map(|t| t.label().to_string()).collect();
            paint_list(buf, width, height, scale, "Theme Settings", &labels, v.selected);
        }
        ScreenView::InGame(v) => {
            let ingame = ingame.expect("InGame view requires InGameScreen reference");
            let cw = cell_w(scale);
            let ch = cell_h(scale);
            let total_cols = (width / cw) as u16;
            let total_rows = (height / ch) as u16;
            let desktop_layout = ingame.desktop.layout(UiRect::new(0, 0, total_cols, total_rows));
            // Pixel layout: menu bar + status bar at top, ticker at bottom
            let menu_h = ch;
            let status_h = ch;
            let ticker_h = ch;
            let map_px_y = menu_h + status_h;
            let map_px_h = height.saturating_sub(map_px_y + ticker_h);
            let tiles_cols = (width / cw) as usize;
            let tiles_rows = (map_px_h / ch) as usize;
            let layout = FrameLayout {
                desktop_layout,
                view_w: tiles_cols.max(1),
                view_h: tiles_rows.max(1),
                col_scale: 1,
            };
            let mut painter = PixelPainter::new(buf, width, height, scale);
            crate::ui::painter::orchestrate_ingame(&mut painter, v, ingame, layout);
        }
    }
}

// ── Start screen ───────────────────────────────────────────────────────────────

fn paint_start(buf: &mut [u32], width: u32, height: u32, scale: u32, view: &StartViewModel) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let title = "T U I C I T Y  2 0 0 0";
    let subtitle = "A city simulation";

    draw_str(
        buf,
        width,
        center_x(width, title.len() as u32, scale),
        ch,
        title,
        TITLE_FG,
        BG,
        scale,
    );
    draw_str(
        buf,
        width,
        center_x(width, subtitle.len() as u32, scale),
        ch * 2,
        subtitle,
        DIM_FG,
        BG,
        scale,
    );

    font::hline(buf, width, 0, ch * 3 + ch / 2, width, BORDER_FG);

    let start_y = ch * 5;
    for (i, option) in view.options.iter().enumerate() {
        let selected = i == view.selected;
        let row_y = start_y + i as u32 * ch * 2;
        let text = format!("  {}  ", option);
        let tx = center_x(width, text.len() as u32, scale);

        if selected {
            fill_rect(buf, width, tx, row_y, text.len() as u32 * cw, ch, SELECT_BG);
            draw_str(buf, width, tx, row_y, &text, SELECT_FG, SELECT_BG, scale);
        } else {
            draw_str(buf, width, tx, row_y, &text, TEXT_FG, BG, scale);
        }
    }

    let hint = "Arrow Keys / Enter";
    draw_str(
        buf,
        width,
        center_x(width, hint.len() as u32, scale),
        height.saturating_sub(ch * 2),
        hint,
        DIM_FG,
        BG,
        scale,
    );
}

// ── Settings screen ────────────────────────────────────────────────────────────

fn paint_settings(buf: &mut [u32], width: u32, height: u32, scale: u32, view: &SettingsViewModel) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let title = "Settings";
    draw_str(
        buf,
        width,
        center_x(width, title.len() as u32, scale),
        ch,
        title,
        TITLE_FG,
        BG,
        scale,
    );

    let info = format!(
        "Theme: {}  |  Renderer: {}",
        view.current_theme_label, view.current_frontend_label
    );
    draw_str(
        buf,
        width,
        center_x(width, info.len() as u32, scale),
        ch * 2,
        &info,
        DIM_FG,
        BG,
        scale,
    );

    font::hline(buf, width, 0, ch * 3, width, BORDER_FG);

    let start_y = ch * 5;
    for (i, option) in view.options.iter().enumerate() {
        let selected = i == view.selected;
        let row_y = start_y + i as u32 * ch * 2;
        let text = format!("  {}  ", option);
        let ox = center_x(width, text.len() as u32, scale);

        if selected {
            fill_rect(buf, width, ox, row_y, text.len() as u32 * cw, ch, SELECT_BG);
            draw_str(buf, width, ox, row_y, &text, SELECT_FG, SELECT_BG, scale);
        } else {
            draw_str(buf, width, ox, row_y, &text, TEXT_FG, BG, scale);
        }
    }

    let hint = "Arrow Keys / Enter / Esc";
    draw_str(
        buf,
        width,
        center_x(width, hint.len() as u32, scale),
        height.saturating_sub(ch * 2),
        hint,
        DIM_FG,
        BG,
        scale,
    );
}

// ── Generic list screen ────────────────────────────────────────────────────────

fn paint_list(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    title: &str,
    items: &[String],
    selected: usize,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    draw_str(
        buf,
        width,
        center_x(width, title.len() as u32, scale),
        ch,
        title,
        TITLE_FG,
        BG,
        scale,
    );
    font::hline(buf, width, 0, ch * 2 + ch / 2, width, BORDER_FG);

    let start_y = ch * 4;
    let max_visible = ((height.saturating_sub(start_y + ch * 2)) / ch) as usize;
    let scroll_start = if selected >= max_visible { selected - max_visible + 1 } else { 0 };

    for (i, item) in items.iter().enumerate().skip(scroll_start).take(max_visible) {
        let sel = i == selected;
        let row_y = start_y + (i - scroll_start) as u32 * ch;
        let text = format!("  {}  ", item);
        let ox = center_x(width, text.len() as u32, scale);
        if sel {
            fill_rect(buf, width, ox, row_y, text.len() as u32 * cw, ch, SELECT_BG);
            draw_str(buf, width, ox, row_y, &text, SELECT_FG, SELECT_BG, scale);
        } else {
            draw_str(buf, width, ox, row_y, &text, TEXT_FG, BG, scale);
        }
    }

    if items.is_empty() {
        let msg = "  (empty)  ";
        draw_str(
            buf,
            width,
            center_x(width, msg.len() as u32, scale),
            start_y,
            msg,
            DIM_FG,
            BG,
            scale,
        );
    }
}

fn paint_simple_text(buf: &mut [u32], width: u32, height: u32, scale: u32, msg: &str) {
    draw_str(
        buf,
        width,
        center_x(width, msg.len() as u32, scale),
        height / 2,
        msg,
        TEXT_FG,
        BG,
        scale,
    );
}

// ── PixelPainter ──────────────────────────────────────────────────────────────

pub struct PixelPainter<'a> {
    buf: &'a mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    cw: u32,
    ch: u32,
    // Animation phases (set in begin_frame)
    fire_ph: u32,
    traffic_ph: u32,
    util_ph: u32,
    blink: bool,
    // Layout values (set in begin_frame)
    menu_h: u32,
    status_h: u32,
    ticker_h: u32,
    map_px_y: u32,
    map_px_h: u32,
    tiles_cols: usize,
    tiles_rows: usize,
    tile_px: u32,
}

impl<'a> PixelPainter<'a> {
    pub fn new(buf: &'a mut [u32], width: u32, height: u32, scale: u32) -> Self {
        let cw = cell_w(scale);
        let ch = cell_h(scale);
        Self {
            buf,
            width,
            height,
            scale,
            cw,
            ch,
            fire_ph: 0,
            traffic_ph: 0,
            util_ph: 0,
            blink: false,
            menu_h: ch,
            status_h: ch,
            ticker_h: ch,
            map_px_y: ch * 2,
            map_px_h: height.saturating_sub(ch * 3),
            tiles_cols: (width / cw) as usize,
            tiles_rows: (height.saturating_sub(ch * 3) / ch) as usize,
            tile_px: cw,
        }
    }

    fn click_area(&self, px: u32, py: u32, pw: u32, ph: u32) -> ClickArea {
        ClickArea {
            x: (px / self.cw) as u16,
            y: (py / self.ch) as u16,
            width: (pw / self.cw).max(1) as u16,
            height: (ph / self.ch).max(1) as u16,
        }
    }

    fn window_chrome(&mut self, wx: u32, wy: u32, ww: u32, wh: u32, title: &str) {
        fill_rect(self.buf, self.width, wx + 3, wy + 3, ww, wh, SHADOW_COLOR);
        fill_rect(self.buf, self.width, wx, wy, ww, wh, POPUP_BG);
        draw_rect_outline(self.buf, self.width, wx, wy, ww, wh, BORDER_FG);
        fill_rect(self.buf, self.width, wx + 1, wy, ww - 2, self.ch, WIN_TITLE_BG);
        draw_str(self.buf, self.width, wx + self.cw, wy, title, WIN_TITLE_FG, WIN_TITLE_BG, self.scale);
        if ww >= 5 * self.cw {
            draw_str(self.buf, self.width, wx + ww - 4 * self.cw, wy, "[X]", DANGER_FG, WIN_TITLE_BG, self.scale);
        }
    }

    fn win_rect(&self, ingame: &InGameScreen, id: WindowId) -> (u32, u32, u32, u32) {
        let win = ingame.desktop.window(id);
        let wx = win.x as u32 * self.cw;
        let wy = win.y as u32 * self.ch;
        let ww = (win.width as u32 * self.cw).min(self.width.saturating_sub(wx));
        let wh = (win.height as u32 * self.ch).min(self.height.saturating_sub(wy));
        (wx, wy, ww, wh)
    }
}

impl<'a> InGamePainter for PixelPainter<'a> {
    fn begin_frame(&mut self, _layout: &FrameLayout) {
        self.fire_ph = anim_phase(180, 4);
        self.traffic_ph = anim_phase(280, 8);
        self.util_ph = anim_phase(220, 6);
        self.blink = anim_phase(400, 2) == 0;

        // Clear frame
        fill_rect(self.buf, self.width, 0, 0, self.width, self.height, BG);
        // Map background
        fill_rect(self.buf, self.width, 0, self.map_px_y, self.width, self.map_px_h, 0x101018);
    }

    fn paint_map(
        &mut self,
        map: &Map,
        camera: &Camera,
        overlay_mode: OverlayMode,
        _view_layer: ViewLayer,
        _current_tool: Tool,
        preview: MapPreview<'_>,
    ) -> MapUiAreas {
        let tiles_cols = self.tiles_cols;
        let tiles_rows = self.tiles_rows;
        let tile_px = self.tile_px;
        let map_px_y = self.map_px_y;

        for tile_row in 0..tiles_rows {
            for tile_col in 0..tiles_cols {
                let map_x = camera.offset_x as usize + tile_col;
                let map_y = camera.offset_y as usize + tile_row;
                if map_x >= map.width || map_y >= map.height {
                    continue;
                }

                let idx = map.idx(map_x, map_y);
                let tile = map.tiles[idx];
                let overlay = map.overlays[idx];

                let road_bits = if matches!(tile, crate::core::map::Tile::Road) {
                    let connects = |dx: i32, dy: i32| -> bool {
                        let nx = map_x as i32 + dx;
                        let ny = map_y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= map.width as i32 || ny >= map.height as i32 {
                            return false;
                        }
                        map.tiles[map.idx(nx as usize, ny as usize)].road_connects()
                    };
                    (connects(0, -1) as u8)
                        | ((connects(1, 0) as u8) << 1)
                        | ((connects(0, 1) as u8) << 2)
                        | ((connects(-1, 0) as u8) << 3)
                } else {
                    0u8
                };

                let px = tile_col as u32 * tile_px;
                let py = map_px_y + tile_row as u32 * tile_px;

                tiles::draw_tile(
                    self.buf, self.width, px, py, tile, &overlay, self.scale,
                    self.fire_ph, self.traffic_ph, self.util_ph, self.blink, road_bits,
                );

                if overlay_mode != OverlayMode::None {
                    let tint_val = overlay_value(overlay, overlay_mode);
                    let (hr, hg, hb) = heat_color(tint_val);
                    let tint = rgb_to_u32(hr, hg, hb);
                    for dy in 0..tile_px {
                        for dx in 0..tile_px {
                            let x = px + dx;
                            let y = py + dy;
                            let i = (y * self.width + x) as usize;
                            if let Some(p) = self.buf.get_mut(i) {
                                *p = lerp_u32(*p, tint, 140);
                            }
                        }
                    }
                }
            }
        }

        // Preview overlays
        match preview {
            MapPreview::Line(coords) | MapPreview::Rect(coords) => {
                for &(mx, my) in coords {
                    if mx < camera.offset_x as usize || my < camera.offset_y as usize {
                        continue;
                    }
                    let tc = mx - camera.offset_x as usize;
                    let tr = my - camera.offset_y as usize;
                    if tc >= tiles_cols || tr >= tiles_rows {
                        continue;
                    }
                    let px = tc as u32 * tile_px;
                    let py = map_px_y + tr as u32 * tile_px;
                    draw_rect_outline(self.buf, self.width, px, py, tile_px, tile_px, 0xFFFF44);
                }
            }
            MapPreview::Footprint(coords, all_valid) => {
                let tint = if all_valid { 0x44FF44 } else { 0xFF4444 };
                let outline = if all_valid { 0x88FF88 } else { 0xFF8888 };
                for &(mx, my) in coords {
                    if mx < camera.offset_x as usize || my < camera.offset_y as usize {
                        continue;
                    }
                    let tc = mx - camera.offset_x as usize;
                    let tr = my - camera.offset_y as usize;
                    if tc >= tiles_cols || tr >= tiles_rows {
                        continue;
                    }
                    let fpx = tc as u32 * tile_px;
                    let fpy = map_px_y + tr as u32 * tile_px;
                    for dy in 0..tile_px {
                        for dx in 0..tile_px {
                            let x = fpx + dx;
                            let y = fpy + dy;
                            let i = (y * self.width + x) as usize;
                            if let Some(p) = self.buf.get_mut(i) {
                                *p = lerp_u32(*p, tint, 80);
                            }
                        }
                    }
                    draw_rect_outline(self.buf, self.width, fpx, fpy, tile_px, tile_px, outline);
                }
            }
            MapPreview::None => {}
        }

        // Cursor highlight
        if camera.offset_x >= 0 && camera.offset_y >= 0 {
            let cur_col = camera.cursor_x.saturating_sub(camera.offset_x as usize);
            let cur_row = camera.cursor_y.saturating_sub(camera.offset_y as usize);
            if cur_col < tiles_cols && cur_row < tiles_rows {
                let cx = cur_col as u32 * tile_px;
                let cy = map_px_y + cur_row as u32 * tile_px;
                draw_rect_outline(self.buf, self.width, cx, cy, tile_px, tile_px, 0xFFFFFF);
            }
        }

        // Overlay mode label
        let overlay_label = overlay_mode.label();
        if !overlay_label.is_empty() {
            let lx = self.width.saturating_sub(overlay_label.len() as u32 * self.cw + self.cw);
            fill_rect(self.buf, self.width, lx, map_px_y, overlay_label.len() as u32 * self.cw, self.ch, 0x000000);
            draw_str(self.buf, self.width, lx, map_px_y, overlay_label, TITLE_FG, 0x000000, self.scale);
        }

        MapUiAreas {
            viewport: ClickArea {
                x: 0,
                y: (map_px_y / self.ch) as u16,
                width: tiles_cols as u16,
                height: tiles_rows as u16,
            },
            ..Default::default()
        }
    }

    fn paint_menu_bar(
        &mut self,
        menu_active: bool,
        menu_selected: usize,
        _menu_item_selected: usize,
    ) -> MenuBarAreas {
        let mut areas = MenuBarAreas::default();

        fill_rect(self.buf, self.width, 0, 0, self.width, self.ch, MENU_BG);

        let title = " TuiCity 2000 ";
        draw_str(self.buf, self.width, 0, 0, title, TITLE_FG, MENU_BG, self.scale);
        let mut x = title.len() as u32 * self.cw;

        areas.menu_bar = self.click_area(0, 0, self.width, self.ch);

        for (i, &menu_title) in crate::app::screens::MENU_TITLES.iter().enumerate() {
            let text = format!(" {} ", menu_title);
            let tw = text.len() as u32 * self.cw;
            let (fg, bg) = if menu_active && menu_selected == i {
                (MENU_FOCUS_FG, MENU_FOCUS_BG)
            } else {
                (MENU_FG, MENU_BG)
            };
            if x + tw <= self.width {
                fill_rect(self.buf, self.width, x, 0, tw, self.ch, bg);
                draw_str(self.buf, self.width, x, 0, &text, fg, bg, self.scale);
                areas.menu_items[i] = self.click_area(x, 0, tw, self.ch);
            }
            x += tw + self.cw / 2;
        }

        areas
    }

    fn paint_menu_popup(
        &mut self,
        menu_selected: usize,
        menu_item_selected: usize,
        anchor: ClickArea,
    ) -> MenuPopupAreas {
        let mut areas = MenuPopupAreas::default();

        let rows = crate::app::screens::menu_rows(menu_selected);
        if rows.is_empty() {
            return areas;
        }

        let max_label = rows.iter().map(|r| r.label.len() + r.right.len() + 4).max().unwrap_or(16);
        let pop_cols = max_label.max(16) as u32;
        let pop_rows = rows.len() as u32;
        let pop_w = pop_cols * self.cw;
        let pop_h = (pop_rows + 2) * self.ch;

        let px = (anchor.x as u32 * self.cw).min(self.width.saturating_sub(pop_w));
        let py = self.ch;

        fill_rect(self.buf, self.width, px + 2, py + 2, pop_w, pop_h, SHADOW_COLOR);
        fill_rect(self.buf, self.width, px, py, pop_w, pop_h, POPUP_BG);
        draw_rect_outline(self.buf, self.width, px, py, pop_w, pop_h, BORDER_FG);

        areas.menu_popup = self.click_area(px, py, pop_w, pop_h);

        for (i, row) in rows.iter().enumerate() {
            let item_y = py + self.ch + i as u32 * self.ch;
            let selected = i == menu_item_selected;
            let (fg, bg) = if selected { (SELECT_FG, SELECT_BG) } else { (TEXT_FG, POPUP_BG) };

            fill_rect(self.buf, self.width, px + 1, item_y, pop_w - 2, self.ch, bg);
            let label = format!(" {}", row.label);
            draw_str(self.buf, self.width, px + 1, item_y, &trunc(&label, (pop_cols - 1) as usize), fg, bg, self.scale);
            if !row.right.is_empty() {
                let rx = px + pop_w - (row.right.len() as u32 + 1) * self.cw;
                draw_str(self.buf, self.width, rx, item_y, row.right, DIM_FG, bg, self.scale);
            }

            areas.menu_popup_items.push(self.click_area(px, item_y, pop_w, self.ch));
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
        let mut areas = StatusBarAreas::default();
        let menu_h = self.menu_h;

        fill_rect(self.buf, self.width, 0, menu_h, self.width, self.status_h, STATUS_BG);
        let income_sign = if sim.economy.last_income >= 0 { "+" } else { "" };
        let status = format!(
            " {}  ${:+}  Pop:{}  {}{}  {}{}  {} ",
            sim.city_name,
            sim.economy.treasury,
            sim.pop.residential_population,
            sim.month_name(),
            sim.year,
            income_sign,
            sim.economy.last_income,
            if paused { "[PAUSED]" } else { "       " },
        );
        draw_str(self.buf, self.width, 0, menu_h, &status, TEXT_FG, STATUS_BG, self.scale);

        // Right-side controls
        let pause_label = if paused { "[>]" } else { "[||]" };
        let surface_label = "[Srf]";
        let underground_label = "[Ugr]";

        let mut rx = self.width.saturating_sub(
            (pause_label.len() + surface_label.len() + underground_label.len() + 3) as u32 * self.cw,
        );

        let pw = pause_label.len() as u32 * self.cw;
        let pause_fg = if paused { TITLE_FG } else { TEXT_FG };
        draw_str(self.buf, self.width, rx, menu_h, pause_label, pause_fg, STATUS_BG, self.scale);
        areas.pause_btn = self.click_area(rx, menu_h, pw, self.ch);
        rx += pw + self.cw;

        let sw = surface_label.len() as u32 * self.cw;
        let sfg = if matches!(view_layer, ViewLayer::Surface) { TITLE_FG } else { DIM_FG };
        draw_str(self.buf, self.width, rx, menu_h, surface_label, sfg, STATUS_BG, self.scale);
        areas.layer_surface_btn = self.click_area(rx, menu_h, sw, self.ch);
        rx += sw + self.cw;

        let uw = underground_label.len() as u32 * self.cw;
        let ufg = if matches!(view_layer, ViewLayer::Underground) { TITLE_FG } else { DIM_FG };
        draw_str(self.buf, self.width, rx, menu_h, underground_label, ufg, STATUS_BG, self.scale);
        areas.layer_underground_btn = self.click_area(rx, menu_h, uw, self.ch);

        if let Some(msg) = status_message {
            let rx = self.width.saturating_sub(msg.len() as u32 * self.cw + self.cw + 25 * self.cw);
            draw_str(self.buf, self.width, rx, menu_h, msg, TITLE_FG, STATUS_BG, self.scale);
        }

        areas
    }

    fn paint_panel_window(
        &mut self,
        toolbar: &ToolbarPaletteViewModel,
        current_tool: Tool,
        sim: &SimState,
        inspect_pos: Option<(usize, usize)>,
        map: &Map,
        ingame: &InGameScreen,
    ) -> PanelAreas {
        let mut areas = PanelAreas::default();

        let (wx, wy, ww, wh) = self.win_rect(ingame, WindowId::Panel);
        if ww < 4 || wh < 4 {
            return areas;
        }

        // Shadow + background + border + title
        fill_rect(self.buf, self.width, wx + 3, wy + 3, ww, wh, SHADOW_COLOR);
        fill_rect(self.buf, self.width, wx, wy, ww, wh, SIDEBAR_BG);
        draw_rect_outline(self.buf, self.width, wx, wy, ww, wh, BORDER_FG);
        fill_rect(self.buf, self.width, wx + 1, wy, ww - 2, self.ch, WIN_TITLE_BG);
        draw_str(self.buf, self.width, wx + self.cw, wy, " TOOLBOX ", WIN_TITLE_FG, WIN_TITLE_BG, self.scale);
        if ww >= 5 * self.cw {
            draw_str(self.buf, self.width, wx + ww - 4 * self.cw, wy, "[X]", DANGER_FG, WIN_TITLE_BG, self.scale);
        }

        let cx = wx + self.cw;
        let cy = wy + self.ch + self.ch / 2;
        let iw = ww.saturating_sub(self.cw * 2);
        let cols = (iw / self.cw) as usize;
        let mut row_y = cy;

        // Tool rows
        let tool_rows: &[(&str, Tool, u32)] = &[
            ("? Inspect", Tool::Inspect, 0xaaaaaa),
            ("B Bulldoze", Tool::Bulldoze, 0xff6644),
        ];
        for (label, tool, color) in tool_rows {
            if row_y + self.ch > wy + wh { break; }
            let active = toolbar.current_tool == *tool;
            let (fg, bg) = if active { (SELECT_FG, SELECT_BG) } else { (*color, BTN_BG) };
            fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, bg);
            draw_str(self.buf, self.width, cx, row_y, &trunc(label, cols), fg, bg, self.scale);
            areas.toolbar_items.push(ToolbarHitArea {
                area: self.click_area(cx, row_y, iw, self.ch),
                target: ToolbarHitTarget::SelectTool(*tool),
            });
            row_y += self.ch;
        }

        let chooser_rows: &[(&str, ToolChooserKind, Tool, u32)] = &[
            ("Zones", ToolChooserKind::Zones, toolbar.zone_tool, RES_FG),
            ("Transport", ToolChooserKind::Transport, toolbar.transport_tool, 0xaaaaaa),
            ("Utilities", ToolChooserKind::Utilities, toolbar.utility_tool, POWER_FG),
            ("Plants", ToolChooserKind::PowerPlants, toolbar.power_plant_tool, 0xff4444),
            ("Buildings", ToolChooserKind::Buildings, toolbar.building_tool, COMM_FG),
        ];
        for (prefix, kind, tool, color) in chooser_rows {
            if row_y + self.ch > wy + wh { break; }
            let active = toolbar.current_tool == *tool;
            let (fg, bg) = if active { (SELECT_FG, BTN_SEL_BG) } else { (*color, BTN_BG) };
            let label = format!("{}: {}", prefix, tool.label());
            fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, bg);
            draw_str(self.buf, self.width, cx, row_y, &trunc(&label, cols), fg, bg, self.scale);
            areas.toolbar_items.push(ToolbarHitArea {
                area: self.click_area(cx, row_y, iw, self.ch),
                target: ToolbarHitTarget::OpenChooser(*kind),
            });
            row_y += self.ch;
        }

        // Tool cost
        let cost = current_tool.cost();
        if cost > 0 && row_y + self.ch <= wy + wh {
            let cost_str = format!("${}", cost);
            fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, SIDEBAR_BG);
            draw_str(self.buf, self.width, cx, row_y, &cost_str, DIM_FG, SIDEBAR_BG, self.scale);
            row_y += self.ch;
        }

        if row_y + 2 <= wy + wh {
            font::hline(self.buf, self.width, cx, row_y, iw, BORDER_FG);
            row_y += 1;
        }

        // RCI demand
        if row_y + self.ch <= wy + wh {
            fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, SIDEBAR_BG);
            draw_str(self.buf, self.width, cx, row_y, "DEMAND:", DIM_FG, SIDEBAR_BG, self.scale);
            row_y += self.ch;
        }

        let bar_cols = cols.saturating_sub(3);
        for (label, demand, color) in [
            ("R:", sim.demand.res, RES_FG),
            ("C:", sim.demand.comm, COMM_FG),
            ("I:", sim.demand.ind, IND_FG),
        ] {
            if row_y + self.ch > wy + wh { break; }
            let fill = ((demand.clamp(0.0, 1.0) * bar_cols as f32) as usize).min(bar_cols);
            let bar: String = (0..bar_cols).map(|i| if i < fill { '#' } else { '.' }).collect();
            fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, SIDEBAR_BG);
            draw_str(self.buf, self.width, cx, row_y, label, DIM_FG, SIDEBAR_BG, self.scale);
            let bar_x = cx + 3 * self.cw;
            let filled_bar: String = bar.chars().take(fill).collect();
            let empty_bar: String = bar.chars().skip(fill).collect();
            draw_str(self.buf, self.width, bar_x, row_y, &filled_bar, color, SIDEBAR_BG, self.scale);
            draw_str(self.buf, self.width, bar_x + fill as u32 * self.cw, row_y, &empty_bar, DIM_FG, SIDEBAR_BG, self.scale);
            row_y += self.ch;
        }

        // Power summary
        if row_y + self.ch <= wy + wh {
            let util = &sim.utilities;
            let surplus = util.power_produced_mw as i32 - util.power_consumed_mw as i32;
            let pw_color = if surplus >= 0 { 0x66cc44u32 } else { 0xff4444u32 };
            let pw_str = format!("Pwr:{}/{}MW", util.power_produced_mw, util.power_consumed_mw);
            fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, SIDEBAR_BG);
            draw_str(self.buf, self.width, cx, row_y, &trunc(&pw_str, cols), pw_color, SIDEBAR_BG, self.scale);
            row_y += self.ch;
        }

        // Cursor tile info
        if let Some((tile_cx, tile_cy)) = inspect_pos {
            if tile_cx < map.width && tile_cy < map.height {
                let idx = map.idx(tile_cx, tile_cy);
                let tile = map.tiles[idx];
                let tile_ov = map.overlays[idx];
                if row_y + self.ch <= wy + wh {
                    let pos_str = format!("({},{})", tile_cx, tile_cy);
                    fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, SIDEBAR_BG);
                    draw_str(self.buf, self.width, cx, row_y, &pos_str, DIM_FG, SIDEBAR_BG, self.scale);
                    row_y += self.ch;
                }
                if row_y + self.ch <= wy + wh {
                    fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, SIDEBAR_BG);
                    draw_str(self.buf, self.width, cx, row_y, &trunc(tile.name(), cols), TITLE_FG, SIDEBAR_BG, self.scale);
                    row_y += self.ch;
                }
                if tile_ov.power_level > 0 && row_y + self.ch <= wy + wh {
                    let pct = tile_ov.power_level as u32 * 100 / 255;
                    let s = format!("Pwr {}%", pct);
                    fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, SIDEBAR_BG);
                    draw_str(self.buf, self.width, cx, row_y, &s, POWER_FG, SIDEBAR_BG, self.scale);
                    row_y += self.ch;
                }
            }
        }

        if row_y + 2 <= wy + wh {
            font::hline(self.buf, self.width, cx, row_y, iw, BORDER_FG);
            row_y += 1;
        }

        // Minimap
        let minimap_avail_h = (wy + wh).saturating_sub(row_y + self.ch + 2);
        if minimap_avail_h > 8 && iw > 8 {
            if row_y + self.ch <= wy + wh {
                fill_rect(self.buf, self.width, cx, row_y, iw, self.ch, SIDEBAR_BG);
                draw_str(self.buf, self.width, cx, row_y, "MINIMAP:", DIM_FG, SIDEBAR_BG, self.scale);
                row_y += self.ch;
            }

            let mw = map.width;
            let mh = map.height;
            let mm_w = iw.min(mw as u32 * 2);
            let mm_h = minimap_avail_h.min(mh as u32);
            let mm_x = cx + iw.saturating_sub(mm_w) / 2;
            let mm_y = row_y;

            fill_rect(self.buf, self.width, mm_x, mm_y, mm_w, mm_h, MINIMAP_FRAME);

            let cols_count = (mm_w / 2) as usize;
            let rows_count = mm_h as usize;

            for row in 0..rows_count {
                for col in 0..cols_count {
                    let map_x = if cols_count <= 1 { 0 } else { col * (mw - 1) / (cols_count - 1) };
                    let map_y = if rows_count <= 1 { 0 } else { row * (mh - 1) / (rows_count - 1) };
                    let idx = map.idx(map_x, map_y);
                    let tile = map.tiles[idx];
                    let ov = map.overlays[idx];
                    let glyph = theme::tile_glyph(tile, ov);
                    let color = color_to_u32(glyph.bg);
                    let px = mm_x + col as u32 * 2;
                    let py = mm_y + row as u32;
                    fill_rect(self.buf, self.width, px, py, 2, 1, color);
                }
            }

            // Viewport rectangle on minimap
            let cam = &ingame.camera;
            let vx0 = if mw <= 1 || cols_count <= 1 { 0u32 }
            else { (cam.offset_x.max(0) as u32 * (cols_count as u32 - 1) / (mw as u32 - 1)) * 2 };
            let vy0 = if mh <= 1 || rows_count <= 1 { 0u32 }
            else { cam.offset_y.max(0) as u32 * (rows_count as u32 - 1) / (mh as u32 - 1) };
            let vx1 = if mw <= 1 || cols_count <= 1 { mm_w.saturating_sub(1) }
            else { (((cam.offset_x + cam.view_w as i32).max(0) as u32).min(mw as u32 - 1)
                * (cols_count as u32 - 1) / (mw as u32 - 1)) * 2 };
            let vy1 = if mh <= 1 || rows_count <= 1 { mm_h.saturating_sub(1) }
            else { ((cam.offset_y + cam.view_h as i32).max(0) as u32).min(mh as u32 - 1)
                * (rows_count as u32 - 1) / (mh as u32 - 1) };
            let ax = mm_x + vx0.min(mm_w.saturating_sub(1));
            let ay = mm_y + vy0.min(mm_h.saturating_sub(1));
            let bx = mm_x + vx1.min(mm_w.saturating_sub(1));
            let by = mm_y + vy1.min(mm_h.saturating_sub(1));
            font::hline(self.buf, self.width, ax, ay, bx.saturating_sub(ax) + 1, 0xffffff);
            font::hline(self.buf, self.width, ax, by, bx.saturating_sub(ax) + 1, 0xffffff);
            font::vline(self.buf, self.width, ax, ay, by.saturating_sub(ay) + 1, 0xffffff);
            font::vline(self.buf, self.width, bx, ay, by.saturating_sub(ay) + 1, 0xffffff);

            areas.minimap = self.click_area(mm_x, mm_y, mm_w, mm_h);
        }

        areas
    }

    fn paint_tool_chooser(&mut self, chooser: &ToolChooserViewModel) -> Vec<ClickArea> {
        let mut items = Vec::new();

        let max_name = chooser.tools.iter().map(|t| t.label().len()).max().unwrap_or(10);
        let pop_cols = (max_name + 12).max(24) as u32;
        let pop_rows = (chooser.tools.len() + 3) as u32;
        let pop_w = pop_cols * self.cw;
        let pop_h = pop_rows * self.ch;
        let px = self.width.saturating_sub(pop_w) / 2;
        let py = self.height.saturating_sub(pop_h) / 2;

        fill_rect(self.buf, self.width, px + 3, py + 3, pop_w, pop_h, SHADOW_COLOR);
        fill_rect(self.buf, self.width, px, py, pop_w, pop_h, POPUP_BG);
        draw_rect_outline(self.buf, self.width, px, py, pop_w, pop_h, BORDER_FG);
        fill_rect(self.buf, self.width, px + 1, py, pop_w - 2, self.ch, WIN_TITLE_BG);
        draw_str(self.buf, self.width, px + self.cw, py, "Tool Selection", WIN_TITLE_FG, WIN_TITLE_BG, self.scale);
        draw_str(self.buf, self.width, px + pop_w - 4 * self.cw, py, "[X]", DANGER_FG, WIN_TITLE_BG, self.scale);

        for (i, tool) in chooser.tools.iter().enumerate() {
            let item_y = py + self.ch + i as u32 * self.ch;
            let selected = *tool == chooser.selected_tool;
            let (fg, bg) = if selected { (SELECT_FG, SELECT_BG) } else { (TEXT_FG, POPUP_BG) };
            fill_rect(self.buf, self.width, px + 1, item_y, pop_w - 2, self.ch, bg);
            let cost = tool.cost();
            let label = if cost > 0 {
                format!(" {:<20} ${}", tool.label(), cost)
            } else {
                format!(" {}", tool.label())
            };
            draw_str(self.buf, self.width, px + 1, item_y, &trunc(&label, pop_cols as usize - 1), fg, bg, self.scale);
            items.push(self.click_area(px + 1, item_y, pop_w - 2, self.ch));
        }

        items
    }

    fn paint_confirm_dialog(&mut self, dialog: &ConfirmDialogViewModel) -> Vec<ClickArea> {
        let mut items = Vec::new();
        let pop_cols: u32 = 36;
        let msg_rows = (dialog.message.len() as u32 / pop_cols + 2).max(2);
        let pop_rows = 3 + msg_rows + 2;
        let pop_w = pop_cols * self.cw;
        let pop_h = pop_rows * self.ch;
        let px = self.width.saturating_sub(pop_w) / 2;
        let py = self.height.saturating_sub(pop_h) / 2;

        fill_rect(self.buf, self.width, px + 3, py + 3, pop_w, pop_h, SHADOW_COLOR);
        fill_rect(self.buf, self.width, px, py, pop_w, pop_h, POPUP_BG);
        draw_rect_outline(self.buf, self.width, px, py, pop_w, pop_h, BORDER_FG);
        fill_rect(self.buf, self.width, px + 1, py, pop_w - 2, self.ch, WIN_TITLE_BG);
        draw_str(self.buf, self.width, px + self.cw, py, &trunc(&dialog.title, pop_cols as usize - 2), WIN_TITLE_FG, WIN_TITLE_BG, self.scale);

        let msg_x = px + self.cw;
        let msg_max = (pop_cols - 2) as usize;
        let mut lines: Vec<&str> = Vec::new();
        let mut rest = dialog.message.as_str();
        while !rest.is_empty() {
            let (line, tail) = if rest.len() <= msg_max {
                (rest, "")
            } else {
                let split = rest[..msg_max].rfind(' ').unwrap_or(msg_max);
                (&rest[..split], rest[split..].trim_start_matches(' '))
            };
            lines.push(line);
            rest = tail;
            if lines.len() >= msg_rows as usize { break; }
        }
        for (i, line) in lines.iter().enumerate() {
            let ly = py + self.ch + i as u32 * self.ch;
            fill_rect(self.buf, self.width, px + 1, ly, pop_w - 2, self.ch, POPUP_BG);
            draw_str(self.buf, self.width, msg_x, ly, line, TEXT_FG, POPUP_BG, self.scale);
        }

        let btn_y = py + pop_h - 2 * self.ch;
        fill_rect(self.buf, self.width, px + 1, btn_y, pop_w - 2, self.ch, POPUP_BG);
        let btn_total_w: u32 = dialog.buttons.iter().map(|b| (b.label.len() as u32 + 4) * self.cw).sum();
        let mut bx = px + pop_w.saturating_sub(btn_total_w) / 2;
        for (i, btn) in dialog.buttons.iter().enumerate() {
            let bw = (btn.label.len() as u32 + 4) * self.cw;
            let selected = i == dialog.selected;
            let (fg, bg) = if selected { (SELECT_FG, SELECT_BG) } else { (TEXT_FG, BTN_BG) };
            fill_rect(self.buf, self.width, bx, btn_y, bw, self.ch, bg);
            let text = format!("[ {} ]", btn.label);
            draw_str(self.buf, self.width, bx, btn_y, &text, fg, bg, self.scale);
            items.push(self.click_area(bx, btn_y, bw, self.ch));
            bx += bw + self.cw;
        }

        items
    }

    fn paint_budget_window(&mut self, budget: &BudgetViewModel, ingame: &InGameScreen) {
        let (wx, wy, ww, wh) = self.win_rect(ingame, WindowId::Budget);
        if ww < 8 || wh < 6 { return; }

        self.window_chrome(wx, wy, ww, wh, "Budget Control Center");

        let ix = wx + self.cw;
        let mut iy = wy + self.ch + self.ch / 2;
        let icols = ((ww / self.cw).saturating_sub(2)) as usize;

        for (label, pct) in [
            ("Residential Tax", budget.tax_rates.residential),
            ("Commercial Tax ", budget.tax_rates.commercial),
            ("Industrial Tax  ", budget.tax_rates.industrial),
        ] {
            if iy + self.ch > wy + wh { break; }
            let s = format!("{}: {}%", label, pct);
            fill_rect(self.buf, self.width, ix, iy, ww - 2 * self.cw, self.ch, POPUP_BG);
            draw_str(self.buf, self.width, ix, iy, &trunc(&s, icols), TEXT_FG, POPUP_BG, self.scale);
            iy += self.ch;
        }

        if iy + 1 <= wy + wh {
            font::hline(self.buf, self.width, ix, iy, ww - 2 * self.cw, BORDER_FG);
            iy += 1;
        }

        let income_sign = if budget.current_annual_tax >= 0 { "+" } else { "" };
        for (label, val, color) in [
            ("Treasury", format!("${}", budget.treasury), if budget.treasury >= 0 { RES_FG } else { DANGER_FG }),
            ("Annual Tax", format!("{}{}", income_sign, budget.current_annual_tax), TEXT_FG),
        ] {
            if iy + self.ch > wy + wh { break; }
            let s = format!("{}: {}", label, val);
            fill_rect(self.buf, self.width, ix, iy, ww - 2 * self.cw, self.ch, POPUP_BG);
            draw_str(self.buf, self.width, ix, iy, &trunc(&s, icols), color, POPUP_BG, self.scale);
            iy += self.ch;
        }
    }

    fn paint_statistics_window(&mut self, stats: &StatisticsWindowViewModel, ingame: &InGameScreen) {
        let (wx, wy, ww, wh) = self.win_rect(ingame, WindowId::Statistics);
        if ww < 8 || wh < 6 { return; }

        self.window_chrome(wx, wy, ww, wh, "City Statistics");

        let ix = wx + self.cw;
        let mut iy = wy + self.ch + self.ch / 2;
        let icols = ((ww / self.cw).saturating_sub(2)) as usize;

        let lines = [
            format!("{}", stats.city_name),
            format!("Population: {}", stats.current_population),
            format!("Treasury: ${}", stats.current_treasury),
            format!("Annual Income: ${}", stats.current_income),
            format!("Power: {} / {} MW", stats.current_power_consumed, stats.current_power_produced),
            String::new(),
            format!("History: {} months", stats.treasury_history.len()),
        ];

        for line in &lines {
            if iy + self.ch > wy + wh { break; }
            fill_rect(self.buf, self.width, ix, iy, ww - 2 * self.cw, self.ch, POPUP_BG);
            draw_str(self.buf, self.width, ix, iy, &trunc(line, icols), TEXT_FG, POPUP_BG, self.scale);
            iy += self.ch;
        }

        if iy + self.ch <= wy + wh { iy += self.ch / 2; }
        let chart_w = ((ww - 2 * self.cw) / self.cw) as usize;
        for (label, data, color) in [
            ("Treasury", stats_sparkline_i64(&stats.treasury_history, chart_w), BORDER_FG),
            ("Population", stats_sparkline_u64(&stats.population_history, chart_w), RES_FG),
            ("Income", stats_sparkline_i64(&stats.income_history, chart_w), IND_FG),
        ] {
            if iy + self.ch * 2 > wy + wh { break; }
            fill_rect(self.buf, self.width, ix, iy, ww - 2 * self.cw, self.ch, POPUP_BG);
            draw_str(self.buf, self.width, ix, iy, &trunc(label, icols), DIM_FG, POPUP_BG, self.scale);
            iy += self.ch;
            fill_rect(self.buf, self.width, ix, iy, ww - 2 * self.cw, self.ch, POPUP_BG);
            draw_str(self.buf, self.width, ix, iy, &trunc(&data, icols), color, POPUP_BG, self.scale);
            iy += self.ch;
        }
    }

    fn paint_inspect_window(
        &mut self,
        inspect_pos: Option<(usize, usize)>,
        map: &Map,
        _sim: &SimState,
        ingame: &InGameScreen,
    ) {
        let (wx, wy, ww, wh) = self.win_rect(ingame, WindowId::Inspect);
        if ww < 8 || wh < 6 { return; }

        self.window_chrome(wx, wy, ww, wh, "Inspect");

        let ix = wx + self.cw;
        let mut iy = wy + self.ch + self.ch / 2;
        let icols = ((ww / self.cw).saturating_sub(2)) as usize;

        if let Some((tx, ty)) = inspect_pos {
            if tx < map.width && ty < map.height {
                let idx = map.idx(tx, ty);
                let tile = map.tiles[idx];
                let ov = map.overlays[idx];

                let lines = [
                    format!("Position: ({}, {})", tx, ty),
                    format!("Tile: {}", tile.name()),
                    format!("Power: {}%", ov.power_level as u32 * 100 / 255),
                    format!("Water: {}%", ov.water_service as u32 * 100 / 255),
                    format!("Traffic: {}", ov.traffic),
                    format!("Pollution: {}", ov.pollution),
                    format!("Crime: {}", ov.crime),
                    format!("Land Value: {}", ov.land_value),
                    format!("Fire Risk: {}", ov.fire_risk),
                ];

                for line in &lines {
                    if iy + self.ch > wy + wh { break; }
                    fill_rect(self.buf, self.width, ix, iy, ww - 2 * self.cw, self.ch, POPUP_BG);
                    draw_str(self.buf, self.width, ix, iy, &trunc(line, icols), TEXT_FG, POPUP_BG, self.scale);
                    iy += self.ch;
                }
            }
        } else {
            fill_rect(self.buf, self.width, ix, iy, ww - 2 * self.cw, self.ch, POPUP_BG);
            draw_str(self.buf, self.width, ix, iy, "Move cursor to inspect", DIM_FG, POPUP_BG, self.scale);
        }
    }

    fn paint_text_window(
        &mut self,
        window_id: WindowId,
        view: &TextWindowViewModel,
        ingame: &mut InGameScreen,
    ) {
        let win = ingame.desktop.window(window_id);
        let title = win.title;
        let wx = win.x as u32 * self.cw;
        let wy = win.y as u32 * self.ch;
        let ww = (win.width as u32 * self.cw).min(self.width.saturating_sub(wx));
        let wh = (win.height as u32 * self.ch).min(self.height.saturating_sub(wy));
        if ww < 8 || wh < 6 { return; }

        self.window_chrome(wx, wy, ww, wh, title);

        let ix = wx + self.cw;
        let iy_start = wy + self.ch + self.ch / 2;
        let icols = ((ww / self.cw).saturating_sub(2)) as usize;
        let visible_rows = ((wh - self.ch - self.ch / 2) / self.ch) as usize;

        let scroll = view.scroll_y as usize;
        for (i, line) in view.lines.iter().skip(scroll).take(visible_rows).enumerate() {
            let iy = iy_start + i as u32 * self.ch;
            fill_rect(self.buf, self.width, ix, iy, ww - 2 * self.cw, self.ch, POPUP_BG);
            draw_str(self.buf, self.width, ix, iy, &trunc(line, icols), TEXT_FG, POPUP_BG, self.scale);
        }

        let win = ingame.desktop.window_mut(window_id);
        win.content_height = view.lines.len() as u16;
    }

    fn paint_news_ticker(&mut self, ticker: &NewsTickerViewModel) {
        let ticker_y = self.height.saturating_sub(self.ticker_h);
        let ticker_bg = 0x19142b;
        fill_rect(self.buf, self.width, 0, ticker_y, self.width, self.ch, ticker_bg);
        let ticker_fg = if ticker.is_alerting { 0xff6450 } else { 0xaadcdb };
        let cols = (self.width / self.cw) as usize;
        let max_chars = cols.min(ticker.full_text.len().saturating_sub(ticker.scroll_offset));
        let visible_news: String = ticker
            .full_text
            .chars()
            .skip(ticker.scroll_offset)
            .take(max_chars)
            .collect();
        draw_str(self.buf, self.width, self.cw, ticker_y, &visible_news, ticker_fg, ticker_bg, self.scale);
    }

    fn end_frame(&mut self) {
        // No-op — softbuffer present happens in mod.rs
    }
}

// ── Overlay helpers ────────────────────────────────────────────────────────────

fn overlay_value(overlay: TileOverlay, mode: OverlayMode) -> u8 {
    match mode {
        OverlayMode::None => 0,
        OverlayMode::Power => overlay.power_level,
        OverlayMode::Water => overlay.water_service,
        OverlayMode::Traffic => overlay.traffic,
        OverlayMode::Pollution => overlay.pollution,
        OverlayMode::LandValue => overlay.land_value,
        OverlayMode::Crime => overlay.crime,
        OverlayMode::FireRisk => overlay.fire_risk,
    }
}

fn heat_color(val: u8) -> (u8, u8, u8) {
    let t = val as f32 / 255.0;
    if t < 0.5 {
        let u = t * 2.0;
        (0, (u * 100.0) as u8, (255.0 * (1.0 - u)) as u8)
    } else {
        let u = (t - 0.5) * 2.0;
        ((u * 255.0) as u8, (100.0 * (1.0 - u)) as u8, 0)
    }
}

fn lerp_u32(a: u32, b: u32, t: u8) -> u32 {
    let t = t as u32;
    let s = 255 - t;
    let r = ((a >> 16 & 0xff) * s + (b >> 16 & 0xff) * t) / 255;
    let g = ((a >> 8 & 0xff) * s + (b >> 8 & 0xff) * t) / 255;
    let bl = ((a & 0xff) * s + (b & 0xff) * t) / 255;
    (r << 16) | (g << 8) | bl
}

fn stats_sparkline_i64(data: &std::collections::VecDeque<i64>, width: usize) -> String {
    if data.is_empty() || width == 0 { return String::new(); }
    let sparks = &['_', '.', '-', '~', '+', '*', '#', '@'];
    let min = data.iter().copied().min().unwrap_or(0);
    let max = data.iter().copied().max().unwrap_or(0);
    let range = (max - min).max(1) as f64;
    let step = data.len().max(1) as f64 / width as f64;
    (0..width).map(|i| {
        let idx = ((i as f64 * step) as usize).min(data.len().saturating_sub(1));
        let val = data.get(idx).copied().unwrap_or(0);
        let norm = ((val - min) as f64 / range * 7.0) as usize;
        sparks[norm.min(7)]
    }).collect()
}

fn stats_sparkline_u64(data: &std::collections::VecDeque<u64>, width: usize) -> String {
    if data.is_empty() || width == 0 { return String::new(); }
    let sparks = &['_', '.', '-', '~', '+', '*', '#', '@'];
    let min = data.iter().copied().min().unwrap_or(0);
    let max = data.iter().copied().max().unwrap_or(0);
    let range = (max - min).max(1) as f64;
    let step = data.len().max(1) as f64 / width as f64;
    (0..width).map(|i| {
        let idx = ((i as f64 * step) as usize).min(data.len().saturating_sub(1));
        let val = data.get(idx).copied().unwrap_or(0);
        let norm = ((val - min) as f64 / range * 7.0) as usize;
        sparks[norm.min(7)]
    }).collect()
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn center_x(width: u32, char_count: u32, scale: u32) -> u32 {
    width.saturating_sub(char_count * cell_w(scale)) / 2
}

fn trunc(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}
