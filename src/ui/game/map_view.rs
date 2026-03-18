use crate::{
    app::camera::Camera,
    core::{map::Map, map::Tile, map::TileOverlay, tool::Tool},
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
}

/// N/E/S/W connectivity flags for a committed map tile.
fn map_connectivity(map: &Map, tile: Tile, x: usize, y: usize) -> (bool, bool, bool, bool) {
    let matches_tile = |t: Tile| match tile {
        Tile::Road | Tile::RoadPowerLine => t.road_connects(),
        Tile::PowerLine => t.power_connects(),
        _ => t == tile,
    };
    let n = y
        .checked_sub(1)
        .map(|ny| matches_tile(map.get(x, ny)))
        .unwrap_or(false);
    let e = if x + 1 < map.width { matches_tile(map.get(x + 1, y)) } else { false };
    let s = if y + 1 < map.height { matches_tile(map.get(x, y + 1)) } else { false };
    let w = x
        .checked_sub(1)
        .map(|wx| matches_tile(map.get(wx, y)))
        .unwrap_or(false);
    (n, e, s, w)
}

/// N/E/S/W connectivity for a preview tile — connects to both the preview set
/// AND already-committed matching tiles, so the preview shows accurate junctions.
fn preview_connectivity(
    map: &Map,
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
        let t = map.get(nx, ny);
        match target {
            Tile::Road => t.road_connects(),
            Tile::PowerLine => t.power_connects(),
            _ => t == target,
        }
    };
    let n = y.checked_sub(1).map(|ny| connects(x, ny)).unwrap_or(false);
    let e = if x + 1 < map.width { connects(x + 1, y) } else { false };
    let s = if y + 1 < map.height { connects(x, y + 1) } else { false };
    let w = x.checked_sub(1).map(|wx| connects(wx, y)).unwrap_or(false);
    (n, e, s, w)
}

impl<'a> Widget for MapView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let (cursor_fg, cursor_bg) = theme::cursor_style();

        let preview_set: std::collections::HashSet<(usize, usize)> =
            self.line_preview.iter().copied().collect();

        for row in 0..area.height {
            for col in 0..area.width {
                let map_x = self.camera.offset_x as usize + (col as usize / 2);
                let map_y = self.camera.offset_y as usize + row as usize;

                let buf_x = area.x + col;
                let buf_y = area.y + row;

                if map_x < self.map.width && map_y < self.map.height {
                    let tile = self.map.get(map_x, map_y);
                    let overlay = self.map.get_overlay(map_x, map_y);
                    let glyph = theme::tile_glyph(tile, overlay);

                    let is_cursor = map_x == self.camera.cursor_x && map_y == self.camera.cursor_y;
                    let is_preview = !is_cursor && preview_set.contains(&(map_x, map_y));

                    let ch = if matches!(
                        tile,
                        Tile::Road | Tile::Rail | Tile::PowerLine | Tile::RoadPowerLine
                    ) {
                        let (n, e, s, w) = map_connectivity(self.map, tile, map_x, map_y);
                        theme::network_char(tile, n, e, s, w)
                    } else if matches!(
                        tile,
                        Tile::PowerPlant | Tile::Park | Tile::Police | Tile::Fire
                    ) {
                        let same = |tx: usize, ty: usize| self.map.get(tx, ty) == tile;
                        let n = map_y.checked_sub(1).map(|ny| same(map_x, ny)).unwrap_or(false);
                        let s = if map_y + 1 < self.map.height { same(map_x, map_y + 1) } else { false };
                        let e = if map_x + 1 < self.map.width  { same(map_x + 1, map_y) } else { false };
                        let w = map_x.checked_sub(1).map(|wx| same(wx, map_y)).unwrap_or(false);
                        theme::building_char(tile, n, e, s, w)
                    } else {
                        glyph.ch
                    };

                    let (ch, fg, bg) = if is_cursor {
                        (ch, cursor_fg, cursor_bg)
                    } else if is_preview {
                        match &self.preview_kind {
                            PreviewKind::Line(tool) => {
                                let can_place = tool.can_place(tile);
                                let preview_ch = if let Some(target) = tool.target_tile() {
                                    let (n, e, s, w) = preview_connectivity(
                                        self.map,
                                        target,
                                        map_x,
                                        map_y,
                                        &preview_set,
                                    );
                                    theme::network_char(target, n, e, s, w)
                                } else {
                                    '╬'
                                };
                                if can_place {
                                    (
                                        preview_ch,
                                        Color::Rgb(100, 200, 255),
                                        Color::Rgb(20, 50, 80),
                                    )
                                } else {
                                    (preview_ch, Color::Rgb(255, 80, 80), Color::Rgb(80, 10, 10))
                                }
                            }
                            PreviewKind::Rect(tool) => {
                                let can_place = tool.can_place(tile);
                                let preview_ch = tool
                                    .target_tile()
                                    .map(|t| theme::tile_glyph(t, TileOverlay::default()).ch)
                                    .unwrap_or('?');
                                if can_place {
                                    (preview_ch, Color::Rgb(80, 255, 120), Color::Rgb(10, 60, 20))
                                } else {
                                    (preview_ch, Color::Rgb(255, 80, 80), Color::Rgb(80, 10, 10))
                                }
                            }
                            PreviewKind::Footprint(tool, all_valid) => {
                                let preview_ch = tool
                                    .target_tile()
                                    .map(|t| theme::tile_glyph(t, TileOverlay::default()).ch)
                                    .unwrap_or('?');
                                if *all_valid {
                                    (preview_ch, Color::Rgb(80, 255, 120), Color::Rgb(10, 60, 20))
                                } else {
                                    (preview_ch, Color::Rgb(255, 80, 80), Color::Rgb(80, 10, 10))
                                }
                            }
                            PreviewKind::None => (ch, glyph.fg, glyph.bg),
                        }
                    } else {
                        // Apply heat-map tint (replaces bg, keeps ch and fg)
                        let bg = theme::overlay_tint(self.overlay_mode, overlay)
                            .unwrap_or(glyph.bg);
                        (ch, glyph.fg, bg)
                    };

                    let cell = buf.cell_mut((buf_x, buf_y)).unwrap();
                    cell.set_char(ch);
                    cell.set_fg(fg);
                    cell.set_bg(bg);
                } else {
                    // Out-of-bounds: render dark void
                    let cell = buf.cell_mut((buf_x, buf_y)).unwrap();
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(10, 10, 10));
                }
            }
        }

        // ── Scrollbar overlay ─────────────────────────────────────────────────
        // Drawn after tiles so they always appear on top of map content.
        let track_bg  = Color::Rgb(18, 18, 28);
        let track_fg  = Color::Rgb(55, 55, 75);
        let thumb_fg  = Color::Rgb(150, 150, 190);

        // Vertical scrollbar — right-most column
        if self.map.height > area.height as usize && area.width >= 1 {
            let track_len = area.height as usize;
            let map_h     = self.map.height;
            let view_h    = area.height as usize;
            let max_off   = map_h.saturating_sub(view_h);
            let thumb_len = ((track_len * view_h) / map_h).max(1);
            let thumb_pos = if max_off == 0 { 0 } else {
                (track_len.saturating_sub(thumb_len))
                    * (self.camera.offset_y as usize).min(max_off)
                    / max_off
            };
            let sx = area.x + area.width - 1;
            for row in 0..track_len {
                let is_thumb = row >= thumb_pos && row < thumb_pos + thumb_len;
                let cell = buf.cell_mut((sx, area.y + row as u16)).unwrap();
                cell.set_char(if is_thumb { '█' } else { '░' });
                cell.set_fg(if is_thumb { thumb_fg } else { track_fg });
                cell.set_bg(track_bg);
            }
        }

        // --- Horizontal Scrollbar ---
        if self.map.width > (area.width as usize / 2) && area.height >= 1 {
            let track_len = area.width as usize;
            let map_w     = self.map.width;
            let view_w    = area.width as usize / 2;
            let max_off   = map_w.saturating_sub(view_w);
            let thumb_len = ((track_len * view_w) / map_w).max(1);
            let thumb_pos = if max_off == 0 { 0 } else {
                (track_len.saturating_sub(thumb_len))
                    * (self.camera.offset_x as usize).min(max_off)
                    / max_off
            };
            let sy = area.y + area.height - 1;
            for col in 0..track_len {
                let is_thumb = col >= thumb_pos && col < thumb_pos + thumb_len;
                let cell = buf.cell_mut((area.x + col as u16, sy)).unwrap();
                cell.set_char(if is_thumb { '█' } else { '░' });
                cell.set_fg(if is_thumb { thumb_fg } else { track_fg });
                cell.set_bg(track_bg);
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

        let mw = self.map.width;
        let mh = self.map.height;
        let pw = area.width as usize;
        let ph = area.height as usize;

        // Center crop: find top-left coordinate, ensuring we don't underflow
        let offset_x = (mw / 2).saturating_sub((pw / 2) / 2);
        let offset_y = (mh / 2).saturating_sub(ph / 2);

        for row in 0..area.height {
            for col in 0..area.width {
                let map_x = offset_x + (col as usize / 2);
                let map_y = offset_y + row as usize;

                if map_x < mw && map_y < mh {
                    let tile = self.map.get(map_x, map_y);
                    let overlay = self.map.get_overlay(map_x, map_y);
                    let glyph = theme::tile_glyph(tile, overlay);

                    let cell = buf.cell_mut((area.x + col, area.y + row)).unwrap();
                    cell.set_char(glyph.ch);
                    cell.set_fg(glyph.fg);
                    cell.set_bg(glyph.bg);
                } else {
                    let cell = buf.cell_mut((area.x + col, area.y + row)).unwrap();
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(8, 12, 8)); // Match the void background
                }
            }
        }
    }
}
