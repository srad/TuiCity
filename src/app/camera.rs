#[derive(Clone, Debug)]
pub struct Camera {
    pub offset_x: i32,
    pub offset_y: i32,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub view_w: usize,
    pub view_h: usize,
    /// Columns per map tile in the active frontend's screen coordinate system.
    /// 2 for the terminal (2 chars per tile), 1 for the pixel frontend (1 cell per tile).
    pub col_scale: u8,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            offset_x: 0,
            offset_y: 0,
            cursor_x: 10,
            cursor_y: 10,
            view_w: 80,
            view_h: 40,
            col_scale: 2,
        }
    }
}

impl Camera {
    pub fn move_cursor(&mut self, dx: i32, dy: i32, map_w: usize, map_h: usize) {
        let nx = (self.cursor_x as i32 + dx).max(0).min(map_w as i32 - 1);
        let ny = (self.cursor_y as i32 + dy).max(0).min(map_h as i32 - 1);
        self.cursor_x = nx as usize;
        self.cursor_y = ny as usize;
        self.scroll_to_cursor(map_w, map_h);
    }

    pub fn pan(&mut self, dx: i32, dy: i32, map_w: usize, map_h: usize) {
        let max_x = (map_w as i32 - self.view_w as i32).max(0);
        let max_y = (map_h as i32 - self.view_h as i32).max(0);
        self.offset_x = (self.offset_x + dx).clamp(0, max_x);
        self.offset_y = (self.offset_y + dy).clamp(0, max_y);
    }

    pub fn center_on(&mut self, tile_x: usize, tile_y: usize, map_w: usize, map_h: usize) {
        let max_x = (map_w as i32 - self.view_w as i32).max(0);
        let max_y = (map_h as i32 - self.view_h as i32).max(0);
        let clamped_x = tile_x.min(map_w.saturating_sub(1));
        let clamped_y = tile_y.min(map_h.saturating_sub(1));
        self.cursor_x = clamped_x;
        self.cursor_y = clamped_y;
        self.offset_x = (clamped_x as i32 - self.view_w as i32 / 2).clamp(0, max_x);
        self.offset_y = (clamped_y as i32 - self.view_h as i32 / 2).clamp(0, max_y);
    }

    pub fn scroll_to_cursor(&mut self, map_w: usize, map_h: usize) {
        let margin = 4i32;
        let vw = self.view_w as i32;
        let vh = self.view_h as i32;
        let cx = self.cursor_x as i32;
        let cy = self.cursor_y as i32;

        if cx < self.offset_x + margin {
            self.offset_x = (cx - margin).max(0);
        }
        if cx >= self.offset_x + vw - margin {
            self.offset_x = (cx - vw + margin + 1).clamp(0, (map_w as i32 - vw).max(0));
        }
        if cy < self.offset_y + margin {
            self.offset_y = (cy - margin).max(0);
        }
        if cy >= self.offset_y + vh - margin {
            self.offset_y = (cy - vh + margin + 1).clamp(0, (map_h as i32 - vh).max(0));
        }
    }

    pub fn screen_to_map(&self, sx: u16, sy: u16) -> (usize, usize) {
        let mx = ((sx as i32) / self.col_scale as i32 + self.offset_x).max(0) as usize;
        let my = (sy as i32 + self.offset_y).max(0) as usize;
        (mx, my)
    }
}
