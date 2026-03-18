use crate::{
    app::camera::Camera,
    core::map::Map,
    ui::theme::{self, OverlayMode},
};
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

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

        // Title row
        let title = "── MINI-MAP ──";
        buf.set_string(
            area.x,
            area.y,
            title,
            ratatui::style::Style::default()
                .fg(Color::Rgb(140, 140, 180))
                .bg(Color::Rgb(10, 10, 20)),
        );

        let win_area = Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(1),
            ..area
        };

        if win_area.height == 0 || win_area.width == 0 {
            return;
        }

        let mw = self.map.width as f32;
        let mh = self.map.height as f32;
        // Main map is rendered with 2:1 tiles (double-width).
        // So visually the map is (2 * mw) wide and (mh) high.
        let map_visual_aspect = (2.0 * mw) / mh;

        // Minimap area is (win_area.width) x (win_area.height) cells.
        // Each cell is roughly 1:1 or 2:1 depending on terminal font, but in terms of
        // buffer coordinates, 1 cell x 1 cell is "square-ish" relative to the map logic.
        // BUT wait, a map tile on the main screen is 2x1 cells.
        // To match that proportion on the minimap, we should also target 2:1.
        
        // Let's fit a rectangle into win_area that matches map_visual_aspect.
        let (rw, rh) = if win_area.width as f32 / win_area.height as f32 > map_visual_aspect {
            // Area is too wide: height is the bottleneck.
            let h = win_area.height as f32;
            let w = h * map_visual_aspect;
            (w as u16, h as u16)
        } else {
            // Area is too tall: width is the bottleneck.
            let w = win_area.width as f32;
            let h = w / map_visual_aspect;
            (w as u16, h as u16)
        };

        // Center the render rectangle.
        let rx = win_area.x + (win_area.width.saturating_sub(rw)) / 2;
        let ry = win_area.y + (win_area.height.saturating_sub(rh)) / 2;
        let render_area = Rect::new(rx, ry, rw.max(1), rh.max(1));

        let mw = self.map.width;
        let mh = self.map.height;
        let rw_usize = render_area.width as usize;
        let rh_usize = render_area.height as usize;

        for row in 0..render_area.height {
            for col in 0..render_area.width {
                // Endpoint-interpolation: col 0 → tile 0, col (rw-1) → tile (mw-1)
                let map_x = if rw_usize <= 1 { 0 } else { (col as usize * (mw - 1)) / (rw_usize - 1) };
                let map_y = if rh_usize <= 1 { 0 } else { (row as usize * (mh - 1)) / (rh_usize - 1) };

                let tile = self.map.get(map_x, map_y);
                let overlay = self.map.get_overlay(map_x, map_y);
                let glyph = theme::tile_glyph(tile, overlay);
                
                // Use overlay tint if active.
                let bg = theme::overlay_tint(self.overlay_mode, overlay)
                    .unwrap_or(glyph.bg);

                let cell = buf
                    .cell_mut((render_area.x + col, render_area.y + row))
                    .unwrap();
                cell.set_char(' ');
                cell.set_bg(bg);
            }
        }

        // Draw viewport rectangle — use same endpoint-interpolation formula as tile sampling
        // pixel = tile * (rw-1) / (mw-1)
        let vx0 = if mw <= 1 { 0u16 } else {
            ((self.camera.offset_x as usize * (rw_usize - 1)) / (mw - 1)) as u16
        };
        let vy0 = if mh <= 1 { 0u16 } else {
            ((self.camera.offset_y as usize * (rh_usize - 1)) / (mh - 1)) as u16
        };
        let vx1 = if mw <= 1 { (rw_usize - 1) as u16 } else {
            (((self.camera.offset_x + self.camera.view_w as i32) as usize).min(mw - 1) * (rw_usize - 1)
                / (mw - 1)) as u16
        };
        let vy1 = if mh <= 1 { (rh_usize - 1) as u16 } else {
            (((self.camera.offset_y + self.camera.view_h as i32) as usize).min(mh - 1) * (rh_usize - 1)
                / (mh - 1)) as u16
        };

        let vx0 = vx0.min(render_area.width.saturating_sub(1));
        let vy0 = vy0.min(render_area.height.saturating_sub(1));
        let vx1 = vx1.min(render_area.width.saturating_sub(1));
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

fn set_viewport_cell(buf: &mut Buffer, x: u16, y: u16) {
    if x < buf.area.x + buf.area.width && y < buf.area.y + buf.area.height {
        let cell = buf.cell_mut((x, y)).unwrap();
        cell.set_fg(Color::Rgb(255, 255, 100));
        // Also darken the background slightly so the viewport rectangle is more visible
        // Actually, let's keep it simple and just set the character if it's a corner?
        // Let's just use the foreground color.
    }
}
