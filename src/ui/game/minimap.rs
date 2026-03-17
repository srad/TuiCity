use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::Widget,
};
use crate::{
    app::camera::Camera,
    core::map::Map,
    ui::theme,
};

pub struct MiniMap<'a> {
    pub map: &'a Map,
    pub camera: &'a Camera,
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

        let render_area = Rect {
            y: area.y + 1,
            height: area.height.saturating_sub(1),
            ..area
        };

        if render_area.height == 0 {
            return;
        }

        let mw = self.map.width as f32;
        let mh = self.map.height as f32;

        for row in 0..render_area.height {
            for col in 0..render_area.width {
                let map_x =
                    ((col as f32 / render_area.width as f32) * mw) as usize;
                let map_y =
                    ((row as f32 / render_area.height as f32) * mh) as usize;

                let map_x = map_x.min(self.map.width.saturating_sub(1));
                let map_y = map_y.min(self.map.height.saturating_sub(1));

                let tile = self.map.get(map_x, map_y);
                let overlay = self.map.get_overlay(map_x, map_y);
                let glyph = theme::tile_glyph(tile, overlay);

                let cell = buf.cell_mut((render_area.x + col, render_area.y + row)).unwrap();
                cell.set_char(' ');
                cell.set_bg(glyph.bg);
            }
        }

        // Draw viewport rectangle
        let vx0 = ((self.camera.offset_x as f32 / mw) * render_area.width as f32) as u16;
        let vy0 = ((self.camera.offset_y as f32 / mh) * render_area.height as f32) as u16;
        let vx1 = (((self.camera.offset_x + self.camera.view_w as i32) as f32 / mw)
            * render_area.width as f32) as u16;
        let vy1 = (((self.camera.offset_y + self.camera.view_h as i32) as f32 / mh)
            * render_area.height as f32) as u16;

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
    }
}
