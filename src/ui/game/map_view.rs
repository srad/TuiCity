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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScrollbarMetrics {
    pub thumb_start: u16,
    pub thumb_len: u16,
    pub max_offset: usize,
}

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

pub fn scrollbar_metrics(
    track_len: u16,
    view_items: usize,
    total_items: usize,
    offset: usize,
) -> Option<ScrollbarMetrics> {
    if track_len == 0 || view_items == 0 || total_items <= view_items {
        return None;
    }

    let max_offset = total_items.saturating_sub(view_items);
    let thumb_len = ((track_len as usize * view_items) / total_items).max(1) as u16;
    let thumb_start = if max_offset == 0 || thumb_len >= track_len {
        0
    } else {
        ((track_len - thumb_len) as usize * offset.min(max_offset) / max_offset) as u16
    };

    Some(ScrollbarMetrics {
        thumb_start,
        thumb_len: thumb_len.min(track_len),
        max_offset,
    })
}

pub fn scrollbar_offset_from_pointer(
    track_len: u16,
    thumb_len: u16,
    max_offset: usize,
    pointer: u16,
    grab_offset: u16,
) -> usize {
    if track_len == 0 || thumb_len >= track_len || max_offset == 0 {
        return 0;
    }

    let max_thumb_pos = track_len - thumb_len;
    let thumb_pos = pointer.saturating_sub(grab_offset).min(max_thumb_pos);
    (thumb_pos as usize * max_offset) / max_thumb_pos as usize
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
    let e = if x + 1 < map.width {
        matches_tile(map.get(x + 1, y))
    } else {
        false
    };
    let s = if y + 1 < map.height {
        matches_tile(map.get(x, y + 1))
    } else {
        false
    };
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
    tile: Tile,
    overlay: TileOverlay,
    x: usize,
    y: usize,
) -> theme::TileSprite {
    let glyph = theme::tile_glyph(tile, overlay);
    if matches!(
        tile,
        Tile::Road | Tile::Rail | Tile::PowerLine | Tile::RoadPowerLine
    ) {
        let (n, e, s, w) = map_connectivity(map, tile, x, y);
        theme::network_sprite(tile, n, e, s, w, glyph.fg, glyph.bg)
    } else {
        theme::tile_sprite(tile, overlay)
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
                    let tile = self.map.get(map_x, map_y);
                    let overlay = self.map.get_overlay(map_x, map_y);
                    let is_cursor = map_x == self.camera.cursor_x && map_y == self.camera.cursor_y;
                    let is_preview = !is_cursor && preview_set.contains(&(map_x, map_y));

                    let sprite = if is_cursor {
                        committed_tile_sprite(self.map, tile, overlay, map_x, map_y)
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
                            PreviewKind::None => {
                                committed_tile_sprite(self.map, tile, overlay, map_x, map_y)
                            }
                        }
                    } else {
                        let mut sprite =
                            committed_tile_sprite(self.map, tile, overlay, map_x, map_y);
                        let bg = theme::overlay_tint(self.overlay_mode, overlay)
                            .unwrap_or(sprite.left.bg);
                        sprite = sprite.with_bg(bg);

                        if tile.receives_power() && !overlay.is_powered() {
                            let ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis();
                            if (ms / 500) % 2 == 0 {
                                sprite = theme::TileSprite::uniform('⚡', ui.warning, bg);
                            }
                        }
                        sprite
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
                let sprite = committed_tile_sprite(self.map, tile, overlay, map_x, map_y);
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
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "║");
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
        }
        .render(area, &mut buf);

        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "═");
        assert_eq!(buf.cell((1, 0)).unwrap().symbol(), "═");
    }
}
