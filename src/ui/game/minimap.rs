use crate::{
    app::camera::Camera,
    core::map::Map,
    ui::game::map_view,
    ui::theme::{self, OverlayMode},
};
use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

pub struct MiniMap<'a> {
    pub map: &'a Map,
    pub camera: &'a Camera,
    pub overlay_mode: OverlayMode,
}

impl<'a> Widget for MiniMap<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let ui = theme::ui_palette();

        // Title row
        let title = "── MINI-MAP ──";
        buf.set_string(
            area.x,
            area.y,
            title,
            Style::default().fg(ui.text_muted).bg(ui.panel_window_bg),
        );

        let win_area = Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(1),
            ..area
        };

        if win_area.height == 0 || win_area.width == 0 {
            return;
        }
        let render_area = minimap_render_area(area, self.map.width, self.map.height);
        if render_area.width == 0 || render_area.height == 0 {
            return;
        }

        let mw = self.map.width;
        let mh = self.map.height;
        let rw_usize = (render_area.width / 2) as usize;
        let rh_usize = render_area.height as usize;

        for row in 0..render_area.height {
            for col in 0..rw_usize {
                // Endpoint-interpolation: visual tile 0 → tile 0, last visual tile → last map tile.
                let map_x = if rw_usize <= 1 {
                    0
                } else {
                    (col * (mw - 1)) / (rw_usize - 1)
                };
                let map_y = if rh_usize <= 1 {
                    0
                } else {
                    (row as usize * (mh - 1)) / (rh_usize - 1)
                };

                let tile = self.map.get(map_x, map_y);
                let overlay = self.map.get_overlay(map_x, map_y);
                let mut sprite =
                    map_view::committed_tile_sprite(self.map, tile, overlay, map_x, map_y);
                if let Some(bg) = theme::overlay_tint(self.overlay_mode, overlay) {
                    sprite = sprite.with_bg(bg);
                }
                let bx = render_area.x + col as u16 * 2;
                let by = render_area.y + row;
                map_view::write_tile_sprite(buf, render_area, bx, by, sprite);
            }
        }

        // Draw viewport rectangle — use same endpoint-interpolation formula as tile sampling
        // visual_tile = tile * (rw-1) / (mw-1)
        let vx0_tile = if mw <= 1 || rw_usize <= 1 {
            0u16
        } else {
            ((self.camera.offset_x as usize * (rw_usize - 1)) / (mw - 1)) as u16
        };
        let vy0 = if mh <= 1 || rh_usize <= 1 {
            0u16
        } else {
            ((self.camera.offset_y as usize * (rh_usize - 1)) / (mh - 1)) as u16
        };
        let vx1_tile = if mw <= 1 || rw_usize <= 1 {
            rw_usize.saturating_sub(1) as u16
        } else {
            (((self.camera.offset_x + self.camera.view_w as i32) as usize).min(mw - 1)
                * (rw_usize - 1)
                / (mw - 1)) as u16
        };
        let vy1 = if mh <= 1 || rh_usize <= 1 {
            rh_usize.saturating_sub(1) as u16
        } else {
            (((self.camera.offset_y + self.camera.view_h as i32) as usize).min(mh - 1)
                * (rh_usize - 1)
                / (mh - 1)) as u16
        };

        let vx0 = (vx0_tile.saturating_mul(2)).min(render_area.width.saturating_sub(1));
        let vy0 = vy0.min(render_area.height.saturating_sub(1));
        let vx1 = (vx1_tile.saturating_mul(2) + 1).min(render_area.width.saturating_sub(1));
        let vy1 = vy1.min(render_area.height.saturating_sub(1));

        // Draw viewport border on minimap
        for x in vx0..=vx1 {
            set_viewport_cell(buf, render_area.x + x, render_area.y + vy0);
            set_viewport_cell(buf, render_area.x + x, render_area.y + vy1);
        }
        for y in vy0..=vy1 {
            set_viewport_cell(buf, render_area.x + vx0, render_area.y + y);
            set_viewport_cell(buf, render_area.x + vx1, render_area.y + y);
        }
    }
}

pub fn minimap_render_area(area: Rect, map_width: usize, map_height: usize) -> Rect {
    if area.width == 0 || area.height <= 1 || map_width == 0 || map_height == 0 {
        return Rect::default();
    }

    let body = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };
    if body.width == 0 || body.height == 0 {
        return Rect::default();
    }
    if body.width < 2 {
        return Rect::default();
    }

    let map_visual_aspect = (2.0 * map_width as f32) / map_height as f32;
    let (rw, rh) = if body.width as f32 / body.height as f32 > map_visual_aspect {
        let h = body.height as f32;
        let w = h * map_visual_aspect;
        (w as u16, h as u16)
    } else {
        let w = body.width as f32;
        let h = w / map_visual_aspect;
        (w as u16, h as u16)
    };

    let width = (rw.max(2).min(body.width) / 2) * 2;
    let height = rh.max(1);
    Rect::new(
        body.x + (body.width.saturating_sub(width)) / 2,
        body.y + (body.height.saturating_sub(height)) / 2,
        width,
        height,
    )
}

pub fn tile_at_minimap_click(
    area: Rect,
    map_width: usize,
    map_height: usize,
    col: u16,
    row: u16,
) -> Option<(usize, usize)> {
    let render_area = minimap_render_area(area, map_width, map_height);
    tile_at_render_area_click(render_area, map_width, map_height, col, row)
}

pub fn tile_at_render_area_click(
    render_area: Rect,
    map_width: usize,
    map_height: usize,
    col: u16,
    row: u16,
) -> Option<(usize, usize)> {
    if render_area.width == 0
        || render_area.height == 0
        || col < render_area.x
        || col >= render_area.x + render_area.width
        || row < render_area.y
        || row >= render_area.y + render_area.height
    {
        return None;
    }

    let rel_x = ((col - render_area.x) / 2) as usize;
    let rel_y = (row - render_area.y) as usize;
    let grid_w = (render_area.width / 2) as usize;
    let grid_h = render_area.height as usize;
    let tile_x = if grid_w <= 1 {
        0
    } else {
        rel_x * map_width.saturating_sub(1) / (grid_w - 1)
    };
    let tile_y = if grid_h <= 1 {
        0
    } else {
        rel_y * map_height.saturating_sub(1) / (grid_h - 1)
    };

    Some((tile_x, tile_y))
}

fn set_viewport_cell(buf: &mut Buffer, x: u16, y: u16) {
    if x < buf.area.x + buf.area.width && y < buf.area.y + buf.area.height {
        let cell = buf.cell_mut((x, y)).unwrap();
        cell.set_fg(theme::ui_palette().viewport_outline);
        // Also darken the background slightly so the viewport rectangle is more visible
        // Actually, let's keep it simple and just set the character if it's a corner?
        // Let's just use the foreground color.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_area_stays_inside_body() {
        let render = minimap_render_area(Rect::new(10, 5, 20, 9), 128, 128);
        assert!(render.x >= 10);
        assert!(render.y >= 6);
        assert!(render.x + render.width <= 30);
        assert!(render.y + render.height <= 14);
        assert_eq!(render.width % 2, 0);
    }

    #[test]
    fn click_maps_into_minimap_tiles() {
        let area = Rect::new(0, 0, 20, 9);
        let render = minimap_render_area(area, 128, 64);
        let (tile_x, tile_y) = tile_at_minimap_click(
            area,
            128,
            64,
            render.x + render.width / 2,
            render.y + render.height / 2,
        )
        .expect("center click should hit the rendered minimap");
        assert!(tile_x > 0 && tile_x < 127);
        assert!(tile_y > 0 && tile_y < 63);
    }

    #[test]
    fn both_columns_of_a_sprite_map_to_the_same_tile() {
        let area = Rect::new(0, 0, 20, 9);
        let render = minimap_render_area(area, 8, 8);
        let left =
            tile_at_minimap_click(area, 8, 8, render.x, render.y).expect("left column should hit");
        let right = tile_at_minimap_click(area, 8, 8, render.x + 1, render.y)
            .expect("right column should hit");
        assert_eq!(left, right);
    }
}
