use crate::{
    app::{input::Action, ClickArea},
    core::{
        engine::EngineCommand,
        map::{gen, Map, TerrainTile},
    },
};

use super::{AppContext, InGameScreen, Screen, ScreenTransition};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NewCityField {
    CityName,
    GenerateNameBtn,
    SeedInput,
    WaterSlider,
    TreesSlider,
    RegenerateBtn,
    StartBtn,
    BackBtn,
}

impl NewCityField {
    const ALL: [NewCityField; 8] = [
        NewCityField::CityName,
        NewCityField::GenerateNameBtn,
        NewCityField::SeedInput,
        NewCityField::WaterSlider,
        NewCityField::TreesSlider,
        NewCityField::RegenerateBtn,
        NewCityField::StartBtn,
        NewCityField::BackBtn,
    ];

    pub fn next(self) -> Self {
        crate::ui::runtime::cycle_next(self, &Self::ALL)
    }

    pub fn prev(self) -> Self {
        crate::ui::runtime::cycle_prev(self, &Self::ALL)
    }
}

/// Terrain brushes available in the map generator for free tile painting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerrainBrush {
    Water,
    Land,
    Trees,
}

impl TerrainBrush {
    pub fn to_terrain(self) -> TerrainTile {
        match self {
            TerrainBrush::Water => TerrainTile::Water,
            TerrainBrush::Land => TerrainTile::Grass,
            TerrainBrush::Trees => TerrainTile::Trees,
        }
    }

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            TerrainBrush::Water => "Water",
            TerrainBrush::Land => "Land",
            TerrainBrush::Trees => "Trees",
        }
    }
}

pub struct NewCityState {
    pub seed: u64,
    pub preview_map: Map,
    pub focused_field: NewCityField,
    pub city_name: String,
    pub seed_input: String,
    pub water_pct: usize,
    pub trees_pct: usize,
    pub field_areas: [ClickArea; 8],
    /// Whether an LLM city name generation request is in flight.
    pub llm_name_pending: bool,
    /// Currently selected terrain brush for free painting on the map preview.
    pub terrain_brush: Option<TerrainBrush>,
    /// Click areas for the [None] [Water] [Land] [Trees] brush buttons (4 items).
    pub brush_areas: [ClickArea; 4],
    /// Bounds of the inner map preview widget (set by the renderer each frame).
    pub inner_map_area: ClickArea,
    /// When true, arrow keys move the map cursor instead of navigating fields.
    pub map_cursor_active: bool,
    /// Current cursor position in map tile coordinates.
    pub cursor_x: usize,
    pub cursor_y: usize,
}

impl NewCityState {
    pub fn pack_seed(water_pct: u8, trees_pct: u8, raw_seed: u64) -> u64 {
        let w = (water_pct.min(100) as u64) << 56;
        let t = (trees_pct.min(100) as u64) << 48;
        let s = raw_seed & 0x0000_FFFF_FFFF_FFFF;
        w | t | s
    }

    pub fn unpack_seed(packed: u64) -> (u8, u8, u64) {
        let w = ((packed >> 56) & 0xFF) as u8;
        let t = ((packed >> 48) & 0xFF) as u8;
        let s = packed & 0x0000_FFFF_FFFF_FFFF;
        (w.min(100), t.min(100), s)
    }

    pub fn new() -> Self {
        let raw_seed = rand::random::<u64>() & 0x0000_FFFF_FFFF_FFFF;
        let params = gen::GenParams {
            seed: raw_seed,
            ..Default::default()
        };
        let preview_map = gen::generate(&params);
        let seed = Self::pack_seed(params.water_pct, params.trees_pct, raw_seed);

        Self {
            seed,
            preview_map,
            focused_field: NewCityField::CityName,
            city_name: String::new(),
            seed_input: format!("{seed:016X}"),
            water_pct: params.water_pct as usize,
            trees_pct: params.trees_pct as usize,
            field_areas: [ClickArea::default(); 8],
            llm_name_pending: false,
            terrain_brush: None,
            brush_areas: [ClickArea::default(); 4],
            inner_map_area: ClickArea::default(),
            map_cursor_active: false,
            cursor_x: 64,
            cursor_y: 64,
        }
    }

    /// Cycle to the next brush: None → Water → Land → Trees → None.
    pub fn cycle_brush(&mut self) {
        self.terrain_brush = match self.terrain_brush {
            None => Some(TerrainBrush::Water),
            Some(TerrainBrush::Water) => Some(TerrainBrush::Land),
            Some(TerrainBrush::Land) => Some(TerrainBrush::Trees),
            Some(TerrainBrush::Trees) => None,
        };
    }

    /// Paint the terrain at the cursor position with the active brush (if any).
    pub fn paint_at_cursor(&mut self) {
        if let Some(brush) = self.terrain_brush {
            let mx = self.cursor_x.min(self.preview_map.width.saturating_sub(1));
            let my = self.cursor_y.min(self.preview_map.height.saturating_sub(1));
            self.preview_map.set_terrain(mx, my, brush.to_terrain());
        }
    }

    /// Move the map cursor by (dx, dy), clamped to map bounds.
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let w = self.preview_map.width;
        let h = self.preview_map.height;
        self.cursor_x = (self.cursor_x as i32 + dx).clamp(0, w as i32 - 1) as usize;
        self.cursor_y = (self.cursor_y as i32 + dy).clamp(0, h as i32 - 1) as usize;
    }

    pub fn regenerate(&mut self) {
        let raw_seed = rand::random::<u64>() & 0x0000_FFFF_FFFF_FFFF;
        let w = self.water_pct as u8;
        let t = self.trees_pct as u8;
        self.seed = Self::pack_seed(w, t, raw_seed);
        self.seed_input = format!("{:016X}", self.seed);
        self.rebuild_map();
    }

    pub fn apply_seed_input(&mut self) {
        let text = self.seed_input.clone();
        let raw = text
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let parsed = u64::from_str_radix(raw, 16).or_else(|_| text.trim().parse::<u64>());
        if let Ok(new_seed) = parsed {
            self.seed = new_seed;
            let (w, t, _) = Self::unpack_seed(self.seed);
            self.water_pct = w as usize;
            self.trees_pct = t as usize;
        }
        self.seed_input = format!("{:016X}", self.seed);
        self.rebuild_map();
    }

    pub fn sync_sliders_to_seed(&mut self) {
        let w = self.water_pct as u8;
        let t = self.trees_pct as u8;
        let (_, _, raw) = Self::unpack_seed(self.seed);
        self.seed = Self::pack_seed(w, t, raw);
        self.seed_input = format!("{:016X}", self.seed);
    }

    pub fn rebuild_map(&mut self) {
        let (w, t, raw_seed) = Self::unpack_seed(self.seed);
        let params = gen::GenParams {
            water_pct: w,
            trees_pct: t,
            seed: raw_seed,
            ..Default::default()
        };
        self.preview_map = gen::generate(&params);
    }
}

pub struct NewCityScreen {
    pub state: NewCityState,
}

impl Screen for NewCityScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_tick(&mut self, context: AppContext) -> Option<ScreenTransition> {
        if self.state.llm_name_pending {
            if let Some(resp) = context.textgen.poll() {
                if resp.task_tag == crate::textgen::types::LlmTaskTag::CityName {
                    // Take only the first line, strip trailing punctuation and
                    // anything after a comma (models often append ", State").
                    let raw = resp.text.trim().lines().next().unwrap_or("").trim();
                    let before_comma = raw.split(',').next().unwrap_or(raw).trim();
                    let name: String = before_comma
                        .trim_end_matches(|c: char| {
                            c == '.' || c == ',' || c == ';' || c == ':' || c == '"' || c == '\''
                        })
                        .trim()
                        .chars()
                        .take(30)
                        .collect();
                    if !name.is_empty() {
                        self.state.city_name = name;
                    }
                    self.state.llm_name_pending = false;
                }
            }
        }
        None
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition> {
        // ── Map cursor mode: arrow keys move cursor, Enter paints, Esc exits ──
        if self.state.map_cursor_active {
            match action {
                Action::MoveCursor(dx, dy) => {
                    self.state.move_cursor(dx, dy);
                    self.state.paint_at_cursor();
                    return None;
                }
                Action::MenuSelect => {
                    self.state.paint_at_cursor();
                    return None;
                }
                Action::MenuBack => {
                    self.state.map_cursor_active = false;
                    return None;
                }
                Action::CharInput('\t') => {
                    self.state.cycle_brush();
                    return None;
                }
                _ => {}
            }
        }

        match action {
            Action::MenuBack => Some(ScreenTransition::Pop),
            Action::MoveCursor(dx, dy) => {
                if dx != 0 && self.adjust_focused_slider(dx) {
                    return None;
                }
                if dy != 0 {
                    if self.state.focused_field == NewCityField::SeedInput {
                        self.state.apply_seed_input();
                    }
                    self.state.focused_field = if dy > 0 {
                        self.state.focused_field.next()
                    } else {
                        self.state.focused_field.prev()
                    };
                }
                None
            }
            Action::MouseClick { col, row } => {
                // Brush button clicks (4 buttons: None/Water/Land/Trees)
                let brushes: [Option<TerrainBrush>; 4] = [
                    None,
                    Some(TerrainBrush::Water),
                    Some(TerrainBrush::Land),
                    Some(TerrainBrush::Trees),
                ];
                for (i, area) in self.state.brush_areas.iter().enumerate() {
                    if area.contains(col, row) {
                        self.state.terrain_brush = brushes[i];
                        return None;
                    }
                }
                // Map preview click — paint if brush is selected; enter map mode otherwise
                if let Some((tx, ty)) =
                    map_click_to_tile(col, row, self.state.inner_map_area, &self.state.preview_map)
                {
                    if let Some(brush) = self.state.terrain_brush {
                        self.state
                            .preview_map
                            .set_terrain(tx, ty, brush.to_terrain());
                    } else {
                        // Clicking the map without a brush activates map cursor mode at that tile
                        self.state.cursor_x = tx;
                        self.state.cursor_y = ty;
                        self.state.map_cursor_active = true;
                    }
                    return None;
                }
                for (idx, area) in self.state.field_areas.iter().enumerate() {
                    if area.contains(col, row) {
                        let field = NewCityField::ALL[idx];
                        self.state.focused_field = field;
                        return match field {
                            NewCityField::GenerateNameBtn => {
                                context
                                    .textgen
                                    .request(crate::textgen::types::LlmTask::GenerateCityName);
                                self.state.llm_name_pending = true;
                                None
                            }
                            NewCityField::RegenerateBtn => {
                                self.state.regenerate();
                                None
                            }
                            NewCityField::StartBtn => self.start_city(context),
                            NewCityField::BackBtn => Some(ScreenTransition::Pop),
                            NewCityField::WaterSlider => {
                                let relative =
                                    col.saturating_sub(area.x).min(area.width.saturating_sub(1));
                                self.state.water_pct =
                                    ((relative as u32 * 100) / area.width.max(1) as u32) as usize;
                                self.state.sync_sliders_to_seed();
                                self.state.rebuild_map();
                                None
                            }
                            NewCityField::TreesSlider => {
                                let relative =
                                    col.saturating_sub(area.x).min(area.width.saturating_sub(1));
                                self.state.trees_pct =
                                    ((relative as u32 * 100) / area.width.max(1) as u32) as usize;
                                self.state.sync_sliders_to_seed();
                                self.state.rebuild_map();
                                None
                            }
                            _ => None,
                        };
                    }
                }
                None
            }
            Action::MouseDrag { col, row } => {
                if let Some((tx, ty)) =
                    map_click_to_tile(col, row, self.state.inner_map_area, &self.state.preview_map)
                {
                    if let Some(brush) = self.state.terrain_brush {
                        self.state
                            .preview_map
                            .set_terrain(tx, ty, brush.to_terrain());
                    }
                }
                None
            }
            Action::DeleteChar => {
                match self.state.focused_field {
                    NewCityField::CityName => {
                        self.state.city_name.pop();
                    }
                    NewCityField::SeedInput => {
                        self.state.seed_input.pop();
                        self.state.apply_seed_input();
                    }
                    _ => {}
                }
                None
            }
            Action::CharInput(c) => {
                match c {
                    '\t' => {
                        // Tab cycles terrain brush regardless of focused field
                        self.state.cycle_brush();
                    }
                    'm' | 'M' => {
                        // M toggles map cursor mode (only useful when a brush is selected)
                        self.state.map_cursor_active = !self.state.map_cursor_active;
                    }
                    _ => match self.state.focused_field {
                        NewCityField::CityName => {
                            if !c.is_control() {
                                self.state.city_name.push(c);
                            }
                        }
                        NewCityField::SeedInput => {
                            if c.is_ascii_hexdigit() {
                                self.state.seed_input.push(c.to_ascii_uppercase());
                                self.state.apply_seed_input();
                            }
                        }
                        _ => {}
                    },
                }
                None
            }
            Action::MenuSelect => match self.state.focused_field {
                NewCityField::GenerateNameBtn => {
                    context
                        .textgen
                        .request(crate::textgen::types::LlmTask::GenerateCityName);
                    self.state.llm_name_pending = true;
                    None
                }
                NewCityField::SeedInput => {
                    self.state.apply_seed_input();
                    None
                }
                NewCityField::RegenerateBtn => {
                    self.state.regenerate();
                    None
                }
                NewCityField::StartBtn => self.start_city(context),
                NewCityField::BackBtn => Some(ScreenTransition::Pop),
                _ => None,
            },
            _ => None,
        }
    }

    fn build_view(&self, _context: AppContext<'_>) -> crate::ui::view::ScreenView {
        crate::ui::view::ScreenView::NewCity(crate::ui::view::NewCityViewModel {
            preview_map: self.state.preview_map.clone(),
            focused_field: self.state.focused_field,
            city_name: self.state.city_name.clone(),
            seed_text: self.state.seed_input.clone(),
            water_pct: self.state.water_pct,
            trees_pct: self.state.trees_pct,
            terrain_brush: self.state.terrain_brush,
            cursor: (self.state.cursor_x, self.state.cursor_y),
            map_cursor_active: self.state.map_cursor_active,
            llm_name_pending: self.state.llm_name_pending,
        })
    }
}

impl NewCityScreen {
    fn adjust_focused_slider(&mut self, dx: i32) -> bool {
        let delta = dx.signum();
        if delta == 0 {
            return false;
        }

        let updated = match self.state.focused_field {
            NewCityField::WaterSlider => {
                self.state.water_pct = adjust_slider_value(self.state.water_pct, delta);
                true
            }
            NewCityField::TreesSlider => {
                self.state.trees_pct = adjust_slider_value(self.state.trees_pct, delta);
                true
            }
            _ => false,
        };

        if updated {
            self.state.sync_sliders_to_seed();
            self.state.rebuild_map();
        }

        updated
    }

    fn start_city(&mut self, context: AppContext) -> Option<ScreenTransition> {
        let name = if self.state.city_name.trim().is_empty() {
            "New City".to_string()
        } else {
            self.state.city_name.clone()
        };
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::ReplaceState {
                map: self.state.preview_map.clone(),
                sim: crate::core::sim::SimState::default(),
            });
            let _ = tx.send(EngineCommand::SetCityName(name));
        }
        if self.state.city_name.trim().is_empty() {
            None
        } else {
            Some(ScreenTransition::Replace(Box::new(InGameScreen::new())))
        }
    }
}

/// Translates a terminal (col, row) click within the map preview to a map tile (x, y).
/// Returns `None` if the click is outside the rendered map area.
/// Mirrors the aspect-fitting logic from `MapPreview::render`.
fn map_click_to_tile(col: u16, row: u16, inner: ClickArea, map: &Map) -> Option<(usize, usize)> {
    if inner.width == 0 || inner.height == 0 {
        return None;
    }
    let mw = map.width as f32;
    let mh = map.height as f32;
    let map_aspect = (2.0 * mw) / mh;

    let (rw, rh) = if inner.width as f32 / inner.height as f32 > map_aspect {
        let h = inner.height as f32;
        let w = h * map_aspect;
        ((w as u16 / 2 * 2).max(2), (h as u16).max(1))
    } else {
        let w = inner.width as f32;
        let h = w / map_aspect;
        ((w as u16 / 2 * 2).max(2), (h as u16).max(1))
    };

    let rx = inner.x + (inner.width.saturating_sub(rw)) / 2;
    let ry = inner.y + (inner.height.saturating_sub(rh)) / 2;

    if col < rx || col >= rx + rw || row < ry || row >= ry + rh {
        return None;
    }

    // Visual tile column/row (each visual tile is 2 terminal chars wide, 1 tall).
    let v_col = ((col - rx) / 2) as usize;
    let v_row = (row - ry) as usize;
    let num_v_tiles_x = (rw / 2) as usize;
    let num_v_tiles_y = rh as usize;

    // Mirror the endpoint-interpolation used in MapPreview::render exactly,
    // so the painted tile is always the same tile that visual cell displays.
    let tile_x = if num_v_tiles_x <= 1 {
        0
    } else {
        (v_col * (map.width - 1)) / (num_v_tiles_x - 1)
    };
    let tile_y = if num_v_tiles_y <= 1 {
        0
    } else {
        (v_row * (map.height - 1)) / (num_v_tiles_y - 1)
    };

    if tile_x < map.width && tile_y < map.height {
        Some((tile_x, tile_y))
    } else {
        None
    }
}

fn adjust_slider_value(current: usize, delta: i32) -> usize {
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs() as usize)
    } else {
        current.saturating_add(delta as usize).min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    fn test_fixtures() -> (
        Arc<RwLock<crate::core::engine::SimulationEngine>>,
        crate::textgen::TextGenService,
    ) {
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            crate::core::sim::SimState::default(),
        )));
        let textgen =
            crate::textgen::TextGenService::start(std::path::PathBuf::from("/nonexistent"));
        (engine, textgen)
    }

    #[test]
    fn test_new_city_screen_requires_name() {
        let mut screen = NewCityScreen {
            state: NewCityState::new(),
        };
        let (engine, textgen) = test_fixtures();
        let cmd_tx = None;

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &textgen,
        };

        screen.state.focused_field = NewCityField::StartBtn;
        let transition = screen.on_action(Action::MenuSelect, context);
        assert!(transition.is_none());
    }

    #[test]
    fn test_new_city_screen_fields_and_start_flow() {
        let mut screen = NewCityScreen {
            state: NewCityState::new(),
        };
        let (engine, textgen) = test_fixtures();
        let cmd_tx = None;

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &textgen,
        };

        screen.state.city_name = "Test City".to_string();
        screen.state.focused_field = NewCityField::SeedInput;
        screen.state.seed_input.clear();

        screen.on_action(Action::CharInput('1'), context);
        assert_eq!(screen.state.seed_input, "0000000000000001");

        screen.state.apply_seed_input();
        assert_eq!(screen.state.seed, 1);

        let context_action = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &textgen,
        };
        let transition = screen.on_action(Action::MenuSelect, context_action);
        assert!(transition.is_none());

        screen.state.focused_field = NewCityField::StartBtn;
        let transition_start = screen.on_action(
            Action::MenuSelect,
            AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
                textgen: &textgen,
            },
        );
        assert!(transition_start.is_some());
    }

    #[test]
    fn left_right_keys_adjust_focused_sliders() {
        let mut screen = NewCityScreen {
            state: NewCityState::new(),
        };
        let (engine, textgen) = test_fixtures();
        let cmd_tx = None;

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &textgen,
        };

        screen.state.focused_field = NewCityField::WaterSlider;
        let initial_water = screen.state.water_pct;
        screen.on_action(Action::MoveCursor(1, 0), context);
        assert_eq!(screen.state.water_pct, (initial_water + 1).min(100));

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &textgen,
        };
        screen.state.focused_field = NewCityField::TreesSlider;
        let initial_trees = screen.state.trees_pct;
        screen.on_action(Action::MoveCursor(-1, 0), context);
        assert_eq!(screen.state.trees_pct, initial_trees.saturating_sub(1));
    }

    #[test]
    fn plus_minus_no_longer_adjust_sliders() {
        let mut screen = NewCityScreen {
            state: NewCityState::new(),
        };
        let (engine, textgen) = test_fixtures();
        let cmd_tx = None;

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &textgen,
        };

        screen.state.focused_field = NewCityField::WaterSlider;
        let initial_water = screen.state.water_pct;
        screen.on_action(Action::CharInput('+'), context);
        assert_eq!(screen.state.water_pct, initial_water);

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &textgen,
        };
        screen.on_action(Action::CharInput('-'), context);
        assert_eq!(screen.state.water_pct, initial_water);
    }

    #[test]
    fn generate_name_btn_is_in_field_list() {
        assert_eq!(NewCityField::ALL.len(), 8);
        assert_eq!(NewCityField::ALL[1], NewCityField::GenerateNameBtn);
    }

    #[test]
    fn generate_name_btn_navigation() {
        let btn = NewCityField::CityName.next();
        assert_eq!(btn, NewCityField::GenerateNameBtn);
        let next = btn.next();
        assert_eq!(next, NewCityField::SeedInput);
    }

    #[test]
    fn generate_name_always_sets_pending() {
        let mut screen = NewCityScreen {
            state: NewCityState::new(),
        };
        let (engine, textgen) = test_fixtures();
        let cmd_tx = None;

        screen.state.focused_field = NewCityField::GenerateNameBtn;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &textgen,
        };
        let transition = screen.on_action(Action::MenuSelect, context);
        assert!(transition.is_none());
        // With static backend, pending should be true (request is always sent).
        assert!(screen.state.llm_name_pending);
    }
}
