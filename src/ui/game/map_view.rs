use crate::{
    app::camera::Camera,
    core::{map::Map, map::Tile, map::TileOverlay, map::ViewLayer, tool::Tool},
    ui::runtime::{scrollbar_metrics, scrollbar_offset_from_pointer},
    ui::theme::{self, OverlayMode},
};
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

pub enum PreviewKind {
    None,
    Line(Tool),
    Rect(Tool),
    /// Multi-tile building footprint preview. `bool` = all tiles are placeable.
    Footprint(Tool, bool),
}

pub struct MapView<'a> {
    pub map: &'a Map,
    pub camera: &'a Camera,
    /// Active drag preview tiles (line or rect), or empty slice when no drag is in progress.
    pub line_preview: &'a [(usize, usize)],
    /// The kind of preview currently active.
    pub preview_kind: PreviewKind,
    /// Current heat-map overlay mode.
    pub overlay_mode: OverlayMode,
    pub view_layer: ViewLayer,
}

const UNPOWERED_WARNING_MARKER: char = '!';
const TRAFFIC_ANIMATION_THRESHOLD: u8 = 24;
const TRAFFIC_ANIMATION_PERIOD: u32 = 8;
const TRAFFIC_ANIMATION_STEP_MS: u128 = 280;
const FIRE_ANIMATION_PERIOD: u32 = 4;
const FIRE_ANIMATION_STEP_MS: u128 = 180;
const UTILITY_ANIMATION_PERIOD: u32 = 6;
const UTILITY_ANIMATION_STEP_MS: u128 = 220;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapChromeLayout {
    pub viewport: Rect,
    pub view_tiles_w: usize,
    pub view_tiles_h: usize,
    pub vertical_bar: Rect,
    pub vertical_dec: Rect,
    pub vertical_track: Rect,
    pub vertical_thumb: Rect,
    pub vertical_inc: Rect,
    pub vertical_page_step: usize,
    pub horizontal_bar: Rect,
    pub horizontal_dec: Rect,
    pub horizontal_track: Rect,
    pub horizontal_thumb: Rect,
    pub horizontal_inc: Rect,
    pub horizontal_page_step: usize,
    pub corner: Rect,
}

fn split_vertical_bar(bar: Rect) -> (Rect, Rect, Rect) {
    if bar.width == 0 || bar.height == 0 {
        return (Rect::default(), Rect::default(), Rect::default());
    }
    let dec = Rect::new(bar.x, bar.y, bar.width, 1);
    if bar.height == 1 {
        return (dec, Rect::default(), Rect::default());
    }
    let inc = Rect::new(bar.x, bar.y + bar.height - 1, bar.width, 1);
    let track = if bar.height > 2 {
        Rect::new(bar.x, bar.y + 1, bar.width, bar.height - 2)
    } else {
        Rect::default()
    };
    (dec, track, inc)
}

fn split_horizontal_bar(bar: Rect) -> (Rect, Rect, Rect) {
    if bar.width == 0 || bar.height == 0 {
        return (Rect::default(), Rect::default(), Rect::default());
    }
    let dec = Rect::new(bar.x, bar.y, 1, bar.height);
    if bar.width == 1 {
        return (dec, Rect::default(), Rect::default());
    }
    let inc = Rect::new(bar.x + bar.width - 1, bar.y, 1, bar.height);
    let track = if bar.width > 2 {
        Rect::new(bar.x + 1, bar.y, bar.width - 2, bar.height)
    } else {
        Rect::default()
    };
    (dec, track, inc)
}

pub fn layout_map_chrome(
    area: Rect,
    map_w: usize,
    map_h: usize,
    offset_x: usize,
    offset_y: usize,
) -> MapChromeLayout {
    if area.width == 0 || area.height == 0 {
        return MapChromeLayout::default();
    }

    let mut viewport_w = area.width;
    let mut viewport_h = area.height;

    let (need_h, need_v) = loop {
        let view_tiles_w = (viewport_w as usize / 2).max(1);
        let next_need_h = map_w > view_tiles_w;
        let next_need_v = map_h > viewport_h as usize;

        let next_viewport_w = area.width.saturating_sub(u16::from(next_need_v));
        let next_viewport_h = area.height.saturating_sub(u16::from(next_need_h));

        if next_viewport_w == viewport_w && next_viewport_h == viewport_h {
            break (next_need_h, next_need_v);
        }

        viewport_w = next_viewport_w;
        viewport_h = next_viewport_h;
    };

    let viewport = Rect::new(area.x, area.y, viewport_w, viewport_h);
    let view_tiles_w = (viewport.width as usize / 2).max(1);
    let view_tiles_h = viewport.height as usize;

    let vertical_bar = if need_v {
        let h = if need_h { viewport.height } else { area.height };
        Rect::new(viewport.x + viewport.width, area.y, 1, h)
    } else {
        Rect::default()
    };
    let horizontal_bar = if need_h {
        let w = if need_v { viewport.width } else { area.width };
        Rect::new(area.x, viewport.y + viewport.height, w, 1)
    } else {
        Rect::default()
    };
    let corner = if need_h && need_v {
        Rect::new(
            viewport.x + viewport.width,
            viewport.y + viewport.height,
            1,
            1,
        )
    } else {
        Rect::default()
    };

    let (vertical_dec, vertical_track, vertical_inc) = split_vertical_bar(vertical_bar);
    let vertical_thumb = if let Some(metrics) =
        scrollbar_metrics(vertical_track.height, view_tiles_h, map_h, offset_y)
    {
        Rect::new(
            vertical_track.x,
            vertical_track.y + metrics.thumb_start,
            vertical_track.width,
            metrics.thumb_len,
        )
    } else {
        Rect::default()
    };

    let (horizontal_dec, horizontal_track, horizontal_inc) = split_horizontal_bar(horizontal_bar);
    let horizontal_thumb = if let Some(metrics) =
        scrollbar_metrics(horizontal_track.width, view_tiles_w, map_w, offset_x)
    {
        Rect::new(
            horizontal_track.x + metrics.thumb_start,
            horizontal_track.y,
            metrics.thumb_len,
            horizontal_track.height,
        )
    } else {
        Rect::default()
    };

    MapChromeLayout {
        viewport,
        view_tiles_w,
        view_tiles_h,
        vertical_bar,
        vertical_dec,
        vertical_track,
        vertical_thumb,
        vertical_inc,
        vertical_page_step: view_tiles_h.saturating_sub(2).max(1),
        horizontal_bar,
        horizontal_dec,
        horizontal_track,
        horizontal_thumb,
        horizontal_inc,
        horizontal_page_step: view_tiles_w.saturating_sub(2).max(1),
        corner,
    }
}

pub fn render_scrollbars(layout: &MapChromeLayout, buf: &mut Buffer) {
    let ui = theme::ui_palette();
    let button_fg = ui.scrollbar_button_fg;
    let button_bg = ui.scrollbar_button_bg;
    let track_fg = ui.scrollbar_track_fg;
    let track_bg = ui.scrollbar_track_bg;
    let thumb_fg = ui.scrollbar_thumb_fg;
    let thumb_bg = ui.scrollbar_thumb_bg;
    let corner_fg = ui.scrollbar_corner_fg;
    let corner_bg = ui.scrollbar_corner_bg;

    let fill = |buf: &mut Buffer, area: Rect, ch: char, fg: Color, bg: Color| {
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_char(ch);
                cell.set_fg(fg);
                cell.set_bg(bg);
            }
        }
    };

    if layout.vertical_bar.width > 0 {
        fill(buf, layout.vertical_bar, '▒', track_fg, track_bg);
        fill(buf, layout.vertical_thumb, '█', thumb_fg, thumb_bg);
        if layout.vertical_dec.width > 0 {
            let cell = buf
                .cell_mut((layout.vertical_dec.x, layout.vertical_dec.y))
                .unwrap();
            cell.set_char('▲');
            cell.set_fg(button_fg);
            cell.set_bg(button_bg);
        }
        if layout.vertical_inc.width > 0 {
            let cell = buf
                .cell_mut((layout.vertical_inc.x, layout.vertical_inc.y))
                .unwrap();
            cell.set_char('▼');
            cell.set_fg(button_fg);
            cell.set_bg(button_bg);
        }
    }

    if layout.horizontal_bar.width > 0 {
        fill(buf, layout.horizontal_bar, '▒', track_fg, track_bg);
        fill(buf, layout.horizontal_thumb, '█', thumb_fg, thumb_bg);
        if layout.horizontal_dec.width > 0 {
            let cell = buf
                .cell_mut((layout.horizontal_dec.x, layout.horizontal_dec.y))
                .unwrap();
            cell.set_char('◄');
            cell.set_fg(button_fg);
            cell.set_bg(button_bg);
        }
        if layout.horizontal_inc.width > 0 {
            let cell = buf
                .cell_mut((layout.horizontal_inc.x, layout.horizontal_inc.y))
                .unwrap();
            cell.set_char('►');
            cell.set_fg(button_fg);
            cell.set_bg(button_bg);
        }
    }

    if layout.corner.width > 0 {
        fill(buf, layout.corner, '▒', corner_fg, corner_bg);
    }
}

/// N/E/S/W connectivity flags for a committed map tile.
fn map_connectivity(
    map: &Map,
    layer: ViewLayer,
    tile: Tile,
    x: usize,
    y: usize,
) -> (bool, bool, bool, bool) {
    let matches_tile = |t: Tile| match tile {
        Tile::Road | Tile::RoadPowerLine | Tile::Onramp => t.road_connects(),
        Tile::Highway => t == Tile::Highway || t == Tile::Onramp,
        Tile::PowerLine => t.power_connects(),
        Tile::WaterPipe => t.water_connects(),
        Tile::SubwayTunnel => t.subway_connects() || t == Tile::SubwayStation,
        _ => t == tile,
    };
    let n = y
        .checked_sub(1)
        .map(|ny| matches_tile(map.view_tile(layer, x, ny)))
        .unwrap_or(false);
    let e = if x + 1 < map.width {
        matches_tile(map.view_tile(layer, x + 1, y))
    } else {
        false
    };
    let s = if y + 1 < map.height {
        matches_tile(map.view_tile(layer, x, y + 1))
    } else {
        false
    };
    let w = x
        .checked_sub(1)
        .map(|wx| matches_tile(map.view_tile(layer, wx, y)))
        .unwrap_or(false);
    (n, e, s, w)
}

/// N/E/S/W connectivity for a preview tile — connects to both the preview set
/// AND already-committed matching tiles, so the preview shows accurate junctions.
fn preview_connectivity(
    map: &Map,
    layer: ViewLayer,
    target: Tile,
    x: usize,
    y: usize,
    preview: &std::collections::HashSet<(usize, usize)>,
) -> (bool, bool, bool, bool) {
    let connects = |nx: usize, ny: usize| {
        if preview.contains(&(nx, ny)) {
            return true;
        }
        if nx >= map.width || ny >= map.height {
            return false;
        }
        let t = map.view_tile(layer, nx, ny);
        match target {
            Tile::Road | Tile::Onramp => t.road_connects(),
            Tile::Highway => t == Tile::Highway || t == Tile::Onramp,
            Tile::PowerLine => t.power_connects(),
            Tile::WaterPipe => t.water_connects(),
            Tile::SubwayTunnel => t.subway_connects() || t == Tile::SubwayStation,
            _ => t == target,
        }
    };
    let n = y.checked_sub(1).map(|ny| connects(x, ny)).unwrap_or(false);
    let e = if x + 1 < map.width {
        connects(x + 1, y)
    } else {
        false
    };
    let s = if y + 1 < map.height {
        connects(x, y + 1)
    } else {
        false
    };
    let w = x.checked_sub(1).map(|wx| connects(wx, y)).unwrap_or(false);
    (n, e, s, w)
}

pub(crate) fn committed_tile_sprite(
    map: &Map,
    layer: ViewLayer,
    tile: Tile,
    overlay: TileOverlay,
    x: usize,
    y: usize,
) -> theme::TileSprite {
    let glyph = theme::tile_glyph(tile, overlay);
    if matches!(
        tile,
        Tile::Road
            | Tile::Rail
            | Tile::PowerLine
            | Tile::RoadPowerLine
            | Tile::Highway
            | Tile::Onramp
            | Tile::WaterPipe
            | Tile::SubwayTunnel
    ) {
        let (n, e, s, w) = map_connectivity(map, layer, tile, x, y);
        theme::network_sprite(tile, n, e, s, w, glyph.fg, glyph.bg)
    } else {
        theme::tile_sprite(tile, overlay)
    }
}

fn traffic_animation_phase() -> u32 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / TRAFFIC_ANIMATION_STEP_MS
        % TRAFFIC_ANIMATION_PERIOD as u128) as u32
}

fn fire_animation_phase() -> u32 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / FIRE_ANIMATION_STEP_MS
        % FIRE_ANIMATION_PERIOD as u128) as u32
}

fn utility_animation_phase() -> u32 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / UTILITY_ANIMATION_STEP_MS
        % UTILITY_ANIMATION_PERIOD as u128) as u32
}

fn traffic_marker_char(tile: Tile, traffic: u8) -> char {
    match tile {
        Tile::Highway => {
            if traffic >= 160 {
                '■'
            } else {
                '▪'
            }
        }
        _ => {
            if traffic >= 160 {
                '•'
            } else {
                '·'
            }
        }
    }
}

fn traffic_marker_color(tile: Tile, traffic: u8) -> Color {
    match tile {
        Tile::Highway => {
            if traffic >= 160 {
                Color::Rgb(255, 238, 170)
            } else {
                Color::Rgb(255, 208, 120)
            }
        }
        _ => {
            if traffic >= 160 {
                Color::Rgb(215, 245, 255)
            } else {
                Color::Rgb(255, 228, 170)
            }
        }
    }
}

fn add_traffic_marker(
    mut sprite: theme::TileSprite,
    slot: usize,
    marker: char,
    fg: Color,
) -> theme::TileSprite {
    let cell = if slot == 0 {
        &mut sprite.left
    } else {
        &mut sprite.right
    };
    cell.ch = marker;
    cell.fg = fg;
    sprite
}

fn animate_transport_sprite(
    map: &Map,
    view_layer: ViewLayer,
    tile: Tile,
    overlay: TileOverlay,
    x: usize,
    y: usize,
    sprite: theme::TileSprite,
    phase: u32,
) -> theme::TileSprite {
    if view_layer != ViewLayer::Surface
        || !tile.is_drive_network()
        || overlay.traffic < TRAFFIC_ANIMATION_THRESHOLD
    {
        return sprite;
    }

    let (n, e, s, w) = map_connectivity(map, view_layer, tile, x, y);
    if !(n || e || s || w) {
        return sprite;
    }

    let local_phase = (phase + ((x as u32 * 3 + y as u32 * 5) % TRAFFIC_ANIMATION_PERIOD))
        % TRAFFIC_ANIMATION_PERIOD;
    let marker = traffic_marker_char(tile, overlay.traffic);
    let marker_fg = traffic_marker_color(tile, overlay.traffic);
    let has_horizontal = e || w;
    let has_vertical = n || s;
    let lane_slot = ((local_phase / 4) % 2) as usize;

    match (has_horizontal, has_vertical) {
        (true, false) => add_traffic_marker(sprite, lane_slot, marker, marker_fg),
        (false, true) => {
            if lane_slot == 1 {
                add_traffic_marker(sprite, 1, marker, marker_fg)
            } else {
                sprite
            }
        }
        (true, true) => add_traffic_marker(sprite, lane_slot, marker, marker_fg),
        (false, false) => sprite,
    }
}

fn animate_fire_sprite(
    mut sprite: theme::TileSprite,
    overlay: TileOverlay,
    phase: u32,
) -> theme::TileSprite {
    if !overlay.on_fire {
        return sprite;
    }

    let (left_ch, right_ch, fg, bg) = match phase % FIRE_ANIMATION_PERIOD {
        0 => ('*', '+', Color::Rgb(255, 220, 120), Color::Rgb(175, 55, 0)),
        1 => ('+', '*', Color::Rgb(255, 190, 70), Color::Rgb(145, 35, 0)),
        2 => ('x', '*', Color::Rgb(255, 230, 140), Color::Rgb(185, 65, 0)),
        _ => ('*', 'x', Color::Rgb(255, 175, 60), Color::Rgb(135, 28, 0)),
    };

    sprite.left.ch = left_ch;
    sprite.right.ch = right_ch;
    sprite.left.fg = fg;
    sprite.right.fg = fg;
    sprite.left.bg = bg;
    sprite.right.bg = bg;
    sprite
}

fn animate_power_overlay_sprite(
    map: &Map,
    view_layer: ViewLayer,
    overlay_mode: OverlayMode,
    tile: Tile,
    overlay: TileOverlay,
    x: usize,
    y: usize,
    sprite: theme::TileSprite,
    phase: u32,
) -> theme::TileSprite {
    if overlay_mode != OverlayMode::Power
        || view_layer != ViewLayer::Surface
        || !tile.power_connects()
        || overlay.power_level == 0
    {
        return sprite;
    }

    let (_, e, _, w) = map_connectivity(map, view_layer, tile, x, y);
    let has_horizontal = e || w;
    let local_phase = (phase + ((x as u32 * 2 + y as u32 * 3) % UTILITY_ANIMATION_PERIOD))
        % UTILITY_ANIMATION_PERIOD;
    let pulse_slot = ((local_phase / 3) % 2) as usize;
    let marker = if overlay.power_level >= 192 {
        '•'
    } else {
        '·'
    };
    let fg = if overlay.power_level >= 192 {
        Color::Rgb(255, 255, 170)
    } else {
        Color::Rgb(245, 220, 120)
    };

    if has_horizontal {
        add_traffic_marker(sprite, pulse_slot, marker, fg)
    } else if local_phase % 3 != 1 {
        add_traffic_marker(sprite, 1, marker, fg)
    } else {
        sprite
    }
}

fn animate_underground_water_sprite(
    map: &Map,
    view_layer: ViewLayer,
    overlay_mode: OverlayMode,
    tile: Tile,
    overlay: TileOverlay,
    x: usize,
    y: usize,
    sprite: theme::TileSprite,
    phase: u32,
) -> theme::TileSprite {
    if view_layer != ViewLayer::Underground
        || overlay_mode == OverlayMode::Water
        || tile != Tile::WaterPipe
        || overlay.water_service == 0
    {
        return sprite;
    }

    let (_, e, _, w) = map_connectivity(map, view_layer, tile, x, y);
    let has_horizontal = e || w;
    let local_phase =
        (phase + ((x as u32 + y as u32 * 2) % UTILITY_ANIMATION_PERIOD)) % UTILITY_ANIMATION_PERIOD;
    let pulse_slot = ((local_phase / 3) % 2) as usize;
    let marker = if overlay.water_service >= 192 {
        '◦'
    } else {
        '·'
    };
    let fg = if overlay.water_service >= 192 {
        Color::Rgb(200, 245, 255)
    } else {
        Color::Rgb(140, 215, 245)
    };

    if has_horizontal {
        add_traffic_marker(sprite, pulse_slot, marker, fg)
    } else if local_phase % 3 != 1 {
        add_traffic_marker(sprite, 1, marker, fg)
    } else {
        sprite
    }
}

fn animate_subway_station_sprite(
    view_layer: ViewLayer,
    tile: Tile,
    sprite: theme::TileSprite,
    phase: u32,
) -> theme::TileSprite {
    if view_layer != ViewLayer::Underground || tile != Tile::SubwayStation {
        return sprite;
    }

    match phase % UTILITY_ANIMATION_PERIOD {
        0 | 1 => {
            theme::TileSprite::pair('U', '•', Color::Rgb(230, 245, 255), Color::Rgb(40, 78, 125))
        }
        2 | 3 => {
            theme::TileSprite::pair('U', 'o', Color::Rgb(205, 228, 245), Color::Rgb(28, 64, 108))
        }
        _ => sprite,
    }
}

fn orientation_landmark_tile(tile: Tile) -> bool {
    tile.is_transport()
        || tile.is_service_building()
        || matches!(
            tile,
            Tile::PowerLine
                | Tile::WaterPipe
                | Tile::SubwayTunnel
                | Tile::SubwayStation
                | Tile::PowerPlantCoal
                | Tile::PowerPlantGas
                | Tile::Water
                | Tile::Trees
                | Tile::Dirt
        )
}

fn orientation_network_fallback(tile: Tile) -> Option<char> {
    match tile {
        Tile::Road | Tile::RoadPowerLine => Some('·'),
        Tile::Highway | Tile::Onramp => Some('█'),
        Tile::Rail => Some('┄'),
        Tile::PowerLine => Some('┆'),
        Tile::WaterPipe => Some('┈'),
        Tile::SubwayTunnel => Some('╍'),
        _ => None,
    }
}

fn water_overlay_mass_marker(tile: Tile) -> char {
    match tile {
        Tile::ZoneRes | Tile::ResLow | Tile::ResMed | Tile::ResHigh => '•',
        Tile::ZoneComm | Tile::CommLow | Tile::CommHigh => '▪',
        Tile::ZoneInd | Tile::IndLight | Tile::IndHeavy => '■',
        Tile::Rubble => '░',
        _ => '·',
    }
}

fn water_overlay_sprite(
    map: &Map,
    layer: ViewLayer,
    tile: Tile,
    overlay: TileOverlay,
    x: usize,
    y: usize,
) -> theme::TileSprite {
    let glyph = theme::tile_glyph(tile, overlay);
    let bg = theme::overlay_tint(OverlayMode::Water, overlay).unwrap_or(glyph.bg);

    if overlay.on_fire || orientation_landmark_tile(tile) {
        let mut sprite = committed_tile_sprite(map, layer, tile, overlay, x, y).with_bg(bg);
        if sprite.left.ch == ' ' && sprite.right.ch == ' ' {
            if let Some(marker) = orientation_network_fallback(tile) {
                sprite = theme::TileSprite::uniform(marker, glyph.fg, bg);
            }
        }
        return sprite;
    }

    theme::TileSprite::uniform(water_overlay_mass_marker(tile), glyph.fg, bg)
}

fn underground_context_tile(tile: Tile) -> bool {
    orientation_landmark_tile(tile)
}

fn underground_active_sprite(
    map: &Map,
    tile: Tile,
    overlay: TileOverlay,
    x: usize,
    y: usize,
) -> theme::TileSprite {
    let glyph = theme::tile_glyph(tile, overlay);
    let mut sprite = committed_tile_sprite(map, ViewLayer::Underground, tile, overlay, x, y);
    if sprite.left.ch == ' ' && sprite.right.ch == ' ' {
        if let Some(marker) = orientation_network_fallback(tile) {
            sprite = theme::TileSprite::uniform(marker, glyph.fg, glyph.bg);
        }
    }
    sprite
}

fn dim_color(color: Color, fallback: Color, factor: f32) -> Color {
    let (r, g, b) =
        color_to_rgb(color).unwrap_or_else(|| color_to_rgb(fallback).unwrap_or((0, 0, 0)));
    Color::Rgb(
        ((r as f32) * factor).round() as u8,
        ((g as f32) * factor).round() as u8,
        ((b as f32) * factor).round() as u8,
    )
}

fn color_to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Reset => None,
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((128, 0, 0)),
        Color::Green => Some((0, 128, 0)),
        Color::Yellow => Some((128, 128, 0)),
        Color::Blue => Some((0, 0, 128)),
        Color::Magenta => Some((128, 0, 128)),
        Color::Cyan => Some((0, 128, 128)),
        Color::Gray => Some((192, 192, 192)),
        Color::DarkGray => Some((128, 128, 128)),
        Color::LightRed => Some((255, 0, 0)),
        Color::LightGreen => Some((0, 255, 0)),
        Color::LightYellow => Some((255, 255, 0)),
        Color::LightBlue => Some((0, 0, 255)),
        Color::LightMagenta => Some((255, 0, 255)),
        Color::LightCyan => Some((0, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(idx) => Some(indexed_color_to_rgb(idx)),
    }
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

fn ghost_surface_sprite(
    map: &Map,
    tile: Tile,
    overlay: TileOverlay,
    x: usize,
    y: usize,
) -> Option<theme::TileSprite> {
    if !underground_context_tile(tile) {
        return None;
    }

    let backdrop = theme::tile_glyph(Tile::Dirt, TileOverlay::default()).bg;
    let base = committed_tile_sprite(map, ViewLayer::Surface, tile, overlay, x, y);
    let fg_fallback = theme::ui_palette().text_dim;
    let recolor_cell = |cell: theme::SpriteCell| theme::SpriteCell {
        ch: cell.ch,
        fg: dim_color(cell.fg, fg_fallback, 0.55),
        bg: dim_color(cell.bg, backdrop, 0.5),
    };
    let mut sprite = theme::TileSprite {
        left: recolor_cell(base.left),
        right: recolor_cell(base.right),
    };

    if sprite.left.ch == ' ' && sprite.right.ch == ' ' {
        if let Some(marker) = orientation_network_fallback(tile) {
            let glyph = theme::tile_glyph(tile, overlay);
            sprite = theme::TileSprite::uniform(
                marker,
                dim_color(glyph.fg, fg_fallback, 0.55),
                dim_color(glyph.bg, backdrop, 0.5),
            );
        }
    }

    Some(sprite)
}

fn merge_sprite_cell(base: theme::SpriteCell, overlay: theme::SpriteCell) -> theme::SpriteCell {
    if overlay.ch == ' ' {
        base
    } else {
        theme::SpriteCell {
            ch: overlay.ch,
            fg: overlay.fg,
            bg: base.bg,
        }
    }
}

fn underground_view_sprite(
    map: &Map,
    overlay: TileOverlay,
    x: usize,
    y: usize,
) -> theme::TileSprite {
    let underground_tile = map.view_tile(ViewLayer::Underground, x, y);
    let surface_tile = map.view_tile(ViewLayer::Surface, x, y);
    let backdrop = ghost_surface_sprite(map, surface_tile, overlay, x, y);

    match (backdrop, underground_tile != Tile::Dirt) {
        (Some(surface), true) => {
            let active = underground_active_sprite(map, underground_tile, overlay, x, y);
            theme::TileSprite {
                left: merge_sprite_cell(surface.left, active.left),
                right: merge_sprite_cell(surface.right, active.right),
            }
        }
        (Some(surface), false) => surface,
        (None, true) => underground_active_sprite(map, underground_tile, overlay, x, y),
        (None, false) => {
            committed_tile_sprite(map, ViewLayer::Underground, underground_tile, overlay, x, y)
        }
    }
}

fn write_sprite_cell(buf: &mut Buffer, x: u16, y: u16, cell_data: theme::SpriteCell) {
    let cell = buf.cell_mut((x, y)).unwrap();
    cell.set_char(cell_data.ch);
    cell.set_fg(cell_data.fg);
    cell.set_bg(cell_data.bg);
}

pub(crate) fn write_tile_sprite(
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    sprite: theme::TileSprite,
) {
    if x < area.x + area.width {
        write_sprite_cell(buf, x, y, sprite.left);
    }
    if x + 1 < area.x + area.width {
        write_sprite_cell(buf, x + 1, y, sprite.right);
    }
}

impl<'a> Widget for MapView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let ui = theme::ui_palette();
        let (cursor_fg, cursor_bg) = theme::cursor_style();
        let animation_phase = traffic_animation_phase();
        let fire_phase = fire_animation_phase();
        let utility_phase = utility_animation_phase();

        let preview_set: std::collections::HashSet<(usize, usize)> =
            self.line_preview.iter().copied().collect();
        let visible_tiles_w = (area.width as usize + 1) / 2;

        for row in 0..area.height {
            let map_y = self.camera.offset_y as usize + row as usize;
            for tile_col in 0..visible_tiles_w {
                let map_x = self.camera.offset_x as usize + tile_col;
                let buf_x = area.x + tile_col as u16 * 2;
                let buf_y = area.y + row;

                if map_x < self.map.width && map_y < self.map.height {
                    let tile = self.map.view_tile(self.view_layer, map_x, map_y);
                    let lot_tile = if self.view_layer == ViewLayer::Surface {
                        self.map.surface_lot_tile(map_x, map_y)
                    } else {
                        tile
                    };
                    let overlay = self.map.get_overlay(map_x, map_y);
                    let is_cursor = map_x == self.camera.cursor_x && map_y == self.camera.cursor_y;
                    let is_preview = !is_cursor && preview_set.contains(&(map_x, map_y));

                    let sprite = if is_cursor {
                        committed_tile_sprite(
                            self.map,
                            self.view_layer,
                            tile,
                            overlay,
                            map_x,
                            map_y,
                        )
                        .recolor(cursor_fg, cursor_bg)
                    } else if is_preview {
                        match &self.preview_kind {
                            PreviewKind::Line(tool) => {
                                let can_place = tool.can_place(tile);
                                let (preview_fg, preview_bg) = if can_place {
                                    (ui.preview_line_fg, ui.preview_line_bg)
                                } else {
                                    (ui.preview_invalid_fg, ui.preview_invalid_bg)
                                };
                                if let Some(target) = tool.target_tile() {
                                    if matches!(
                                        target,
                                        Tile::Road
                                            | Tile::Rail
                                            | Tile::PowerLine
                                            | Tile::RoadPowerLine
                                    ) {
                                        let (n, e, s, w) = preview_connectivity(
                                            self.map,
                                            self.view_layer,
                                            target,
                                            map_x,
                                            map_y,
                                            &preview_set,
                                        );
                                        theme::network_sprite(
                                            target, n, e, s, w, preview_fg, preview_bg,
                                        )
                                    } else {
                                        theme::tile_sprite(target, TileOverlay::default())
                                            .recolor(preview_fg, preview_bg)
                                    }
                                } else {
                                    theme::TileSprite::uniform('╬', preview_fg, preview_bg)
                                }
                            }
                            PreviewKind::Rect(tool) => {
                                let can_place = tool.can_place(tile);
                                let preview_ch = tool
                                    .target_tile()
                                    .map(|t| theme::tile_glyph(t, TileOverlay::default()).ch)
                                    .unwrap_or('?');
                                if can_place {
                                    theme::TileSprite::uniform(
                                        preview_ch,
                                        ui.preview_valid_fg,
                                        ui.preview_valid_bg,
                                    )
                                } else {
                                    theme::TileSprite::uniform(
                                        preview_ch,
                                        ui.preview_invalid_fg,
                                        ui.preview_invalid_bg,
                                    )
                                }
                            }
                            PreviewKind::Footprint(tool, all_valid) => {
                                let preview_ch = tool
                                    .target_tile()
                                    .map(|t| theme::tile_glyph(t, TileOverlay::default()).ch)
                                    .unwrap_or('?');
                                if *all_valid {
                                    theme::TileSprite::uniform(
                                        preview_ch,
                                        ui.preview_valid_fg,
                                        ui.preview_valid_bg,
                                    )
                                } else {
                                    theme::TileSprite::uniform(
                                        preview_ch,
                                        ui.preview_invalid_fg,
                                        ui.preview_invalid_bg,
                                    )
                                }
                            }
                            PreviewKind::None => committed_tile_sprite(
                                self.map,
                                self.view_layer,
                                tile,
                                overlay,
                                map_x,
                                map_y,
                            ),
                        }
                    } else {
                        let sprite = if self.overlay_mode == OverlayMode::Water {
                            let surface_tile = self.map.view_tile(ViewLayer::Surface, map_x, map_y);
                            water_overlay_sprite(
                                self.map,
                                ViewLayer::Surface,
                                surface_tile,
                                overlay,
                                map_x,
                                map_y,
                            )
                        } else {
                            let mut sprite = if self.view_layer == ViewLayer::Underground {
                                underground_view_sprite(self.map, overlay, map_x, map_y)
                            } else {
                                committed_tile_sprite(
                                    self.map,
                                    self.view_layer,
                                    tile,
                                    overlay,
                                    map_x,
                                    map_y,
                                )
                            };
                            let bg = theme::overlay_tint(self.overlay_mode, overlay)
                                .unwrap_or(sprite.left.bg);
                            sprite = sprite.with_bg(bg);
                            sprite
                        };
                        let sprite = animate_transport_sprite(
                            self.map,
                            self.view_layer,
                            tile,
                            overlay,
                            map_x,
                            map_y,
                            sprite,
                            animation_phase,
                        );
                        let sprite = animate_power_overlay_sprite(
                            self.map,
                            self.view_layer,
                            self.overlay_mode,
                            tile,
                            overlay,
                            map_x,
                            map_y,
                            sprite,
                            utility_phase,
                        );
                        let sprite = animate_underground_water_sprite(
                            self.map,
                            self.view_layer,
                            self.overlay_mode,
                            tile,
                            overlay,
                            map_x,
                            map_y,
                            sprite,
                            utility_phase,
                        );
                        let sprite = animate_subway_station_sprite(
                            self.view_layer,
                            tile,
                            sprite,
                            utility_phase,
                        );
                        let sprite = animate_fire_sprite(sprite, overlay, fire_phase);

                        if lot_tile.receives_power() && !overlay.is_powered() {
                            let ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis();
                            if (ms / 500) % 2 == 0 {
                                theme::TileSprite::uniform(
                                    UNPOWERED_WARNING_MARKER,
                                    ui.warning,
                                    sprite.left.bg,
                                )
                            } else {
                                sprite
                            }
                        } else {
                            sprite
                        }
                    };

                    write_tile_sprite(buf, area, buf_x, buf_y, sprite);
                } else {
                    let sprite = theme::TileSprite::uniform(' ', ui.text_dim, ui.desktop_bg);
                    write_tile_sprite(buf, area, buf_x, buf_y, sprite);
                }
            }
        }
    }
}

/// Read-only map view (no cursor) for the New City preview
pub struct MapPreview<'a> {
    pub map: &'a Map,
}

impl<'a> Widget for MapPreview<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let ui = theme::ui_palette();
        let mw = self.map.width as f32;
        let mh = self.map.height as f32;
        // Main map is rendered with 2:1 tiles (double-width).
        let map_visual_aspect = (2.0 * mw) / mh;

        // Fit a rectangle into area that matches map_visual_aspect.
        let (mut rw, mut rh) = if area.width as f32 / area.height as f32 > map_visual_aspect {
            let h = area.height as f32;
            let w = h * map_visual_aspect;
            (w as u16, h as u16)
        } else {
            let w = area.width as f32;
            let h = w / map_visual_aspect;
            (w as u16, h as u16)
        };

        // Ensure width is even for double-width tiles
        rw = (rw / 2) * 2;
        rw = rw.max(2);
        rh = rh.max(1);

        // Center the render rectangle
        let rx = area.x + (area.width.saturating_sub(rw)) / 2;
        let ry = area.y + (area.height.saturating_sub(rh)) / 2;
        let render_area = Rect::new(rx, ry, rw, rh);

        // Clear entire widget area with void color first
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_char(' ');
                cell.set_bg(ui.map_window_bg);
            }
        }

        let mw_usize = self.map.width;
        let mh_usize = self.map.height;
        let num_v_tiles_x = (render_area.width / 2) as usize;
        let num_v_tiles_y = render_area.height as usize;

        for v_row in 0..num_v_tiles_y {
            for v_col in 0..num_v_tiles_x {
                // Endpoint-interpolation
                let map_x = if num_v_tiles_x <= 1 {
                    0
                } else {
                    (v_col * (mw_usize - 1)) / (num_v_tiles_x - 1)
                };
                let map_y = if num_v_tiles_y <= 1 {
                    0
                } else {
                    (v_row * (mh_usize - 1)) / (num_v_tiles_y - 1)
                };

                let tile = self.map.get(map_x, map_y);
                let overlay = self.map.get_overlay(map_x, map_y);
                let sprite = committed_tile_sprite(
                    self.map,
                    ViewLayer::Surface,
                    tile,
                    overlay,
                    map_x,
                    map_y,
                );
                let bx = render_area.x + (v_col as u16 * 2);
                let by = render_area.y + v_row as u16;
                write_tile_sprite(buf, area, bx, by, sprite);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn layout_reserves_both_scrollbars_when_needed() {
        let layout = layout_map_chrome(Rect::new(0, 0, 20, 10), 40, 20, 0, 0);
        assert_eq!(layout.viewport, Rect::new(0, 0, 19, 9));
        assert_eq!(layout.vertical_bar, Rect::new(19, 0, 1, 9));
        assert_eq!(layout.horizontal_bar, Rect::new(0, 9, 19, 1));
        assert_eq!(layout.corner, Rect::new(19, 9, 1, 1));
    }

    #[test]
    fn scrollbar_offset_from_pointer_clamps_to_max() {
        let offset = scrollbar_offset_from_pointer(10, 3, 20, 99, 1);
        assert_eq!(offset, 20);
    }

    #[test]
    fn vertical_road_uses_single_stem_sprite() {
        let mut map = Map::new(3, 3);
        map.set(1, 0, Tile::Road);
        map.set(1, 1, Tile::Road);
        map.set(1, 2, Tile::Road);

        let camera = Camera {
            offset_x: 1,
            offset_y: 1,
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        MapView {
            map: &map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::None,
            view_layer: ViewLayer::Surface,
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "║");
    }

    #[test]
    fn traffic_animation_moves_marker_across_horizontal_roads() {
        let mut map = Map::new(3, 1);
        map.set(0, 0, Tile::Road);
        map.set(1, 0, Tile::Road);
        map.set(2, 0, Tile::Road);
        map.set_overlay(
            1,
            0,
            TileOverlay {
                traffic: 180,
                ..TileOverlay::default()
            },
        );

        let overlay = map.get_overlay(1, 0);
        let base = committed_tile_sprite(&map, ViewLayer::Surface, Tile::Road, overlay, 1, 0);
        let first =
            animate_transport_sprite(&map, ViewLayer::Surface, Tile::Road, overlay, 1, 0, base, 0);
        let second =
            animate_transport_sprite(&map, ViewLayer::Surface, Tile::Road, overlay, 1, 0, base, 1);

        assert_eq!(first.left.ch, '•');
        assert_eq!(first.right.ch, '═');
        assert_eq!(second.left.ch, '═');
        assert_eq!(second.right.ch, '•');
    }

    #[test]
    fn traffic_animation_only_runs_on_surface_drive_tiles() {
        let mut map = Map::new(1, 3);
        map.set(0, 0, Tile::Road);
        map.set(0, 1, Tile::Road);
        map.set(0, 2, Tile::Road);
        map.set_overlay(
            0,
            1,
            TileOverlay {
                traffic: 200,
                ..TileOverlay::default()
            },
        );

        let overlay = map.get_overlay(0, 1);
        let base = committed_tile_sprite(&map, ViewLayer::Surface, Tile::Road, overlay, 0, 1);
        let surface =
            animate_transport_sprite(&map, ViewLayer::Surface, Tile::Road, overlay, 0, 1, base, 1);
        let underground = animate_transport_sprite(
            &map,
            ViewLayer::Underground,
            Tile::Road,
            overlay,
            0,
            1,
            base,
            1,
        );

        assert_eq!(surface.right.ch, '•');
        assert_eq!(underground, base);
    }

    #[test]
    fn fire_animation_flickers_between_distinct_ascii_frames() {
        let overlay = TileOverlay {
            on_fire: true,
            ..TileOverlay::default()
        };
        let base = theme::TileSprite::uniform('*', Color::Rgb(255, 200, 0), Color::Rgb(150, 40, 0));

        let first = animate_fire_sprite(base, overlay, 0);
        let second = animate_fire_sprite(base, overlay, 1);

        assert_eq!(first.left.ch, '*');
        assert_eq!(first.right.ch, '+');
        assert_eq!(second.left.ch, '+');
        assert_eq!(second.right.ch, '*');
        assert_ne!(first.left.bg, second.left.bg);
    }

    #[test]
    fn fire_animation_leaves_non_burning_tiles_unchanged() {
        let overlay = TileOverlay::default();
        let base = theme::TileSprite::uniform('R', Color::White, Color::Black);

        assert_eq!(animate_fire_sprite(base, overlay, 0), base);
    }

    #[test]
    fn power_overlay_animation_marks_live_power_lines() {
        let mut map = Map::new(3, 1);
        map.set(0, 0, Tile::PowerLine);
        map.set(1, 0, Tile::PowerLine);
        map.set(2, 0, Tile::PowerLine);
        map.set_overlay(
            1,
            0,
            TileOverlay {
                power_level: 255,
                ..TileOverlay::default()
            },
        );

        let overlay = map.get_overlay(1, 0);
        let base = committed_tile_sprite(&map, ViewLayer::Surface, Tile::PowerLine, overlay, 1, 0);
        let animated = animate_power_overlay_sprite(
            &map,
            ViewLayer::Surface,
            OverlayMode::Power,
            Tile::PowerLine,
            overlay,
            1,
            0,
            base,
            0,
        );

        assert!(animated.left.ch == '•' || animated.right.ch == '•');
    }

    #[test]
    fn water_animation_marks_underground_pipes_only() {
        let mut map = Map::new(3, 1);
        map.set_water_pipe(0, 0, true);
        map.set_water_pipe(1, 0, true);
        map.set_water_pipe(2, 0, true);
        map.set_overlay(
            1,
            0,
            TileOverlay {
                water_service: 255,
                ..TileOverlay::default()
            },
        );

        let overlay = map.get_overlay(1, 0);
        let base =
            committed_tile_sprite(&map, ViewLayer::Underground, Tile::WaterPipe, overlay, 1, 0);
        let underground = animate_underground_water_sprite(
            &map,
            ViewLayer::Underground,
            OverlayMode::None,
            Tile::WaterPipe,
            overlay,
            1,
            0,
            base,
            0,
        );
        let surface = animate_underground_water_sprite(
            &map,
            ViewLayer::Surface,
            OverlayMode::None,
            Tile::WaterPipe,
            overlay,
            1,
            0,
            base,
            0,
        );

        assert!(underground.left.ch == '◦' || underground.right.ch == '◦');
        assert_eq!(surface, base);
    }

    #[test]
    fn subway_station_animation_pulses_underground_only() {
        let base = theme::TileSprite::uniform('U', Color::White, Color::Blue);

        let underground =
            animate_subway_station_sprite(ViewLayer::Underground, Tile::SubwayStation, base, 0);
        let surface =
            animate_subway_station_sprite(ViewLayer::Surface, Tile::SubwayStation, base, 0);

        assert_eq!(underground.left.ch, 'U');
        assert_eq!(underground.right.ch, '•');
        assert_eq!(surface, base);
    }

    #[test]
    fn line_preview_uses_network_sprite_pair() {
        let map = Map::new(3, 1);
        let camera = Camera {
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        MapView {
            map: &map,
            camera: &camera,
            line_preview: &[(0, 0), (1, 0)],
            preview_kind: PreviewKind::Line(Tool::Road),
            overlay_mode: OverlayMode::None,
            view_layer: ViewLayer::Surface,
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "═");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "═");
    }

    #[test]
    fn water_overlay_keeps_isolated_roads_visible() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::Road);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                water_service: 200,
                ..TileOverlay::default()
            },
        );
        let camera = Camera {
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        MapView {
            map: &map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::Water,
            view_layer: ViewLayer::Surface,
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "·");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "·");
    }

    #[test]
    fn water_overlay_keeps_landmark_buildings_recognizable() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::Police);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                water_service: 200,
                power_level: 255,
                ..TileOverlay::default()
            },
        );
        let camera = Camera {
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        MapView {
            map: &map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::Water,
            view_layer: ViewLayer::Surface,
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "P");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "]");
    }

    #[test]
    fn water_overlay_simplifies_generic_buildings() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::ResLow);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                water_service: 200,
                power_level: 255,
                ..TileOverlay::default()
            },
        );
        let camera = Camera {
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        MapView {
            map: &map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::Water,
            view_layer: ViewLayer::Surface,
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "•");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "•");
    }

    #[test]
    fn underground_view_keeps_isolated_roads_visible() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::Road);
        let camera = Camera {
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        MapView {
            map: &map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::None,
            view_layer: ViewLayer::Underground,
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "·");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "·");
    }

    #[test]
    fn water_overlay_stays_surface_oriented_in_underground_layer() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::Road);
        map.set_water_pipe(0, 0, true);
        map.set_overlay(
            0,
            0,
            TileOverlay {
                water_service: 200,
                ..TileOverlay::default()
            },
        );
        let camera = Camera {
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        MapView {
            map: &map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::Water,
            view_layer: ViewLayer::Underground,
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "·");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "·");
    }

    #[test]
    fn underground_view_keeps_landmark_buildings_visible() {
        let mut map = Map::new(1, 1);
        map.set(0, 0, Tile::Police);
        let camera = Camera {
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);
        let mut buf = Buffer::empty(area);

        MapView {
            map: &map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::None,
            view_layer: ViewLayer::Underground,
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "P");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "]");
    }

    #[test]
    fn underground_view_uses_surface_context_under_pipes() {
        let camera = Camera {
            cursor_x: usize::MAX,
            cursor_y: usize::MAX,
            ..Camera::default()
        };
        let area = Rect::new(0, 0, 2, 1);

        let mut road_map = Map::new(1, 1);
        road_map.set(0, 0, Tile::Road);
        road_map.set_water_pipe(0, 0, true);
        let mut road_buf = Buffer::empty(area);
        MapView {
            map: &road_map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::None,
            view_layer: ViewLayer::Underground,
        }
        .render(area, &mut road_buf);

        let mut plain_map = Map::new(1, 1);
        plain_map.set_water_pipe(0, 0, true);
        let mut plain_buf = Buffer::empty(area);
        MapView {
            map: &plain_map,
            camera: &camera,
            line_preview: &[],
            preview_kind: PreviewKind::None,
            overlay_mode: OverlayMode::None,
            view_layer: ViewLayer::Underground,
        }
        .render(area, &mut plain_buf);

        assert_eq!(road_buf.cell((0, 0)).unwrap().symbol(), "┈");
        assert_eq!(plain_buf.cell((0, 0)).unwrap().symbol(), "┈");
        assert_ne!(
            road_buf.cell((0, 0)).unwrap().bg,
            plain_buf.cell((0, 0)).unwrap().bg
        );
    }

    #[test]
    fn unpowered_warning_marker_is_single_width_safe() {
        assert_eq!(UNPOWERED_WARNING_MARKER, '!');
    }
}
