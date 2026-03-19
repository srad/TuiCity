use crate::core::tool::Tool;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolChooserKind {
    Zones,
    Transport,
    Utilities,
    PowerPlants,
    Buildings,
}

impl ToolChooserKind {
    pub fn title(self) -> &'static str {
        match self {
            ToolChooserKind::Zones => " Zone Selection ",
            ToolChooserKind::Transport => " Transport Selection ",
            ToolChooserKind::Utilities => " Utility Selection ",
            ToolChooserKind::PowerPlants => " Power Plant Selection ",
            ToolChooserKind::Buildings => " Building Selection ",
        }
    }

    pub fn button_label(self) -> &'static str {
        match self {
            ToolChooserKind::Zones => "Zones",
            ToolChooserKind::Transport => "Transport",
            ToolChooserKind::Utilities => "Utilities",
            ToolChooserKind::PowerPlants => "Power Plants",
            ToolChooserKind::Buildings => "Buildings",
        }
    }

    pub fn tools(self) -> &'static [Tool] {
        match self {
            ToolChooserKind::Zones => &[
                Tool::ZoneResLight,
                Tool::ZoneResDense,
                Tool::ZoneCommLight,
                Tool::ZoneCommDense,
                Tool::ZoneIndLight,
                Tool::ZoneIndDense,
            ],
            ToolChooserKind::Transport => &[
                Tool::Road,
                Tool::Highway,
                Tool::Onramp,
                Tool::Rail,
                Tool::BusDepot,
                Tool::RailDepot,
            ],
            ToolChooserKind::Utilities => &[
                Tool::PowerLine,
                Tool::WaterPipe,
                Tool::Subway,
                Tool::SubwayStation,
                Tool::WaterPump,
                Tool::WaterTower,
                Tool::WaterTreatment,
                Tool::Desalination,
            ],
            ToolChooserKind::PowerPlants => &[Tool::PowerPlantCoal, Tool::PowerPlantGas],
            ToolChooserKind::Buildings => &[Tool::Park, Tool::Police, Tool::Fire],
        }
    }

    pub fn for_tool(tool: Tool) -> Option<Self> {
        match tool {
            Tool::ZoneResLight
            | Tool::ZoneResDense
            | Tool::ZoneCommLight
            | Tool::ZoneCommDense
            | Tool::ZoneIndLight
            | Tool::ZoneIndDense => Some(ToolChooserKind::Zones),
            Tool::Road
            | Tool::Highway
            | Tool::Onramp
            | Tool::Rail
            | Tool::BusDepot
            | Tool::RailDepot => Some(ToolChooserKind::Transport),
            Tool::PowerLine
            | Tool::WaterPipe
            | Tool::Subway
            | Tool::SubwayStation
            | Tool::WaterPump
            | Tool::WaterTower
            | Tool::WaterTreatment
            | Tool::Desalination => Some(ToolChooserKind::Utilities),
            Tool::PowerPlantCoal | Tool::PowerPlantGas => Some(ToolChooserKind::PowerPlants),
            Tool::Park | Tool::Police | Tool::Fire => Some(ToolChooserKind::Buildings),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarHitTarget {
    SelectTool(Tool),
    OpenChooser(ToolChooserKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolbarHitArea {
    pub area: ClickArea,
    pub target: ToolbarHitTarget,
}

#[derive(Default)]
pub struct UiAreas {
    pub menu_bar: ClickArea,
    pub menu_items: [ClickArea; 6],
    pub menu_popup: ClickArea,
    pub menu_popup_items: Vec<ClickArea>,
    pub map: MapUiAreas,
    pub minimap: ClickArea,
    pub pause_btn: ClickArea,
    pub layer_surface_btn: ClickArea,
    pub layer_underground_btn: ClickArea,
    pub toolbar_items: Vec<ToolbarHitArea>,
    pub tool_chooser_items: Vec<ClickArea>,
    pub dialog_items: Vec<ClickArea>,
}

#[derive(Clone, Debug)]
pub struct WindowState {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub visible: bool,
    pub movable: bool,
    pub closable: bool,
    pub shadowed: bool,
    pub modal: bool,
    pub center_on_open: bool,
    pub title: &'static str,
}

impl WindowState {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
            visible: true,
            movable: true,
            closable: false,
            shadowed: true,
            modal: false,
            center_on_open: false,
            title: "",
        }
    }

    pub fn title_bar_contains(&self, col: u16, row: u16) -> bool {
        self.width > 0 && row == self.y && col >= self.x && col < self.x + self.width
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

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct UiRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl UiRect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowId {
    Map,
    Panel,
    Budget,
    Statistics,
    Inspect,
    PowerPicker,
    Help,
    About,
}

impl WindowId {
    pub const ALL: [WindowId; 8] = [
        WindowId::Map,
        WindowId::Panel,
        WindowId::Budget,
        WindowId::Statistics,
        WindowId::Inspect,
        WindowId::PowerPicker,
        WindowId::Help,
        WindowId::About,
    ];

    pub fn index(self) -> usize {
        match self {
            WindowId::Map => 0,
            WindowId::Panel => 1,
            WindowId::Budget => 2,
            WindowId::Statistics => 3,
            WindowId::Inspect => 4,
            WindowId::PowerPicker => 5,
            WindowId::Help => 6,
            WindowId::About => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowDragState {
    pub id: WindowId,
    pub offset_x: u16,
    pub offset_y: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowLayout {
    pub outer: UiRect,
    pub inner: UiRect,
    pub title_bar: UiRect,
    pub close_button: UiRect,
}

#[derive(Clone, Debug)]
pub struct DesktopLayout {
    pub menu_bar: UiRect,
    pub status_bar: UiRect,
    pub news_ticker: UiRect,
    windows: [WindowLayout; 8],
}

impl DesktopLayout {
    pub fn window(&self, id: WindowId) -> WindowLayout {
        self.windows[id.index()]
    }
}

#[derive(Clone, Debug)]
pub struct DesktopState {
    windows: [WindowState; 8],
    pub drag: Option<WindowDragState>,
    pub z_order: Vec<WindowId>,
}

impl DesktopState {
    pub fn new_ingame() -> Self {
        let mut map = WindowState::new(0, 2, 999, 999);
        map.title = " MAP ";

        let mut panel = WindowState::new(u16::MAX, 4, 24, 35);
        panel.title = " TOOLBOX ";
        panel.closable = true;

        let mut budget = WindowState::new(8, 4, 74, 29);
        budget.title = " Budget Control Center ";
        budget.visible = false;
        budget.closable = true;
        budget.modal = true;

        let mut statistics = WindowState::new(0, 0, 86, 24);
        statistics.title = " City Statistics ";
        statistics.visible = false;
        statistics.closable = true;
        statistics.modal = true;
        statistics.center_on_open = true;

        let mut inspect = WindowState::new(15, 5, 34, 16);
        inspect.title = " Inspect ";
        inspect.visible = false;
        inspect.closable = true;

        let mut power = WindowState::new(0, 0, 38, 16);
        power.title = " Tool Selection ";
        power.visible = false;
        power.closable = true;
        power.modal = true;
        power.center_on_open = true;

        let mut help = WindowState::new(0, 0, 74, 25);
        help.title = " Help ";
        help.visible = false;
        help.closable = true;
        help.modal = true;
        help.center_on_open = true;

        let mut about = WindowState::new(0, 0, 60, 8);
        about.title = " About ";
        about.visible = false;
        about.closable = true;
        about.modal = true;
        about.center_on_open = true;

        Self {
            windows: [map, panel, budget, statistics, inspect, power, help, about],
            drag: None,
            z_order: vec![
                WindowId::Map,
                WindowId::Panel,
                WindowId::Budget,
                WindowId::Statistics,
                WindowId::Inspect,
                WindowId::PowerPicker,
                WindowId::Help,
                WindowId::About,
            ],
        }
    }

    pub fn window(&self, id: WindowId) -> &WindowState {
        &self.windows[id.index()]
    }

    pub fn window_mut(&mut self, id: WindowId) -> &mut WindowState {
        &mut self.windows[id.index()]
    }

    pub fn is_open(&self, id: WindowId) -> bool {
        self.window(id).visible
    }

    pub fn open(&mut self, id: WindowId, centered: bool) {
        let win = self.window_mut(id);
        win.visible = true;
        if centered {
            win.center_on_open = true;
        }
        self.focus(id);
    }

    pub fn close(&mut self, id: WindowId) {
        let win = self.window_mut(id);
        win.visible = false;
        win.center_on_open = false;
        if self.drag.map(|drag| drag.id) == Some(id) {
            self.drag = None;
        }
    }

    pub fn toggle(&mut self, id: WindowId, centered: bool) {
        if self.is_open(id) {
            self.close(id);
        } else {
            self.open(id, centered);
        }
    }

    pub fn focus(&mut self, id: WindowId) {
        self.z_order.retain(|&existing| existing != id);
        self.z_order.push(id);
    }

    pub fn begin_drag(&mut self, id: WindowId, col: u16, row: u16) -> bool {
        let (visible, movable, x, y, title_contains) = {
            let win = self.window(id);
            (
                win.visible,
                win.movable,
                win.x,
                win.y,
                win.title_bar_contains(col, row),
            )
        };
        if !visible || !movable || !title_contains {
            return false;
        }
        self.focus(id);
        self.drag = Some(WindowDragState {
            id,
            offset_x: col.saturating_sub(x),
            offset_y: row.saturating_sub(y),
        });
        true
    }

    pub fn update_drag(&mut self, col: u16, row: u16) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        let win = self.window_mut(drag.id);
        win.x = col.saturating_sub(drag.offset_x);
        win.y = row.saturating_sub(drag.offset_y);
        true
    }

    pub fn end_drag(&mut self) -> Option<WindowId> {
        self.drag.take().map(|drag| drag.id)
    }

    pub fn contains(&self, id: WindowId, col: u16, row: u16) -> bool {
        let win = self.window(id);
        win.visible && win.contains(col, row)
    }

    pub fn layout(&mut self, full: UiRect) -> DesktopLayout {
        let menu_bar = UiRect::new(full.x, full.y, full.width, 1);
        let status_bar = UiRect::new(full.x, full.y.saturating_add(1), full.width, 1);
        let news_ticker = UiRect::new(
            full.x,
            full.y + full.height.saturating_sub(1),
            full.width,
            full.height.min(1),
        );
        let desktop = UiRect::new(
            full.x,
            full.y.saturating_add(2),
            full.width,
            full.height.saturating_sub(3),
        );
        let mut windows = [WindowLayout::default(); 8];

        for id in WindowId::ALL {
            let win = self.window_mut(id);
            if !win.visible {
                continue;
            }
            if win.center_on_open {
                let fitted = centered_fit_rect(desktop, win.width, win.height);
                win.width = fitted.width;
                win.height = fitted.height;
                win.x = fitted.x;
                win.y = fitted.y;
                win.center_on_open = false;
            }

            let outer = clamp_window_to_desktop(win, desktop);
            let inner = UiRect::new(
                outer.x.saturating_add(1),
                outer.y.saturating_add(1),
                outer.width.saturating_sub(2),
                outer.height.saturating_sub(2),
            );
            let title_bar = UiRect::new(outer.x, outer.y, outer.width, outer.height.min(1));
            let close_button = if win.closable && outer.width >= 5 && outer.height > 0 {
                UiRect::new(outer.x + outer.width.saturating_sub(5), outer.y, 5, 1)
            } else {
                UiRect::default()
            };
            windows[id.index()] = WindowLayout {
                outer,
                inner,
                title_bar,
                close_button,
            };
        }

        DesktopLayout {
            menu_bar,
            status_bar,
            news_ticker,
            windows,
        }
    }
}

pub fn centered_fit_rect(desktop: UiRect, desired_w: u16, desired_h: u16) -> UiRect {
    if desktop.width == 0 || desktop.height == 0 {
        return UiRect::default();
    }

    let width = desired_w.min(desktop.width);
    let height = desired_h.min(desktop.height);
    let x = desktop.x + desktop.width.saturating_sub(width) / 2;
    let y = desktop.y + desktop.height.saturating_sub(height) / 2;
    UiRect::new(x, y, width, height)
}

pub fn clamp_window_to_desktop(window: &mut WindowState, desktop: UiRect) -> UiRect {
    let h = window.height.max(4);
    let w = window.width.max(6);
    if window.x == u16::MAX {
        window.x = desktop.x + desktop.width.saturating_sub(w).saturating_sub(3);
    }

    let min_x = (desktop.x + 4).saturating_sub(w);
    let max_x = desktop.x
        + desktop
            .width
            .saturating_sub(4)
            .min(desktop.width.saturating_sub(w));
    let x = window.x.clamp(min_x, max_x);
    let y = window
        .y
        .clamp(desktop.y, desktop.y + desktop.height.saturating_sub(1));
    window.x = x;
    window.y = y;

    let right = (x + w).min(desktop.x + desktop.width);
    let actual_w = right.saturating_sub(x).max(1);
    let bottom = (y + h).min(desktop.y + desktop.height);
    let actual_h = bottom.saturating_sub(y).max(1);
    UiRect::new(x, y, actual_w, actual_h)
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
        let desktop = UiRect::new(0, 2, 40, 12);
        let fitted = centered_fit_rect(desktop, 74, 29);
        assert_eq!(fitted.width, 40);
        assert_eq!(fitted.height, 12);
        assert_eq!(fitted.x, 0);
        assert_eq!(fitted.y, 2);
    }

    #[test]
    fn centered_fit_rect_centers_smaller_window() {
        let desktop = UiRect::new(0, 2, 100, 40);
        let fitted = centered_fit_rect(desktop, 74, 29);
        assert_eq!(fitted.width, 74);
        assert_eq!(fitted.height, 29);
        assert_eq!(fitted.x, 13);
        assert_eq!(fitted.y, 7);
    }

    #[test]
    fn desktop_state_opens_centers_and_closes_modal_windows() {
        let mut desktop = DesktopState::new_ingame();
        assert!(!desktop.is_open(WindowId::Budget));

        desktop.open(WindowId::Budget, true);
        let layout = desktop.layout(UiRect::new(0, 0, 100, 40));
        let budget = layout.window(WindowId::Budget);

        assert!(desktop.is_open(WindowId::Budget));
        assert!(budget.outer.width <= 100);
        assert!(budget.outer.height <= 37);
        assert_eq!(layout.news_ticker.y, 39);

        desktop.close(WindowId::Budget);
        assert!(!desktop.is_open(WindowId::Budget));
    }

    #[test]
    fn desktop_drag_updates_window_position() {
        let mut desktop = DesktopState::new_ingame();
        let _ = desktop.layout(UiRect::new(0, 0, 100, 40));
        let panel_start = desktop.window(WindowId::Panel).clone();
        assert!(desktop.begin_drag(WindowId::Panel, panel_start.x + 5, panel_start.y));
        assert!(desktop.update_drag(panel_start.x + 12, panel_start.y + 6));

        let panel = desktop.window(WindowId::Panel);
        assert_eq!(panel.x, panel_start.x + 7);
        assert_eq!(panel.y, panel_start.y + 6);
        assert_eq!(desktop.end_drag(), Some(WindowId::Panel));
    }
}
