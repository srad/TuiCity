use crate::{
    app::{screens::InGameScreen, ClickArea, MapUiAreas},
    core::{map::TileOverlay, tool::Tool},
    ui::{
        runtime::{ToolbarHitArea, ToolbarHitTarget, ToolChooserKind, UiRect, WindowId},
        theme,
        view::{ScreenView, SettingsViewModel, StartViewModel},
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
            paint_ingame(buf, width, height, scale, v, ingame);
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

// ── In-game screen ─────────────────────────────────────────────────────────────

fn paint_ingame(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    view: &crate::ui::view::InGameDesktopView,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let total_cols = (width / cw) as u16;
    let total_rows = (height / ch) as u16;

    // Animation phases (mirrors src/ui/game/map_view.rs pattern)
    let fire_ph = anim_phase(180, 4);
    let traffic_ph = anim_phase(280, 8);
    let util_ph = anim_phase(220, 6);
    let blink = anim_phase(400, 2) == 0;

    // Compute desktop layout (resolves window positions, centering, clamping)
    let desktop_layout = ingame.desktop.layout(UiRect::new(0, 0, total_cols, total_rows));
    ingame.ui_areas.desktop = desktop_layout.clone();

    // Layout rows (in pixels)
    let menu_h = ch;       // row 0: menu bar
    let status_h = ch;     // row 1: status bar
    let ticker_h = ch;     // last row: news ticker
    let map_px_y = menu_h + status_h;
    let map_px_h = height.saturating_sub(map_px_y + ticker_h);
    let tiles_cols = (width / cw) as usize;
    let tiles_rows = (map_px_h / ch) as usize;
    let tile_px = cw; // square tiles

    // ── Map background ────────────────────────────────────────────────────
    fill_rect(buf, width, 0, map_px_y, width, map_px_h, 0x101018);

    // ── Map tiles ─────────────────────────────────────────────────────────
    let map = &view.map;
    let cam = &view.camera;
    let overlay_mode = view.overlay_mode;

    ingame.camera.view_w = tiles_cols.max(1);
    ingame.camera.view_h = tiles_rows.max(1);
    // Pixel frontend: 1 cell per tile — col_scale=1 so screen_to_map doesn't halve x
    ingame.camera.col_scale = 1;

    ingame.ui_areas.map = MapUiAreas {
        viewport: ClickArea {
            x: 0,
            y: (map_px_y / ch) as u16,
            width: tiles_cols as u16,
            height: tiles_rows as u16,
        },
        ..Default::default()
    };

    for tile_row in 0..tiles_rows {
        for tile_col in 0..tiles_cols {
            let map_x = cam.offset_x as usize + tile_col;
            let map_y = cam.offset_y as usize + tile_row;
            if map_x >= map.width || map_y >= map.height {
                continue;
            }

            let idx = map.idx(map_x, map_y);
            let tile = map.tiles[idx];
            let overlay = map.overlays[idx];

            // Road connectivity bits: N=0, E=1, S=2, W=3
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
                buf, width, px, py, tile, &overlay, scale,
                fire_ph, traffic_ph, util_ph, blink, road_bits,
            );

            // Per-overlay tint (on top of animated sprite)
            if overlay_mode != crate::ui::theme::OverlayMode::None {
                let tint_val = overlay_value(overlay, overlay_mode);
                let (hr, hg, hb) = heat_color(tint_val);
                let tint = rgb_to_u32(hr, hg, hb);
                for dy in 0..tile_px {
                    for dx in 0..tile_px {
                        let x = px + dx;
                        let y = py + dy;
                        let i = (y * width + x) as usize;
                        if let Some(p) = buf.get_mut(i) {
                            *p = lerp_u32(*p, tint, 140);
                        }
                    }
                }
            }
        }
    }

    // ── Line/rect placement preview ───────────────────────────────────────
    for &(mx, my) in view.line_preview.iter().chain(view.rect_preview.iter()) {
        if mx < cam.offset_x as usize || my < cam.offset_y as usize {
            continue;
        }
        let tc = mx - cam.offset_x as usize;
        let tr = my - cam.offset_y as usize;
        if tc >= tiles_cols || tr >= tiles_rows {
            continue;
        }
        let px = tc as u32 * tile_px;
        let py = map_px_y + tr as u32 * tile_px;
        draw_rect_outline(buf, width, px, py, tile_px, tile_px, 0xFFFF44);
    }

    // ── Cursor highlight ──────────────────────────────────────────────────
    if cam.offset_x >= 0 && cam.offset_y >= 0 {
        let cur_col = cam.cursor_x.saturating_sub(cam.offset_x as usize);
        let cur_row = cam.cursor_y.saturating_sub(cam.offset_y as usize);
        if cur_col < tiles_cols && cur_row < tiles_rows {
            let cx = cur_col as u32 * tile_px;
            let cy = map_px_y + cur_row as u32 * tile_px;
            draw_rect_outline(buf, width, cx, cy, tile_px, tile_px, 0xFFFFFF);
        }
    }

    // ── Overlay mode label (top-right of map) ─────────────────────────────
    let overlay_label = overlay_mode.label();
    if !overlay_label.is_empty() {
        let lx = width.saturating_sub(overlay_label.len() as u32 * cw + cw);
        fill_rect(buf, width, lx, map_px_y, overlay_label.len() as u32 * cw, ch, 0x000000);
        draw_str(buf, width, lx, map_px_y, overlay_label, TITLE_FG, 0x000000, scale);
    }

    // ── Menu bar ──────────────────────────────────────────────────────────
    paint_menu_bar(buf, width, scale, view, ingame);

    // ── Status bar ────────────────────────────────────────────────────────
    fill_rect(buf, width, 0, menu_h, width, status_h, STATUS_BG);
    let sim = &view.sim;
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
        if view.paused { "[PAUSED]" } else { "       " },
    );
    draw_str(buf, width, 0, menu_h, &status, TEXT_FG, STATUS_BG, scale);

    // Right-side controls: [||] [Surface] [Underground]
    {
        let pause_label = if view.paused { "[>]" } else { "[||]" };
        let surface_label = "[Srf]";
        let underground_label = "[Ugr]";

        let mut rx = width.saturating_sub(
            (pause_label.len() + surface_label.len() + underground_label.len() + 3) as u32 * cw,
        );

        let pw = pause_label.len() as u32 * cw;
        let pause_fg = if view.paused { TITLE_FG } else { TEXT_FG };
        draw_str(buf, width, rx, menu_h, pause_label, pause_fg, STATUS_BG, scale);
        ingame.ui_areas.pause_btn = pixel_click_area(rx, menu_h, pw, ch, scale);
        rx += pw + cw;

        let sw = surface_label.len() as u32 * cw;
        let sfg = if matches!(view.view_layer, crate::core::map::ViewLayer::Surface) {
            TITLE_FG
        } else {
            DIM_FG
        };
        draw_str(buf, width, rx, menu_h, surface_label, sfg, STATUS_BG, scale);
        ingame.ui_areas.layer_surface_btn = pixel_click_area(rx, menu_h, sw, ch, scale);
        rx += sw + cw;

        let uw = underground_label.len() as u32 * cw;
        let ufg = if matches!(view.view_layer, crate::core::map::ViewLayer::Underground) {
            TITLE_FG
        } else {
            DIM_FG
        };
        draw_str(buf, width, rx, menu_h, underground_label, ufg, STATUS_BG, scale);
        ingame.ui_areas.layer_underground_btn = pixel_click_area(rx, menu_h, uw, ch, scale);
    }

    if let Some(msg) = &view.status_message {
        let rx = width.saturating_sub(msg.len() as u32 * cw + cw + 25 * cw);
        draw_str(buf, width, rx, menu_h, msg, TITLE_FG, STATUS_BG, scale);
    }

    // ── Floating Panel window (TOOLBOX) ───────────────────────────────────
    ingame.ui_areas.toolbar_items.clear();
    ingame.ui_areas.minimap = ClickArea::default();
    paint_panel_window(buf, width, height, scale, view, ingame, map_px_y, map_px_h);

    // ── Tool chooser popup ────────────────────────────────────────────────
    ingame.ui_areas.tool_chooser_items.clear();
    if let Some(chooser) = &view.tool_chooser {
        paint_tool_chooser(buf, width, height, scale, chooser, ingame);
    }

    // ── Confirm dialog ────────────────────────────────────────────────────
    ingame.ui_areas.dialog_items.clear();
    if let Some(dialog) = &view.confirm_dialog {
        paint_confirm_dialog(buf, width, height, scale, dialog, ingame);
    }

    // ── Budget window ─────────────────────────────────────────────────────
    if ingame.desktop.is_open(WindowId::Budget) {
        paint_budget_window(buf, width, height, scale, view, ingame);
    }

    // ── Statistics window ────────────────────────────────────────────────
    if let Some(stats) = &view.statistics {
        paint_statistics_window(buf, width, height, scale, stats, ingame);
    }

    // ── Inspect window ──────────────────────────────────────────────────
    if ingame.desktop.is_open(WindowId::Inspect) {
        paint_inspect_window(buf, width, height, scale, view, ingame);
    }

    // ── Help / About / Legend text windows ───────────────────────────────
    if let Some(help) = &view.help {
        paint_text_window(buf, width, height, scale, WindowId::Help, help, ingame);
    }
    if let Some(about) = &view.about {
        paint_text_window(buf, width, height, scale, WindowId::About, about, ingame);
    }
    if let Some(legend) = &view.legend {
        paint_text_window(buf, width, height, scale, WindowId::Legend, legend, ingame);
    }

    // ── Footprint preview ───────────────────────────────────────────────
    {
        use crate::core::tool::Tool;
        let footprint = view.current_tool.footprint();
        if Tool::uses_footprint_preview(view.current_tool)
            && view.rect_preview.is_empty()
            && view.line_preview.is_empty()
        {
            let (fw, fh) = footprint;
            let cx = cam.cursor_x;
            let cy = cam.cursor_y;
            let ax = cx.saturating_sub(fw / 2).min(view.map.width.saturating_sub(fw));
            let ay = cy.saturating_sub(fh / 2).min(view.map.height.saturating_sub(fh));
            let all_valid = (0..fh).all(|dy| {
                (0..fw).all(|dx| {
                    let x = ax + dx;
                    let y = ay + dy;
                    x < view.map.width
                        && y < view.map.height
                        && view
                            .current_tool
                            .can_place(view.map.view_tile(view.view_layer, x, y))
                })
            });
            let tint = if all_valid { 0x44FF44 } else { 0xFF4444 };
            let outline = if all_valid { 0x88FF88 } else { 0xFF8888 };
            for fy in 0..fh {
                for fx in 0..fw {
                    let mx = ax + fx;
                    let my = ay + fy;
                    if mx < cam.offset_x as usize || my < cam.offset_y as usize {
                        continue;
                    }
                    let tc = mx - cam.offset_x as usize;
                    let tr = my - cam.offset_y as usize;
                    if tc >= tiles_cols || tr >= tiles_rows {
                        continue;
                    }
                    let fpx = tc as u32 * tile_px;
                    let fpy = map_px_y + tr as u32 * tile_px;
                    // Semi-transparent tint for footprint
                    for dy in 0..tile_px {
                        for dx in 0..tile_px {
                            let x = fpx + dx;
                            let y = fpy + dy;
                            let i = (y * width + x) as usize;
                            if let Some(p) = buf.get_mut(i) {
                                *p = lerp_u32(*p, tint, 80);
                            }
                        }
                    }
                    draw_rect_outline(buf, width, fpx, fpy, tile_px, tile_px, outline);
                }
            }
        }
    }

    // ── News ticker ───────────────────────────────────────────────────────
    let ticker_y = height.saturating_sub(ticker_h);
    let ticker_bg = 0x19142b;
    fill_rect(buf, width, 0, ticker_y, width, ch, ticker_bg);
    let ticker_fg = if view.news_ticker.is_alerting { 0xff6450 } else { 0xaadcdb };
    let cols = (width / cw) as usize;
    let max_chars = cols
        .min(view.news_ticker.full_text.len().saturating_sub(view.news_ticker.scroll_offset));
    let visible_news: String = view
        .news_ticker
        .full_text
        .chars()
        .skip(view.news_ticker.scroll_offset)
        .take(max_chars)
        .collect();
    draw_str(buf, width, cw, ticker_y, &visible_news, ticker_fg, ticker_bg, scale);
}

// ── Menu bar ───────────────────────────────────────────────────────────────────

fn paint_menu_bar(
    buf: &mut [u32],
    width: u32,
    scale: u32,
    view: &crate::ui::view::InGameDesktopView,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);

    fill_rect(buf, width, 0, 0, width, ch, MENU_BG);

    // Game title at left
    let title = " TuiCity 2000 ";
    draw_str(buf, width, 0, 0, title, TITLE_FG, MENU_BG, scale);
    let mut x = title.len() as u32 * cw;

    // Register bar click area (full menu bar row)
    ingame.ui_areas.menu_bar = pixel_click_area(0, 0, width, ch, scale);

    for (i, &title) in crate::app::screens::MENU_TITLES.iter().enumerate() {
        let text = format!(" {} ", title);
        let tw = text.len() as u32 * cw;
        let (fg, bg) = if view.menu_active && view.menu_selected == i {
            (MENU_FOCUS_FG, MENU_FOCUS_BG)
        } else {
            (MENU_FG, MENU_BG)
        };
        if x + tw <= width {
            fill_rect(buf, width, x, 0, tw, ch, bg);
            draw_str(buf, width, x, 0, &text, fg, bg, scale);
            ingame.ui_areas.menu_items[i] = pixel_click_area(x, 0, tw, ch, scale);
        }
        x += tw + cw / 2;
    }

    // Menu popup (open below selected item)
    ingame.ui_areas.menu_popup = ClickArea::default();
    ingame.ui_areas.menu_popup_items.clear();
    if view.menu_active {
        paint_menu_popup(buf, width, scale, view, ingame);
    }
}

fn paint_menu_popup(
    buf: &mut [u32],
    width: u32,
    scale: u32,
    view: &crate::ui::view::InGameDesktopView,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let rows = crate::app::screens::menu_rows(view.menu_selected);
    if rows.is_empty() {
        return;
    }

    // Max label width
    let max_label = rows.iter().map(|r| r.label.len() + r.right.len() + 4).max().unwrap_or(16);
    let pop_cols = max_label.max(16) as u32;
    let pop_rows = rows.len() as u32;
    let pop_w = pop_cols * cw;
    let pop_h = (pop_rows + 2) * ch;

    // Position below the selected menu item (item_area.x is in cell coords = pixels / cw)
    let item_area = ingame.ui_areas.menu_items[view.menu_selected];
    let px = (item_area.x as u32 * cw).min(width.saturating_sub(pop_w));
    let py = ch; // below menu bar

    // Draw shadow
    fill_rect(buf, width, px + 2, py + 2, pop_w, pop_h, SHADOW_COLOR);
    // Background + border
    fill_rect(buf, width, px, py, pop_w, pop_h, POPUP_BG);
    draw_rect_outline(buf, width, px, py, pop_w, pop_h, BORDER_FG);

    ingame.ui_areas.menu_popup = pixel_click_area(px, py, pop_w, pop_h, scale);

    for (i, row) in rows.iter().enumerate() {
        let item_y = py + ch + i as u32 * ch;
        let selected = i == view.menu_item_selected;
        let (fg, bg) = if selected { (SELECT_FG, SELECT_BG) } else { (TEXT_FG, POPUP_BG) };

        fill_rect(buf, width, px + 1, item_y, pop_w - 2, ch, bg);
        let label = format!(" {}", row.label);
        draw_str(buf, width, px + 1, item_y, &trunc(&label, (pop_cols - 1) as usize), fg, bg, scale);
        if !row.right.is_empty() {
            let rx = px + pop_w - (row.right.len() as u32 + 1) * cw;
            draw_str(buf, width, rx, item_y, row.right, DIM_FG, bg, scale);
        }

        ingame.ui_areas.menu_popup_items.push(pixel_click_area(px, item_y, pop_w, ch, scale));
    }
}

// ── Panel (TOOLBOX) floating window ────────────────────────────────────────────

fn paint_panel_window(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    view: &crate::ui::view::InGameDesktopView,
    ingame: &mut InGameScreen,
    _map_px_y: u32,
    _map_px_h: u32,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);

    if !ingame.desktop.is_open(WindowId::Panel) {
        return;
    }

    let win = ingame.desktop.window(WindowId::Panel);
    let wx = win.x as u32 * cw;
    let wy = win.y as u32 * ch;
    let ww = (win.width as u32 * cw).min(width.saturating_sub(wx));
    let wh = (win.height as u32 * ch).min(height.saturating_sub(wy));

    if ww < 4 || wh < 4 {
        return;
    }

    // Shadow
    fill_rect(buf, width, wx + 3, wy + 3, ww, wh, SHADOW_COLOR);
    // Background
    fill_rect(buf, width, wx, wy, ww, wh, SIDEBAR_BG);
    // Border
    draw_rect_outline(buf, width, wx, wy, ww, wh, BORDER_FG);
    // Title bar
    fill_rect(buf, width, wx + 1, wy, ww - 2, ch, WIN_TITLE_BG);
    draw_str(buf, width, wx + cw, wy, " TOOLBOX ", WIN_TITLE_FG, WIN_TITLE_BG, scale);
    // [X] close button
    if ww >= 5 * cw {
        draw_str(buf, width, wx + ww - 4 * cw, wy, "[X]", DANGER_FG, WIN_TITLE_BG, scale);
    }

    // Content area (inside border, below title bar)
    let cx = wx + cw;
    let cy = wy + ch + ch / 2; // inner top: 1px border + title row + small padding
    let iw = ww.saturating_sub(cw * 2);
    let cols = (iw / cw) as usize;

    let mut row_y = cy;

    // ── Toolbar rows ──────────────────────────────────────────────────────
    let tb = &view.toolbar;

    let tool_rows: &[(&str, Tool, u32)] = &[
        ("? Inspect", Tool::Inspect, 0xaaaaaa),
        ("B Bulldoze", Tool::Bulldoze, 0xff6644),
    ];
    for (label, tool, color) in tool_rows {
        if row_y + ch > wy + wh {
            break;
        }
        let active = tb.current_tool == *tool;
        let (fg, bg) = if active { (SELECT_FG, SELECT_BG) } else { (*color, BTN_BG) };
        fill_rect(buf, width, cx, row_y, iw, ch, bg);
        draw_str(buf, width, cx, row_y, &trunc(label, cols), fg, bg, scale);
        ingame.ui_areas.toolbar_items.push(ToolbarHitArea {
            area: pixel_click_area(cx, row_y, iw, ch, scale),
            target: ToolbarHitTarget::SelectTool(*tool),
        });
        row_y += ch;
    }

    let chooser_rows: &[(&str, ToolChooserKind, Tool, u32)] = &[
        ("Zones", ToolChooserKind::Zones, tb.zone_tool, RES_FG),
        ("Transport", ToolChooserKind::Transport, tb.transport_tool, 0xaaaaaa),
        ("Utilities", ToolChooserKind::Utilities, tb.utility_tool, POWER_FG),
        ("Plants", ToolChooserKind::PowerPlants, tb.power_plant_tool, 0xff4444),
        ("Buildings", ToolChooserKind::Buildings, tb.building_tool, COMM_FG),
    ];
    for (prefix, kind, tool, color) in chooser_rows {
        if row_y + ch > wy + wh {
            break;
        }
        let active = tb.current_tool == *tool;
        let (fg, bg) = if active { (SELECT_FG, BTN_SEL_BG) } else { (*color, BTN_BG) };
        let label = format!("{}: {}", prefix, tool.label());
        fill_rect(buf, width, cx, row_y, iw, ch, bg);
        draw_str(buf, width, cx, row_y, &trunc(&label, cols), fg, bg, scale);
        ingame.ui_areas.toolbar_items.push(ToolbarHitArea {
            area: pixel_click_area(cx, row_y, iw, ch, scale),
            target: ToolbarHitTarget::OpenChooser(*kind),
        });
        row_y += ch;
    }

    // Tool cost
    let cost = view.current_tool.cost();
    if cost > 0 && row_y + ch <= wy + wh {
        let cost_str = format!("${}", cost);
        fill_rect(buf, width, cx, row_y, iw, ch, SIDEBAR_BG);
        draw_str(buf, width, cx, row_y, &cost_str, DIM_FG, SIDEBAR_BG, scale);
        row_y += ch;
    }

    // Divider
    if row_y + 2 <= wy + wh {
        font::hline(buf, width, cx, row_y, iw, BORDER_FG);
        row_y += 1;
    }

    // ── RCI demand ────────────────────────────────────────────────────────
    if row_y + ch <= wy + wh {
        fill_rect(buf, width, cx, row_y, iw, ch, SIDEBAR_BG);
        draw_str(buf, width, cx, row_y, "DEMAND:", DIM_FG, SIDEBAR_BG, scale);
        row_y += ch;
    }

    let bar_cols = cols.saturating_sub(3);
    for (label, demand, color) in [
        ("R:", view.sim.demand.res, RES_FG),
        ("C:", view.sim.demand.comm, COMM_FG),
        ("I:", view.sim.demand.ind, IND_FG),
    ] {
        if row_y + ch > wy + wh {
            break;
        }
        let fill = ((demand.clamp(0.0, 1.0) * bar_cols as f32) as usize).min(bar_cols);
        let bar: String = (0..bar_cols).map(|i| if i < fill { '#' } else { '.' }).collect();
        fill_rect(buf, width, cx, row_y, iw, ch, SIDEBAR_BG);
        draw_str(buf, width, cx, row_y, label, DIM_FG, SIDEBAR_BG, scale);
        let bar_x = cx + 3 * cw;
        let filled_bar: String = bar.chars().take(fill).collect();
        let empty_bar: String = bar.chars().skip(fill).collect();
        draw_str(buf, width, bar_x, row_y, &filled_bar, color, SIDEBAR_BG, scale);
        draw_str(buf, width, bar_x + fill as u32 * cw, row_y, &empty_bar, DIM_FG, SIDEBAR_BG, scale);
        row_y += ch;
    }

    // Power summary
    if row_y + ch <= wy + wh {
        let util = &view.sim.utilities;
        let surplus = util.power_produced_mw as i32 - util.power_consumed_mw as i32;
        let pw_color = if surplus >= 0 { 0x66cc44u32 } else { 0xff4444u32 };
        let pw_str = format!("Pwr:{}/{}MW", util.power_produced_mw, util.power_consumed_mw);
        fill_rect(buf, width, cx, row_y, iw, ch, SIDEBAR_BG);
        draw_str(buf, width, cx, row_y, &trunc(&pw_str, cols), pw_color, SIDEBAR_BG, scale);
        row_y += ch;
    }

    // Cursor tile info
    if let Some((tile_cx, tile_cy)) = view.inspect_pos {
        if tile_cx < view.map.width && tile_cy < view.map.height {
            let idx = view.map.idx(tile_cx, tile_cy);
            let tile = view.map.tiles[idx];
            let tile_ov = view.map.overlays[idx];
            if row_y + ch <= wy + wh {
                let pos_str = format!("({},{})", tile_cx, tile_cy);
                fill_rect(buf, width, cx, row_y, iw, ch, SIDEBAR_BG);
                draw_str(buf, width, cx, row_y, &pos_str, DIM_FG, SIDEBAR_BG, scale);
                row_y += ch;
            }
            if row_y + ch <= wy + wh {
                fill_rect(buf, width, cx, row_y, iw, ch, SIDEBAR_BG);
                draw_str(
                    buf, width, cx, row_y,
                    &trunc(tile.name(), cols),
                    TITLE_FG, SIDEBAR_BG, scale,
                );
                row_y += ch;
            }
            if tile_ov.power_level > 0 && row_y + ch <= wy + wh {
                let pct = tile_ov.power_level as u32 * 100 / 255;
                let s = format!("Pwr {}%", pct);
                fill_rect(buf, width, cx, row_y, iw, ch, SIDEBAR_BG);
                draw_str(buf, width, cx, row_y, &s, POWER_FG, SIDEBAR_BG, scale);
                row_y += ch;
            }
        }
    }

    // Divider before minimap
    if row_y + 2 <= wy + wh {
        font::hline(buf, width, cx, row_y, iw, BORDER_FG);
        row_y += 1;
    }

    // ── Minimap ───────────────────────────────────────────────────────────
    let minimap_avail_h = (wy + wh).saturating_sub(row_y + ch + 2);
    if minimap_avail_h > 8 && iw > 8 {
        if row_y + ch <= wy + wh {
            fill_rect(buf, width, cx, row_y, iw, ch, SIDEBAR_BG);
            draw_str(buf, width, cx, row_y, "MINIMAP:", DIM_FG, SIDEBAR_BG, scale);
            row_y += ch;
        }

        let mw = view.map.width;
        let mh = view.map.height;
        let mm_w = iw.min(mw as u32 * 2);
        let mm_h = minimap_avail_h.min(mh as u32);
        let mm_x = cx + iw.saturating_sub(mm_w) / 2;
        let mm_y = row_y;

        fill_rect(buf, width, mm_x, mm_y, mm_w, mm_h, MINIMAP_FRAME);

        let cols_count = (mm_w / 2) as usize;
        let rows_count = mm_h as usize;

        for row in 0..rows_count {
            for col in 0..cols_count {
                let map_x =
                    if cols_count <= 1 { 0 } else { col * (mw - 1) / (cols_count - 1) };
                let map_y =
                    if rows_count <= 1 { 0 } else { row * (mh - 1) / (rows_count - 1) };
                let idx = view.map.idx(map_x, map_y);
                let tile = view.map.tiles[idx];
                let ov = view.map.overlays[idx];
                let glyph = theme::tile_glyph(tile, ov);
                let color = color_to_u32(glyph.bg);
                let px = mm_x + col as u32 * 2;
                let py = mm_y + row as u32;
                fill_rect(buf, width, px, py, 2, 1, color);
            }
        }

        // Viewport rectangle on minimap
        let cam = &view.camera;
        let vx0 = if mw <= 1 || cols_count <= 1 {
            0u32
        } else {
            (cam.offset_x.max(0) as u32 * (cols_count as u32 - 1) / (mw as u32 - 1)) * 2
        };
        let vy0 = if mh <= 1 || rows_count <= 1 {
            0u32
        } else {
            cam.offset_y.max(0) as u32 * (rows_count as u32 - 1) / (mh as u32 - 1)
        };
        let vx1 = if mw <= 1 || cols_count <= 1 {
            mm_w.saturating_sub(1)
        } else {
            (((cam.offset_x + cam.view_w as i32).max(0) as u32).min(mw as u32 - 1)
                * (cols_count as u32 - 1)
                / (mw as u32 - 1))
                * 2
        };
        let vy1 = if mh <= 1 || rows_count <= 1 {
            mm_h.saturating_sub(1)
        } else {
            ((cam.offset_y + cam.view_h as i32).max(0) as u32).min(mh as u32 - 1)
                * (rows_count as u32 - 1)
                / (mh as u32 - 1)
        };
        let ax = mm_x + vx0.min(mm_w.saturating_sub(1));
        let ay = mm_y + vy0.min(mm_h.saturating_sub(1));
        let bx = mm_x + vx1.min(mm_w.saturating_sub(1));
        let by = mm_y + vy1.min(mm_h.saturating_sub(1));
        font::hline(buf, width, ax, ay, bx.saturating_sub(ax) + 1, 0xffffff);
        font::hline(buf, width, ax, by, bx.saturating_sub(ax) + 1, 0xffffff);
        font::vline(buf, width, ax, ay, by.saturating_sub(ay) + 1, 0xffffff);
        font::vline(buf, width, bx, ay, by.saturating_sub(ay) + 1, 0xffffff);

        ingame.ui_areas.minimap = pixel_click_area(mm_x, mm_y, mm_w, mm_h, scale);
    }
}

// ── Tool chooser popup ─────────────────────────────────────────────────────────

fn paint_tool_chooser(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    chooser: &crate::ui::view::ToolChooserViewModel,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);

    let max_name = chooser.tools.iter().map(|t| t.label().len()).max().unwrap_or(10);
    let pop_cols = (max_name + 12).max(24) as u32;
    let pop_rows = (chooser.tools.len() + 3) as u32;
    let pop_w = pop_cols * cw;
    let pop_h = pop_rows * ch;
    let px = width.saturating_sub(pop_w) / 2;
    let py = height.saturating_sub(pop_h) / 2;

    // Shadow
    fill_rect(buf, width, px + 3, py + 3, pop_w, pop_h, SHADOW_COLOR);
    // Background + border
    fill_rect(buf, width, px, py, pop_w, pop_h, POPUP_BG);
    draw_rect_outline(buf, width, px, py, pop_w, pop_h, BORDER_FG);
    // Title
    fill_rect(buf, width, px + 1, py, pop_w - 2, ch, WIN_TITLE_BG);
    let title = "Tool Selection";
    draw_str(buf, width, px + cw, py, title, WIN_TITLE_FG, WIN_TITLE_BG, scale);
    // [X] close
    draw_str(buf, width, px + pop_w - 4 * cw, py, "[X]", DANGER_FG, WIN_TITLE_BG, scale);

    for (i, tool) in chooser.tools.iter().enumerate() {
        let item_y = py + ch + i as u32 * ch;
        let selected = *tool == chooser.selected_tool;
        let (fg, bg) = if selected { (SELECT_FG, SELECT_BG) } else { (TEXT_FG, POPUP_BG) };
        fill_rect(buf, width, px + 1, item_y, pop_w - 2, ch, bg);
        let cost = tool.cost();
        let label = if cost > 0 {
            format!(" {:<20} ${}", tool.label(), cost)
        } else {
            format!(" {}", tool.label())
        };
        draw_str(buf, width, px + 1, item_y, &trunc(&label, pop_cols as usize - 1), fg, bg, scale);
        ingame.ui_areas.tool_chooser_items.push(pixel_click_area(px + 1, item_y, pop_w - 2, ch, scale));
    }
}

// ── Confirm dialog ─────────────────────────────────────────────────────────────

fn paint_confirm_dialog(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    dialog: &crate::ui::view::ConfirmDialogViewModel,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let pop_cols: u32 = 36;
    let msg_rows = (dialog.message.len() as u32 / pop_cols + 2).max(2);
    let pop_rows = 3 + msg_rows + 2; // title + gap + msg + gap + buttons
    let pop_w = pop_cols * cw;
    let pop_h = pop_rows * ch;
    let px = width.saturating_sub(pop_w) / 2;
    let py = height.saturating_sub(pop_h) / 2;

    // Shadow + background + border
    fill_rect(buf, width, px + 3, py + 3, pop_w, pop_h, SHADOW_COLOR);
    fill_rect(buf, width, px, py, pop_w, pop_h, POPUP_BG);
    draw_rect_outline(buf, width, px, py, pop_w, pop_h, BORDER_FG);

    // Title bar
    fill_rect(buf, width, px + 1, py, pop_w - 2, ch, WIN_TITLE_BG);
    draw_str(buf, width, px + cw, py, &trunc(&dialog.title, pop_cols as usize - 2), WIN_TITLE_FG, WIN_TITLE_BG, scale);

    // Message (wrap at pop_cols - 2)
    let msg_x = px + cw;
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
        if lines.len() >= msg_rows as usize {
            break;
        }
    }
    for (i, line) in lines.iter().enumerate() {
        let ly = py + ch + i as u32 * ch;
        fill_rect(buf, width, px + 1, ly, pop_w - 2, ch, POPUP_BG);
        draw_str(buf, width, msg_x, ly, line, TEXT_FG, POPUP_BG, scale);
    }

    // Button row
    let btn_y = py + pop_h - 2 * ch;
    fill_rect(buf, width, px + 1, btn_y, pop_w - 2, ch, POPUP_BG);
    let btn_total_w: u32 = dialog.buttons.iter().map(|b| (b.label.len() as u32 + 4) * cw).sum();
    let mut bx = px + pop_w.saturating_sub(btn_total_w) / 2;
    for (i, btn) in dialog.buttons.iter().enumerate() {
        let bw = (btn.label.len() as u32 + 4) * cw;
        let selected = i == dialog.selected;
        let (fg, bg) = if selected { (SELECT_FG, SELECT_BG) } else { (TEXT_FG, BTN_BG) };
        fill_rect(buf, width, bx, btn_y, bw, ch, bg);
        let text = format!("[ {} ]", btn.label);
        draw_str(buf, width, bx, btn_y, &text, fg, bg, scale);
        ingame.ui_areas.dialog_items.push(pixel_click_area(bx, btn_y, bw, ch, scale));
        bx += bw + cw;
    }
}

// ── Budget window ──────────────────────────────────────────────────────────────

fn paint_budget_window(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    view: &crate::ui::view::InGameDesktopView,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let win = ingame.desktop.window(WindowId::Budget);
    let wx = win.x as u32 * cw;
    let wy = win.y as u32 * ch;
    let ww = (win.width as u32 * cw).min(width.saturating_sub(wx));
    let wh = (win.height as u32 * ch).min(height.saturating_sub(wy));
    if ww < 8 || wh < 6 {
        return;
    }

    fill_rect(buf, width, wx + 3, wy + 3, ww, wh, SHADOW_COLOR);
    fill_rect(buf, width, wx, wy, ww, wh, POPUP_BG);
    draw_rect_outline(buf, width, wx, wy, ww, wh, BORDER_FG);
    fill_rect(buf, width, wx + 1, wy, ww - 2, ch, WIN_TITLE_BG);
    draw_str(buf, width, wx + cw, wy, "Budget Control Center", WIN_TITLE_FG, WIN_TITLE_BG, scale);
    if ww >= 5 * cw {
        draw_str(buf, width, wx + ww - 4 * cw, wy, "[X]", DANGER_FG, WIN_TITLE_BG, scale);
    }

    let bud = &view.budget;
    let ix = wx + cw;
    let mut iy = wy + ch + ch / 2;
    let icols = ((ww / cw).saturating_sub(2)) as usize;

    // Tax rates
    for (label, pct) in [
        ("Residential Tax", bud.tax_rates.residential),
        ("Commercial Tax ", bud.tax_rates.commercial),
        ("Industrial Tax  ", bud.tax_rates.industrial),
    ] {
        if iy + ch > wy + wh {
            break;
        }
        let s = format!("{}: {}%", label, pct);
        fill_rect(buf, width, ix, iy, ww - 2 * cw, ch, POPUP_BG);
        draw_str(buf, width, ix, iy, &trunc(&s, icols), TEXT_FG, POPUP_BG, scale);
        iy += ch;
    }

    if iy + 1 <= wy + wh {
        font::hline(buf, width, ix, iy, ww - 2 * cw, BORDER_FG);
        iy += 1;
    }

    // Financials
    let income_sign = if bud.current_annual_tax >= 0 { "+" } else { "" };
    for (label, val, color) in [
        ("Treasury", format!("${}", bud.treasury), if bud.treasury >= 0 { RES_FG } else { DANGER_FG }),
        ("Annual Tax", format!("{}{}", income_sign, bud.current_annual_tax), TEXT_FG),
    ] {
        if iy + ch > wy + wh {
            break;
        }
        let s = format!("{}: {}", label, val);
        fill_rect(buf, width, ix, iy, ww - 2 * cw, ch, POPUP_BG);
        draw_str(buf, width, ix, iy, &trunc(&s, icols), color, POPUP_BG, scale);
        iy += ch;
    }
}

// ── Statistics window ──────────────────────────────────────────────────────────

fn paint_statistics_window(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    stats: &crate::ui::view::StatisticsWindowViewModel,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let win = ingame.desktop.window(WindowId::Statistics);
    let wx = win.x as u32 * cw;
    let wy = win.y as u32 * ch;
    let ww = (win.width as u32 * cw).min(width.saturating_sub(wx));
    let wh = (win.height as u32 * ch).min(height.saturating_sub(wy));
    if ww < 8 || wh < 6 {
        return;
    }

    paint_window_chrome(buf, width, wx, wy, ww, wh, ch, cw, scale, "City Statistics");

    let ix = wx + cw;
    let mut iy = wy + ch + ch / 2;
    let icols = ((ww / cw).saturating_sub(2)) as usize;

    let lines = [
        format!("{}", stats.city_name),
        format!("Population: {}", stats.current_population),
        format!("Treasury: ${}", stats.current_treasury),
        format!("Annual Income: ${}", stats.current_income),
        format!(
            "Power: {} / {} MW",
            stats.current_power_consumed, stats.current_power_produced
        ),
        String::new(),
        format!("History: {} months", stats.treasury_history.len()),
    ];

    // Summary lines
    for line in &lines {
        if iy + ch > wy + wh {
            break;
        }
        fill_rect(buf, width, ix, iy, ww - 2 * cw, ch, POPUP_BG);
        draw_str(buf, width, ix, iy, &trunc(line, icols), TEXT_FG, POPUP_BG, scale);
        iy += ch;
    }

    // Simple sparkline charts
    if iy + ch <= wy + wh {
        iy += ch / 2;
    }
    let chart_w = ((ww - 2 * cw) / cw) as usize;
    for (label, data, color) in [
        ("Treasury", stats_sparkline_i64(&stats.treasury_history, chart_w), BORDER_FG),
        ("Population", stats_sparkline_u64(&stats.population_history, chart_w), RES_FG),
        ("Income", stats_sparkline_i64(&stats.income_history, chart_w), IND_FG),
    ] {
        if iy + ch * 2 > wy + wh {
            break;
        }
        fill_rect(buf, width, ix, iy, ww - 2 * cw, ch, POPUP_BG);
        draw_str(buf, width, ix, iy, &trunc(label, icols), DIM_FG, POPUP_BG, scale);
        iy += ch;
        fill_rect(buf, width, ix, iy, ww - 2 * cw, ch, POPUP_BG);
        draw_str(buf, width, ix, iy, &trunc(&data, icols), color, POPUP_BG, scale);
        iy += ch;
    }
}

fn stats_sparkline_i64(data: &std::collections::VecDeque<i64>, width: usize) -> String {
    if data.is_empty() || width == 0 {
        return String::new();
    }
    let sparks = &['_', '.', '-', '~', '+', '*', '#', '@'];
    let min = data.iter().copied().min().unwrap_or(0);
    let max = data.iter().copied().max().unwrap_or(0);
    let range = (max - min).max(1) as f64;
    let step = data.len().max(1) as f64 / width as f64;
    (0..width)
        .map(|i| {
            let idx = ((i as f64 * step) as usize).min(data.len().saturating_sub(1));
            let val = data.get(idx).copied().unwrap_or(0);
            let norm = ((val - min) as f64 / range * 7.0) as usize;
            sparks[norm.min(7)]
        })
        .collect()
}

fn stats_sparkline_u64(data: &std::collections::VecDeque<u64>, width: usize) -> String {
    if data.is_empty() || width == 0 {
        return String::new();
    }
    let sparks = &['_', '.', '-', '~', '+', '*', '#', '@'];
    let min = data.iter().copied().min().unwrap_or(0);
    let max = data.iter().copied().max().unwrap_or(0);
    let range = (max - min).max(1) as f64;
    let step = data.len().max(1) as f64 / width as f64;
    (0..width)
        .map(|i| {
            let idx = ((i as f64 * step) as usize).min(data.len().saturating_sub(1));
            let val = data.get(idx).copied().unwrap_or(0);
            let norm = ((val - min) as f64 / range * 7.0) as usize;
            sparks[norm.min(7)]
        })
        .collect()
}

// ── Inspect window ────────────────────────────────────────────────────────────

fn paint_inspect_window(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    view: &crate::ui::view::InGameDesktopView,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let win = ingame.desktop.window(WindowId::Inspect);
    let wx = win.x as u32 * cw;
    let wy = win.y as u32 * ch;
    let ww = (win.width as u32 * cw).min(width.saturating_sub(wx));
    let wh = (win.height as u32 * ch).min(height.saturating_sub(wy));
    if ww < 8 || wh < 6 {
        return;
    }

    paint_window_chrome(buf, width, wx, wy, ww, wh, ch, cw, scale, "Inspect");

    let ix = wx + cw;
    let mut iy = wy + ch + ch / 2;
    let icols = ((ww / cw).saturating_sub(2)) as usize;

    if let Some((tx, ty)) = view.inspect_pos {
        if tx < view.map.width && ty < view.map.height {
            let idx = view.map.idx(tx, ty);
            let tile = view.map.tiles[idx];
            let ov = view.map.overlays[idx];

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
                if iy + ch > wy + wh {
                    break;
                }
                fill_rect(buf, width, ix, iy, ww - 2 * cw, ch, POPUP_BG);
                draw_str(buf, width, ix, iy, &trunc(line, icols), TEXT_FG, POPUP_BG, scale);
                iy += ch;
            }
        }
    } else {
        fill_rect(buf, width, ix, iy, ww - 2 * cw, ch, POPUP_BG);
        draw_str(buf, width, ix, iy, "Move cursor to inspect", DIM_FG, POPUP_BG, scale);
    }
}

// ── Text window (Help / About / Legend) ───────────────────────────────────────

fn paint_text_window(
    buf: &mut [u32],
    width: u32,
    height: u32,
    scale: u32,
    window_id: WindowId,
    view: &crate::ui::view::TextWindowViewModel,
    ingame: &mut InGameScreen,
) {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    let win = ingame.desktop.window(window_id);
    let title = win.title;
    let wx = win.x as u32 * cw;
    let wy = win.y as u32 * ch;
    let ww = (win.width as u32 * cw).min(width.saturating_sub(wx));
    let wh = (win.height as u32 * ch).min(height.saturating_sub(wy));
    if ww < 8 || wh < 6 {
        return;
    }

    paint_window_chrome(buf, width, wx, wy, ww, wh, ch, cw, scale, title);

    let ix = wx + cw;
    let iy_start = wy + ch + ch / 2;
    let icols = ((ww / cw).saturating_sub(2)) as usize;
    let visible_rows = ((wh - ch - ch / 2) / ch) as usize;

    let scroll = view.scroll_y as usize;
    for (i, line) in view.lines.iter().skip(scroll).take(visible_rows).enumerate() {
        let iy = iy_start + i as u32 * ch;
        fill_rect(buf, width, ix, iy, ww - 2 * cw, ch, POPUP_BG);
        draw_str(buf, width, ix, iy, &trunc(line, icols), TEXT_FG, POPUP_BG, scale);
    }

    // Update content_height for scrollbar hit-testing
    let win = ingame.desktop.window_mut(window_id);
    win.content_height = view.lines.len() as u16;
}

// ── Window chrome helper ──────────────────────────────────────────────────────

fn paint_window_chrome(
    buf: &mut [u32],
    width: u32,
    wx: u32,
    wy: u32,
    ww: u32,
    wh: u32,
    ch: u32,
    cw: u32,
    scale: u32,
    title: &str,
) {
    // Shadow
    fill_rect(buf, width, wx + 3, wy + 3, ww, wh, SHADOW_COLOR);
    // Background
    fill_rect(buf, width, wx, wy, ww, wh, POPUP_BG);
    // Border
    draw_rect_outline(buf, width, wx, wy, ww, wh, BORDER_FG);
    // Title bar
    fill_rect(buf, width, wx + 1, wy, ww - 2, ch, WIN_TITLE_BG);
    draw_str(buf, width, wx + cw, wy, title, WIN_TITLE_FG, WIN_TITLE_BG, scale);
    // [X] close button
    if ww >= 5 * cw {
        draw_str(buf, width, wx + ww - 4 * cw, wy, "[X]", DANGER_FG, WIN_TITLE_BG, scale);
    }
}

// ── Overlay helpers ────────────────────────────────────────────────────────────

fn overlay_value(overlay: TileOverlay, mode: crate::ui::theme::OverlayMode) -> u8 {
    use crate::ui::theme::OverlayMode;
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

// ── Helpers ────────────────────────────────────────────────────────────────────

fn center_x(width: u32, char_count: u32, scale: u32) -> u32 {
    width.saturating_sub(char_count * cell_w(scale)) / 2
}

fn trunc(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Convert a pixel rect to cell-coord ClickArea for the existing hit-test system.
/// x is NOT doubled — only the map viewport needs the /2 compensation.
fn pixel_click_area(px: u32, py: u32, pw: u32, ph: u32, scale: u32) -> ClickArea {
    let cw = cell_w(scale);
    let ch = cell_h(scale);
    ClickArea {
        x: (px / cw) as u16,
        y: (py / ch) as u16,
        width: (pw / cw).max(1) as u16,
        height: (ph / ch).max(1) as u16,
    }
}
