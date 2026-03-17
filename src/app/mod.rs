pub mod camera;
pub mod input;
pub mod line_drag;
pub mod rect_drag;
pub mod save;

use line_drag::{LineDrag, line_shortest_path};
use rect_drag::RectDrag;

use camera::Camera;
use input::Action;
use crate::core::{
    map::{gen, Map, Tile},
    sim::SimState,
    tool::Tool,
};

// ── Click-area type (no ratatui dependency) ──────────────────────────────────

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

// ── UI Areas (written by ui::render each frame) ───────────────────────────────

#[derive(Default)]
pub struct UiAreas {
    pub map: ClickArea,
    pub minimap: ClickArea,
    pub pause_btn: ClickArea,
    pub toolbar_buttons: Vec<(Tool, ClickArea)>,
}

// ── Screen-specific state ─────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NewCityField {
    CityName,
    SeedInput,
    WaterSlider,
    TreesSlider,
    RegenerateBtn,
    StartBtn,
    BackBtn,
}

impl NewCityField {
    const ALL: [NewCityField; 7] = [
        NewCityField::CityName,
        NewCityField::SeedInput,
        NewCityField::WaterSlider,
        NewCityField::TreesSlider,
        NewCityField::RegenerateBtn,
        NewCityField::StartBtn,
        NewCityField::BackBtn,
    ];

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

pub struct StartState {
    pub selected: usize,
}

impl Default for StartState {
    fn default() -> Self {
        Self { selected: 0 }
    }
}

pub struct NewCityState {
    pub city_name: String,
    pub seed: u64,
    pub seed_input: String,   // editable hex — synced with seed on focus-leave / Enter
    pub water_pct: u8,
    pub trees_pct: u8,
    pub preview_map: Map,
    pub focused_field: NewCityField,
}

impl NewCityState {
    pub fn new() -> Self {
        let seed = rand::random::<u64>();
        let params = gen::GenParams { seed, ..Default::default() };
        let preview_map = gen::generate(&params);
        Self {
            city_name: String::new(),
            seed,
            seed_input: format!("{:016X}", seed),
            water_pct: params.water_pct,
            trees_pct: params.trees_pct,
            preview_map,
            focused_field: NewCityField::CityName,
        }
    }

    /// Randomise seed and rebuild map.
    pub fn regenerate(&mut self) {
        self.seed = rand::random::<u64>();
        self.seed_input = format!("{:016X}", self.seed);
        self.rebuild_map();
    }

    /// Try to parse `seed_input` as a hex (or decimal) u64, then rebuild.
    pub fn apply_seed_input(&mut self) {
        let raw = self.seed_input.trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let parsed = u64::from_str_radix(raw, 16)
            .or_else(|_| self.seed_input.trim().parse::<u64>());
        if let Ok(new_seed) = parsed {
            self.seed = new_seed;
        }
        // Always reformat to canonical hex so the field stays clean
        self.seed_input = format!("{:016X}", self.seed);
        self.rebuild_map();
    }

    pub fn rebuild_map(&mut self) {
        let params = gen::GenParams {
            water_pct: self.water_pct,
            trees_pct: self.trees_pct,
            seed: self.seed,
            ..Default::default()
        };
        self.preview_map = gen::generate(&params);
    }
}

pub struct LoadCityState {
    pub saves: Vec<save::SaveEntry>,
    pub selected: usize,
}

// ── Screen enum ───────────────────────────────────────────────────────────────

pub enum Screen {
    Start(StartState),
    NewCity(NewCityState),
    LoadCity(LoadCityState),
    InGame,
}

// ── AppState ──────────────────────────────────────────────────────────────────

pub struct AppState {
    pub screen: Screen,
    pub map: Map,
    pub camera: Camera,
    pub sim: SimState,
    pub current_tool: Tool,
    pub ui_areas: UiAreas,
    pub running: bool,
    pub paused: bool,
    pub message: Option<String>,
    pub ticks_since_month: u32,
    pub ticks_per_month: u32,
    pub line_drag: Option<LineDrag>,
    pub rect_drag: Option<RectDrag>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            screen: Screen::Start(StartState::default()),
            map: Map::new(128, 128),
            camera: Camera::default(),
            sim: SimState::default(),
            current_tool: Tool::Inspect,
            ui_areas: UiAreas::default(),
            running: true,
            paused: false,
            message: None,
            ticks_since_month: 0,
            ticks_per_month: 50,
            line_drag: None,
            rect_drag: None,
        }
    }

    pub fn on_tick(&mut self) {
        if !matches!(self.screen, Screen::InGame) {
            return;
        }
        if self.paused {
            return;
        }
        // Clear ephemeral messages after a few ticks (but not while dragging)
        if self.message.is_some() && self.line_drag.is_none() && self.rect_drag.is_none() {
            self.ticks_since_month += 1;
            if self.ticks_since_month % 20 == 0 {
                self.message = None;
            }
        }
        self.ticks_since_month += 1;
        if self.ticks_since_month >= self.ticks_per_month {
            self.ticks_since_month = 0;
            self.sim.advance_month(&mut self.map);
        }
    }

    pub fn on_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.running = false;
                return;
            }
            Action::SaveGame => {
                if matches!(self.screen, Screen::InGame) {
                    match save::save_city(&self.sim, &self.map) {
                        Ok(()) => self.message = Some("City saved!".to_string()),
                        Err(e) => self.message = Some(format!("Save failed: {e}")),
                    }
                }
                return;
            }
            _ => {}
        }

        match &self.screen {
            Screen::Start(_) => self.handle_start(action),
            Screen::NewCity(_) => self.handle_new_city(action),
            Screen::LoadCity(_) => self.handle_load_city(action),
            Screen::InGame => self.handle_ingame(action),
        }
    }

    // ── Start screen handler ─────────────────────────────────────────────────

    fn handle_start(&mut self, action: Action) {
        const N: usize = 3;
        let selected = if let Screen::Start(ref s) = self.screen {
            s.selected
        } else {
            return;
        };

        match action {
            Action::CharInput('q') => {
                self.running = false;
            }
            Action::MoveCursor(_, dy) => {
                let new_sel = if dy > 0 {
                    (selected + 1) % N
                } else if dy < 0 {
                    selected.checked_sub(1).unwrap_or(N - 1)
                } else {
                    selected
                };
                if let Screen::Start(ref mut s) = self.screen {
                    s.selected = new_sel;
                }
            }
            Action::MenuSelect | Action::MouseClick { .. } => {
                // For mouse clicks on start screen, re-check which was clicked
                // For simplicity, keyboard Enter uses `selected`
                self.activate_start_option(selected);
            }
            _ => {}
        }
    }

    fn activate_start_option(&mut self, idx: usize) {
        match idx {
            0 => {
                // Load City
                let saves = save::list_saves();
                self.screen = Screen::LoadCity(LoadCityState { saves, selected: 0 });
            }
            1 => {
                // New City
                self.screen = Screen::NewCity(NewCityState::new());
            }
            2 => {
                self.running = false;
            }
            _ => {}
        }
    }

    // ── New City handler ─────────────────────────────────────────────────────

    fn handle_new_city(&mut self, action: Action) {
        let (focused, water, trees) = if let Screen::NewCity(ref s) = self.screen {
            (s.focused_field, s.water_pct, s.trees_pct)
        } else {
            return;
        };

        match action {
            Action::MenuBack => {
                self.screen = Screen::Start(StartState::default());
                return;
            }
            Action::MoveCursor(dx, dy) => {
                if dy != 0 {
                    // Leaving SeedInput: commit whatever was typed
                    if focused == NewCityField::SeedInput {
                        if let Screen::NewCity(ref mut s) = self.screen {
                            s.apply_seed_input();
                        }
                    }
                    let next = if dy > 0 { focused.next() } else { focused.prev() };
                    if let Screen::NewCity(ref mut s) = self.screen {
                        s.focused_field = next;
                    }
                } else if dx != 0 {
                    match focused {
                        NewCityField::WaterSlider => {
                            let new_val = (water as i32 + dx * 5).clamp(0, 90) as u8;
                            if let Screen::NewCity(ref mut s) = self.screen {
                                s.water_pct = new_val;
                                s.rebuild_map();
                            }
                        }
                        NewCityField::TreesSlider => {
                            let new_val = (trees as i32 + dx * 5).clamp(0, 90) as u8;
                            if let Screen::NewCity(ref mut s) = self.screen {
                                s.trees_pct = new_val;
                                s.rebuild_map();
                            }
                        }
                        _ => {}
                    }
                }
            }
            Action::MenuSelect => {
                match focused {
                    NewCityField::SeedInput => {
                        // Enter on seed field commits the typed value
                        if let Screen::NewCity(ref mut s) = self.screen {
                            s.apply_seed_input();
                        }
                    }
                    NewCityField::RegenerateBtn => {
                        if let Screen::NewCity(ref mut s) = self.screen {
                            s.regenerate();
                        }
                    }
                    NewCityField::StartBtn => {
                        self.start_new_game();
                    }
                    NewCityField::BackBtn => {
                        self.screen = Screen::Start(StartState::default());
                    }
                    _ => {}
                }
            }
            Action::CharInput(c) => {
                match focused {
                    NewCityField::CityName => {
                        if let Screen::NewCity(ref mut s) = self.screen {
                            if s.city_name.len() < 24 {
                                s.city_name.push(c);
                            }
                        }
                    }
                    NewCityField::SeedInput => {
                        if let Screen::NewCity(ref mut s) = self.screen {
                            // Only hex digits (and 'x' for 0x prefix)
                            if c.is_ascii_hexdigit() || c == 'x' || c == 'X' {
                                if s.seed_input.len() < 18 {
                                    s.seed_input.push(c.to_ascii_uppercase());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Action::DeleteChar => {
                match focused {
                    NewCityField::CityName => {
                        if let Screen::NewCity(ref mut s) = self.screen {
                            s.city_name.pop();
                        }
                    }
                    NewCityField::SeedInput => {
                        if let Screen::NewCity(ref mut s) = self.screen {
                            s.seed_input.pop();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn start_new_game(&mut self) {
        let placeholder = Screen::InGame;
        let old = std::mem::replace(&mut self.screen, placeholder);
        if let Screen::NewCity(state) = old {
            self.map = state.preview_map;
            self.sim = SimState::default();
            self.sim.city_name = if state.city_name.is_empty() {
                "New City".to_string()
            } else {
                state.city_name
            };
            self.camera = Camera::default();
            self.current_tool = Tool::Inspect;
            self.paused = false;
            self.ticks_since_month = 0;
            self.message = None;
            self.line_drag = None;
            self.rect_drag = None;
        }
    }

    // ── Load City handler ────────────────────────────────────────────────────

    fn handle_load_city(&mut self, action: Action) {
        let (count, selected) = if let Screen::LoadCity(ref s) = self.screen {
            (s.saves.len(), s.selected)
        } else {
            return;
        };

        match action {
            Action::MenuBack => {
                self.screen = Screen::Start(StartState::default());
            }
            Action::MoveCursor(_, dy) => {
                if count == 0 {
                    return;
                }
                let new_sel = if dy > 0 {
                    (selected + 1) % count
                } else {
                    selected.checked_sub(1).unwrap_or(count.saturating_sub(1))
                };
                if let Screen::LoadCity(ref mut s) = self.screen {
                    s.selected = new_sel;
                }
            }
            Action::MenuSelect => {
                if count == 0 {
                    return;
                }
                let path = if let Screen::LoadCity(ref s) = self.screen {
                    s.saves.get(s.selected).map(|e| e.path.clone())
                } else {
                    None
                };
                if let Some(path) = path {
                    match save::load_city(&path) {
                        Ok((map, sim)) => {
                            self.map = map;
                            self.sim = sim;
                            self.camera = Camera::default();
                            self.current_tool = Tool::Inspect;
                            self.paused = false;
                            self.ticks_since_month = 0;
                            self.message = Some("City loaded!".to_string());
                            self.line_drag = None;
                            self.rect_drag = None;
                            self.screen = Screen::InGame;
                        }
                        Err(e) => {
                            self.message = Some(format!("Load failed: {e}"));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ── In-Game handler ──────────────────────────────────────────────────────

    fn handle_ingame(&mut self, action: Action) {
        match action {
            Action::MenuBack => {
                if self.rect_drag.is_some() {
                    self.rect_drag = None;
                    self.message = None;
                } else if self.line_drag.is_some() {
                    self.line_drag = None;
                    self.message = None;
                } else {
                    self.screen = Screen::Start(StartState::default());
                    self.map = Map::new(128, 128);
                }
            }
            Action::MoveCursor(dx, dy) => {
                let (mw, mh) = (self.map.width, self.map.height);
                self.camera.move_cursor(dx, dy, mw, mh);
                // Place tool on keyboard move — single-tile tools only
                if self.current_tool != Tool::Inspect && !Tool::uses_footprint_preview(self.current_tool) {
                    self.place_current_tool();
                }
            }
            Action::PanCamera(dx, dy) => {
                let (mw, mh) = (self.map.width, self.map.height);
                self.camera.pan(dx, dy, mw, mh);
            }
            Action::CharInput(c) => {
                let new_tool = match c {
                    'q' => { self.running = false; None }
                    ' ' => { self.paused = !self.paused; None }
                    '?' => Some(Tool::Inspect),
                    '1' => Some(Tool::ZoneRes),
                    '2' => Some(Tool::ZoneComm),
                    '3' => Some(Tool::ZoneInd),
                    'r' => Some(Tool::Road),
                    'l' => Some(Tool::Rail),
                    'p' => Some(Tool::PowerLine),
                    'e' => Some(Tool::PowerPlant),
                    'k' => Some(Tool::Park),
                    's' => Some(Tool::Police),
                    'f' => Some(Tool::Fire),
                    'b' => Some(Tool::Bulldoze),
                    _ => None,
                };
                if let Some(tool) = new_tool {
                    self.current_tool = tool;
                    self.line_drag = None;
                    self.rect_drag = None;
                }
            }
            Action::MouseClick { col, row } => {
                if Tool::uses_line_drag(self.current_tool) && self.ui_areas.map.contains(col, row) {
                    let (mx, my) = self.screen_to_map_clamped(col, row);
                    self.camera.cursor_x = mx;
                    self.camera.cursor_y = my;
                    self.line_drag = Some(LineDrag::new(self.current_tool, mx, my));
                    self.update_line_drag_message();
                } else if Tool::uses_rect_drag(self.current_tool) && self.ui_areas.map.contains(col, row) {
                    let (mx, my) = self.screen_to_map_clamped(col, row);
                    self.camera.cursor_x = mx;
                    self.camera.cursor_y = my;
                    self.rect_drag = Some(RectDrag::new(self.current_tool, mx, my));
                    self.update_rect_drag_message();
                } else {
                    self.handle_click(col, row, true);
                }
            }
            Action::MouseDrag { col, row } => {
                if self.line_drag.is_some() && self.ui_areas.map.contains(col, row) {
                    let (mx, my) = self.screen_to_map_clamped(col, row);
                    let (tool, sx, sy) = self.line_drag.as_ref()
                        .map(|d| (d.tool, d.start_x, d.start_y))
                        .unwrap();
                    let new_path = line_shortest_path(&self.map, tool, sx, sy, mx, my);
                    if let Some(ref mut drag) = self.line_drag {
                        drag.end_x = mx;
                        drag.end_y = my;
                        drag.path = new_path;
                    }
                    self.camera.cursor_x = mx;
                    self.camera.cursor_y = my;
                    self.update_line_drag_message();
                } else if self.rect_drag.is_some() && self.ui_areas.map.contains(col, row) {
                    let (mx, my) = self.screen_to_map_clamped(col, row);
                    if let Some(ref mut drag) = self.rect_drag { drag.update_end(mx, my); }
                    self.camera.cursor_x = mx;
                    self.camera.cursor_y = my;
                    self.update_rect_drag_message();
                } else if self.line_drag.is_none() && self.rect_drag.is_none() {
                    self.handle_click(col, row, false);
                }
                // if drag active but outside map: keep last preview, ignore
            }
            Action::MouseUp { col, row } => {
                if self.line_drag.is_some() {
                    if self.ui_areas.map.contains(col, row) {
                        let (mx, my) = self.screen_to_map_clamped(col, row);
                        let (tool, sx, sy) = self.line_drag.as_ref()
                            .map(|d| (d.tool, d.start_x, d.start_y))
                            .unwrap();
                        let final_path = line_shortest_path(&self.map, tool, sx, sy, mx, my);
                        if let Some(ref mut drag) = self.line_drag {
                            drag.end_x = mx;
                            drag.end_y = my;
                            drag.path = final_path;
                        }
                    }
                    self.commit_line_drag();
                } else if self.rect_drag.is_some() {
                    if self.ui_areas.map.contains(col, row) {
                        let (mx, my) = self.screen_to_map_clamped(col, row);
                        if let Some(ref mut drag) = self.rect_drag { drag.update_end(mx, my); }
                    }
                    self.commit_rect_drag();
                }
            }
            Action::MouseMove { col, row } => {
                if Tool::uses_footprint_preview(self.current_tool)
                    && self.ui_areas.map.contains(col, row)
                {
                    let (mx, my) = self.screen_to_map_clamped(col, row);
                    self.camera.cursor_x = mx;
                    self.camera.cursor_y = my;
                }
            }
            _ => {}
        }
    }

    fn screen_to_map_clamped(&self, col: u16, row: u16) -> (usize, usize) {
        let sx = col - self.ui_areas.map.x;
        let sy = row - self.ui_areas.map.y;
        let (mx, my) = self.camera.screen_to_map(sx, sy);
        (mx.min(self.map.width.saturating_sub(1)),
         my.min(self.map.height.saturating_sub(1)))
    }

    fn commit_line_drag(&mut self) {
        let drag = match self.line_drag.take() { Some(d) => d, None => return };
        let cost_per = drag.tool.cost();
        for (x, y) in drag.path {
            if x >= self.map.width || y >= self.map.height { continue; }
            let existing = self.map.get(x, y);
            if !drag.tool.can_place(existing) { continue; }
            if self.sim.treasury < cost_per {
                self.message = Some("Insufficient funds!".to_string());
                return;
            }
            let new_tile = match (drag.tool, existing) {
                (Tool::Road, Tile::PowerLine) | (Tool::PowerLine, Tile::Road) => Tile::RoadPowerLine,
                _ => match drag.tool.target_tile() { Some(t) => t, None => continue },
            };
            self.map.set(x, y, new_tile);
            self.sim.treasury -= cost_per;
        }
        self.message = None;
    }

    fn update_line_drag_message(&mut self) {
        if let Some(ref drag) = self.line_drag {
            let tool = drag.tool;
            let placeable = drag.path.iter()
                .filter(|&&(x, y)| x < self.map.width && y < self.map.height
                    && tool.can_place(self.map.get(x, y)))
                .count();
            let blocked = drag.path.len() - placeable;
            let cost = placeable as i64 * tool.cost();
            let name = tool.label();
            self.message = Some(if blocked > 0 {
                format!("{}: {} tiles  ${} ({} blocked)", name, placeable, cost, blocked)
            } else {
                format!("{}: {} tiles  ${}", name, placeable, cost)
            });
        }
    }

    /// Returns a reference to the cached line-drag preview path — zero allocation, called each frame.
    pub fn line_preview(&self) -> &[(usize, usize)] {
        self.line_drag.as_ref().map(|d| d.path.as_slice()).unwrap_or(&[])
    }

    fn commit_rect_drag(&mut self) {
        let drag = match self.rect_drag.take() { Some(d) => d, None => return };
        let target = match drag.tool.target_tile() { Some(t) => t, None => return };
        let cost_per = drag.tool.cost();
        for &(x, y) in &drag.tiles_cache {
            if x >= self.map.width || y >= self.map.height { continue; }
            if !drag.tool.can_place(self.map.get(x, y)) { continue; }
            if self.sim.treasury < cost_per {
                self.message = Some("Insufficient funds!".to_string());
                return;
            }
            self.map.set(x, y, target);
            self.sim.treasury -= cost_per;
        }
        self.message = None;
    }

    fn update_rect_drag_message(&mut self) {
        if let Some(ref drag) = self.rect_drag {
            let tool = drag.tool;
            let placeable = drag.tiles_cache.iter()
                .filter(|&&(x, y)| x < self.map.width && y < self.map.height
                    && tool.can_place(self.map.get(x, y)))
                .count();
            let blocked = drag.tiles_cache.len() - placeable;
            let cost = placeable as i64 * tool.cost();
            let (w, h) = (drag.width(), drag.height());
            self.message = Some(if blocked > 0 {
                format!("{}: {}×{} = {} tiles  ${} ({} blocked)", tool.label(), w, h, placeable, cost, blocked)
            } else {
                format!("{}: {}×{} = {} tiles  ${}", tool.label(), w, h, placeable, cost)
            });
        }
    }

    /// Returns a reference to the cached rect-drag preview tiles — zero allocation, called each frame.
    pub fn rect_preview(&self) -> &[(usize, usize)] {
        self.rect_drag.as_ref().map(|d| d.tiles_cache.as_slice()).unwrap_or(&[])
    }

    fn handle_click(&mut self, col: u16, row: u16, is_click: bool) {
        // Pause button
        if self.ui_areas.pause_btn.contains(col, row) {
            if is_click {
                self.paused = !self.paused;
            }
            return;
        }

        // Toolbar
        let tool = self
            .ui_areas
            .toolbar_buttons
            .iter()
            .find(|(_, area)| area.contains(col, row))
            .map(|(t, _)| *t);
        if let Some(t) = tool {
            self.current_tool = t;
            return;
        }

        // Map area
        if self.ui_areas.map.contains(col, row) {
            let sx = col - self.ui_areas.map.x;
            let sy = row - self.ui_areas.map.y;
            let (mx, my) = self.camera.screen_to_map(sx, sy);
            let mx = mx.min(self.map.width.saturating_sub(1));
            let my = my.min(self.map.height.saturating_sub(1));
            self.camera.cursor_x = mx;
            self.camera.cursor_y = my;

            if self.current_tool != Tool::Inspect {
                self.place_current_tool();
            }
            return;
        }

        // Minimap — pan proportionally
        if self.ui_areas.minimap.contains(col, row) {
            let mm = self.ui_areas.minimap;
            let rx = (col - mm.x) as f32 / mm.width as f32;
            let ry = (row - mm.y) as f32 / mm.height as f32;
            let new_ox = (rx * self.map.width as f32) as i32 - self.camera.view_w as i32 / 2;
            let new_oy = (ry * self.map.height as f32) as i32 - self.camera.view_h as i32 / 2;
            let (mw, mh) = (self.map.width, self.map.height);
            self.camera.offset_x = new_ox.clamp(0, (mw as i32 - self.camera.view_w as i32).max(0));
            self.camera.offset_y = new_oy.clamp(0, (mh as i32 - self.camera.view_h as i32).max(0));
        }
    }

    pub fn place_current_tool(&mut self) {
        let x = self.camera.cursor_x;
        let y = self.camera.cursor_y;
        let (fw, fh) = self.current_tool.footprint();

        // For multi-tile buildings: derive centered anchor, then place
        if fw > 1 || fh > 1 {
            let ax = x.saturating_sub(fw / 2).min(self.map.width.saturating_sub(fw));
            let ay = y.saturating_sub(fh / 2).min(self.map.height.saturating_sub(fh));

            for dy in 0..fh {
                for dx in 0..fw {
                    if !self.current_tool.can_place(self.map.get(ax + dx, ay + dy)) {
                        return;
                    }
                }
            }
            let cost = self.current_tool.cost();
            if self.sim.treasury < cost {
                self.message = Some("Insufficient funds!".to_string());
                return;
            }
            let new_tile = match self.current_tool.target_tile() { Some(t) => t, None => return };
            for dy in 0..fh {
                for dx in 0..fw {
                    self.map.set(ax + dx, ay + dy, new_tile);
                }
            }
            self.sim.treasury -= cost;
            self.message = None;
            return;
        }

        // Bounds check for single-tile tools
        if x >= self.map.width || y >= self.map.height {
            return;
        }

        // Single-tile placement (original logic, preserved for road/powerline combos)
        let tile = self.map.get(x, y);
        if !self.current_tool.can_place(tile) {
            return;
        }
        let cost = self.current_tool.cost();
        if self.sim.treasury < cost {
            self.message = Some("Insufficient funds!".to_string());
            return;
        }
        let new_tile = match (self.current_tool, tile) {
            (Tool::Road, Tile::PowerLine) | (Tool::PowerLine, Tile::Road) => Tile::RoadPowerLine,
            _ => match self.current_tool.target_tile() { Some(t) => t, None => return },
        };
        self.map.set(x, y, new_tile);
        self.sim.treasury -= cost;
        self.message = None;
    }
}
