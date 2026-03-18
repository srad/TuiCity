use ratatui::layout::Rect;

#[derive(Clone, Copy, Default, Debug)]
pub struct ClickArea {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl ClickArea {
    pub fn contains(&self, col: u16, row: u16) -> bool {
        self.width > 0
            && self.height > 0
            && col >= self.x
            && col < self.x + self.width
            && row >= self.y
            && row < self.y + self.height
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct MapUiAreas {
    pub viewport: ClickArea,
    pub vertical_bar: ClickArea,
    pub vertical_dec: ClickArea,
    pub vertical_track: ClickArea,
    pub vertical_thumb: ClickArea,
    pub vertical_inc: ClickArea,
    pub horizontal_bar: ClickArea,
    pub horizontal_dec: ClickArea,
    pub horizontal_track: ClickArea,
    pub horizontal_thumb: ClickArea,
    pub horizontal_inc: ClickArea,
    pub corner: ClickArea,
}

#[derive(Default)]
pub struct UiAreas {
    pub map: MapUiAreas,
    pub minimap: ClickArea,
    pub pause_btn: ClickArea,
}

#[derive(Clone, Debug)]
pub struct WindowState {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl WindowState {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub fn title_bar_contains(&self, col: u16, row: u16) -> bool {
        self.width > 0
            && row == self.y
            && col >= self.x
            && col < self.x + self.width
    }

    pub fn contains(&self, col: u16, row: u16) -> bool {
        self.width > 0
            && self.height > 0
            && col >= self.x
            && col < self.x + self.width
            && row >= self.y
            && row < self.y + self.height
    }
}

pub fn centered_fit_rect(desktop: Rect, desired_w: u16, desired_h: u16) -> Rect {
    if desktop.width == 0 || desktop.height == 0 {
        return Rect::default();
    }

    let width = desired_w.min(desktop.width);
    let height = desired_h.min(desktop.height);
    let x = desktop.x + desktop.width.saturating_sub(width) / 2;
    let y = desktop.y + desktop.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

pub fn clamp_window_to_desktop(window: &mut WindowState, desktop: Rect) -> Rect {
    let h = window.height.max(4);
    let w = window.width.max(6);
    if window.x == u16::MAX {
        window.x = desktop.x + desktop.width.saturating_sub(w);
    }

    let min_x = (desktop.x + 4).saturating_sub(w);
    let max_x = desktop.x + desktop.width.saturating_sub(4).min(desktop.width.saturating_sub(w));
    let x = window.x.clamp(min_x, max_x);
    let y = window.y.clamp(desktop.y, desktop.y + desktop.height.saturating_sub(1));
    window.x = x;
    window.y = y;

    let right = (x + w).min(desktop.x + desktop.width);
    let actual_w = right.saturating_sub(x).max(1);
    let bottom = (y + h).min(desktop.y + desktop.height);
    let actual_h = bottom.saturating_sub(y).max(1);
    Rect::new(x, y, actual_w, actual_h)
}

pub fn cycle_next<T: Copy + Eq>(current: T, order: &[T]) -> T {
    let idx = order.iter().position(|&item| item == current).unwrap_or(0);
    order[(idx + 1) % order.len()]
}

pub fn cycle_prev<T: Copy + Eq>(current: T, order: &[T]) -> T {
    let idx = order.iter().position(|&item| item == current).unwrap_or(0);
    order[(idx + order.len() - 1) % order.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_fit_rect_never_exceeds_desktop() {
        let desktop = Rect::new(0, 2, 40, 12);
        let fitted = centered_fit_rect(desktop, 74, 29);
        assert_eq!(fitted.width, 40);
        assert_eq!(fitted.height, 12);
        assert_eq!(fitted.x, 0);
        assert_eq!(fitted.y, 2);
    }

    #[test]
    fn centered_fit_rect_centers_smaller_window() {
        let desktop = Rect::new(0, 2, 100, 40);
        let fitted = centered_fit_rect(desktop, 74, 29);
        assert_eq!(fitted.width, 74);
        assert_eq!(fitted.height, 29);
        assert_eq!(fitted.x, 13);
        assert_eq!(fitted.y, 7);
    }
}
