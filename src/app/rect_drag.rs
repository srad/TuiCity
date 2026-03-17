use crate::core::tool::Tool;

pub struct RectDrag {
    pub tool: Tool,
    pub start_x: usize,
    pub start_y: usize,
    pub end_x: usize,
    pub end_y: usize,
    /// Cached tile list, recomputed only when end changes.
    pub tiles_cache: Vec<(usize, usize)>,
}

impl RectDrag {
    pub fn new(tool: Tool, x: usize, y: usize) -> Self {
        Self {
            tool,
            start_x: x,
            start_y: y,
            end_x: x,
            end_y: y,
            tiles_cache: vec![(x, y)],
        }
    }

    pub fn update_end(&mut self, ex: usize, ey: usize) {
        if self.end_x == ex && self.end_y == ey {
            return;
        }
        self.end_x = ex;
        self.end_y = ey;
        let x0 = self.start_x.min(ex);
        let x1 = self.start_x.max(ex);
        let y0 = self.start_y.min(ey);
        let y1 = self.start_y.max(ey);
        self.tiles_cache.clear();
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.tiles_cache.push((x, y));
            }
        }
    }

    pub fn width(&self) -> usize {
        self.start_x.abs_diff(self.end_x) + 1
    }
    pub fn height(&self) -> usize {
        self.start_y.abs_diff(self.end_y) + 1
    }
}
