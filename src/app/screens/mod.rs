use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::sync::mpsc::Sender;
use crate::core::engine::{EngineCommand, SimulationEngine};
use crate::core::sim::SimState;
use crate::app::input::Action;
use crate::ui::theme::OverlayMode;
use ratatui::Frame;
use crate::app::{StartState, LoadCityState, NewCityState, Camera, Tool, LineDrag, RectDrag, UiAreas, FloatingWindow, WindowDrag, save};
use tui_menu::{MenuEvent, MenuItem, MenuState};

/// Ticks between auto-saves (50 ticks/month × 12 months × 6 months ≈ every 6 in-game months).
const AUTO_SAVE_INTERVAL: u32 = 50 * 12 * 6;

// ── Menu actions ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum MenuAction {
    NewCity,
    SaveCity,
    Quit,
    SpeedPause,
    SpeedSlow,
    SpeedNormal,
    SpeedFast,
    DisasterFire,
    DisasterFlood,
    DisasterTornado,
    OpenBudget,
    OverlayNone,
    OverlayPower,
    OverlayPollution,
    OverlayLandValue,
    OverlayCrime,
    OverlayFireRisk,
}

/// Build a fresh MenuState reflecting the current game state.
/// Called each time the user opens the menu so labels stay accurate.
pub fn build_menu_state(
    sim: &SimState,
    paused: bool,
    ticks_per_month: u32,
    overlay: OverlayMode,
) -> MenuState<MenuAction> {
    let fire_label    = if sim.disasters.fire_enabled    { "Fire    [ON] " } else { "Fire    [OFF]" };
    let flood_label   = if sim.disasters.flood_enabled   { "Flood   [ON] " } else { "Flood   [OFF]" };
    let tornado_label = if sim.disasters.tornado_enabled { "Tornado [ON] " } else { "Tornado [OFF]" };
    let pause_label   = if paused { "Resume" } else { "Pause" };
    let speed_tag     = match ticks_per_month { 0..=30 => "Fast", 31..=70 => "Normal", _ => "Slow" };

    // Mark the active overlay with a bullet so the user knows which is on.
    let mark = |mode: OverlayMode| if overlay == mode { "● " } else { "  " };

    MenuState::new(vec![
        MenuItem::group("System", vec![
            MenuItem::item("New City",    MenuAction::NewCity),
            MenuItem::item("Save City",   MenuAction::SaveCity),
            MenuItem::item("Quit",        MenuAction::Quit),
        ]),
        MenuItem::group(format!("Speed [{speed_tag}]"), vec![
            MenuItem::item(pause_label,  MenuAction::SpeedPause),
            MenuItem::item("Slow",       MenuAction::SpeedSlow),
            MenuItem::item("Normal",     MenuAction::SpeedNormal),
            MenuItem::item("Fast",       MenuAction::SpeedFast),
        ]),
        MenuItem::group("Disasters", vec![
            MenuItem::item(fire_label,    MenuAction::DisasterFire),
            MenuItem::item(flood_label,   MenuAction::DisasterFlood),
            MenuItem::item(tornado_label, MenuAction::DisasterTornado),
        ]),
        MenuItem::group("Windows", vec![
            MenuItem::item("Budget & Taxes",                         MenuAction::OpenBudget),
            MenuItem::item(format!("{}Overlay: Off",        mark(OverlayMode::None)),       MenuAction::OverlayNone),
            MenuItem::item(format!("{}Overlay: Power",      mark(OverlayMode::Power)),      MenuAction::OverlayPower),
            MenuItem::item(format!("{}Overlay: Pollution",  mark(OverlayMode::Pollution)),  MenuAction::OverlayPollution),
            MenuItem::item(format!("{}Overlay: Land Value", mark(OverlayMode::LandValue)),  MenuAction::OverlayLandValue),
            MenuItem::item(format!("{}Overlay: Crime",      mark(OverlayMode::Crime)),      MenuAction::OverlayCrime),
            MenuItem::item(format!("{}Overlay: Fire Risk",  mark(OverlayMode::FireRisk)),   MenuAction::OverlayFireRisk),
        ]),
    ])
}

pub enum ScreenTransition {
    Push(Box<dyn Screen>),
    Pop,
    Replace(Box<dyn Screen>),
    Quit,
}

pub struct AppContext<'a> {
    pub engine: &'a Arc<RwLock<SimulationEngine>>,
    pub cmd_tx: &'a Option<Sender<EngineCommand>>,
    pub running: &'a mut bool,
}

pub trait Screen {
    fn on_event(&mut self, _event: &crossterm::event::Event, _context: AppContext) -> Option<ScreenTransition> { None }
    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition>;
    fn on_tick(&mut self, _context: AppContext) {}
    fn render(&mut self, frame: &mut Frame, context: AppContext);
}

// ── Start Screen ─────────────────────────────────────────────────────────────

pub struct StartScreen {
    pub state: StartState,
}

impl StartScreen {
    pub fn new() -> Self {
        Self { state: StartState::default() }
    }
}

impl Screen for StartScreen {
    fn on_action(&mut self, action: Action, _context: AppContext) -> Option<ScreenTransition> {
        const N: usize = 3;
        match action {
            Action::Quit | Action::CharInput('q') => {
                return Some(ScreenTransition::Quit);
            }
            Action::MoveCursor(_, dy) => {
                if dy > 0 {
                    self.state.selected = (self.state.selected + 1) % N;
                } else if dy < 0 {
                    self.state.selected = self.state.selected.checked_sub(1).unwrap_or(N - 1);
                }
            }
            Action::MouseClick { col, row } => {
                for (i, area) in self.state.menu_areas.iter().enumerate() {
                    if area.contains(col, row) {
                        self.state.selected = i;
                        return match self.state.selected {
                            0 => {
                                let saves = save::list_saves();
                                Some(ScreenTransition::Push(Box::new(LoadCityScreen {
                                    state: LoadCityState { saves, selected: 0 }
                                })))
                            }
                            1 => {
                                Some(ScreenTransition::Push(Box::new(NewCityScreen {
                                    state: NewCityState::new()
                                })))
                            }
                            2 => {
                                Some(ScreenTransition::Quit)
                            }
                            _ => None
                        };
                    }
                }
            }
            Action::MenuSelect => {
                match self.state.selected {
                    0 => {
                        let saves = save::list_saves();
                        return Some(ScreenTransition::Push(Box::new(LoadCityScreen {
                            state: LoadCityState { saves, selected: 0 }
                        })));
                    }
                    1 => {
                        return Some(ScreenTransition::Push(Box::new(NewCityScreen {
                            state: NewCityState::new()
                        })));
                    }
                    2 => {
                        return Some(ScreenTransition::Quit);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        None
    }

    fn render(&mut self, frame: &mut Frame, _context: AppContext) {
        let area = frame.area();
        crate::ui::screens::start::render_start(frame, area, &mut self.state);
    }
}

// ── New City Screen ──────────────────────────────────────────────────────────

pub struct NewCityScreen {
    pub state: NewCityState,
}

impl Screen for NewCityScreen {
    fn on_event(&mut self, event: &crossterm::event::Event, context: AppContext) -> Option<ScreenTransition> {
        use rat_widget::event::{TextOutcome, SliderOutcome, ButtonOutcome};
        
        let focus_city = self.state.focused_field == crate::app::NewCityField::CityName;
        let focus_seed = self.state.focused_field == crate::app::NewCityField::SeedInput;
        let focus_water = self.state.focused_field == crate::app::NewCityField::WaterSlider;
        let focus_trees = self.state.focused_field == crate::app::NewCityField::TreesSlider;
        let focus_regen = self.state.focused_field == crate::app::NewCityField::RegenerateBtn;
        let focus_start = self.state.focused_field == crate::app::NewCityField::StartBtn;
        let focus_back = self.state.focused_field == crate::app::NewCityField::BackBtn;

        // Route events to rat-widgets
        let _out_city = rat_widget::text_input::handle_events(&mut self.state.city_name, focus_city, event);
        let out_seed = rat_widget::text_input::handle_events(&mut self.state.seed_input, focus_seed, event);
        let out_water = rat_widget::slider::handle_events(&mut self.state.water_slider, focus_water, event);
        let out_trees = rat_widget::slider::handle_events(&mut self.state.trees_slider, focus_trees, event);
        let out_regen = rat_widget::button::handle_events(&mut self.state.regen_btn, focus_regen, event);
        let out_start = rat_widget::button::handle_events(&mut self.state.start_btn, focus_start, event);
        let out_back = rat_widget::button::handle_events(&mut self.state.back_btn, focus_back, event);

        // Update focus on mouse click (rat_widget state changes focus internally if clicked)
        // Wait, rat_widget button might just return Outcome::Changed, but it doesn't change our focused_field.
        // We can look at `is_focused()` if rat-focus is used, but we are using `NewCityField`.
        // To properly support mouse focusing, we could check if the outcome implies interaction.
        // Actually, if a mouse click happens, we should probably update `focused_field` but it's tricky to map.
        // Instead, let's let `rat_widget` do what it can, and we handle explicit actions.

        if out_seed == TextOutcome::TextChanged {
            self.state.apply_seed_input();
        }
        
        if out_water == SliderOutcome::Changed || out_trees == SliderOutcome::Changed {
            self.state.sync_sliders_to_seed();
            self.state.rebuild_map();
        }

        if out_regen == ButtonOutcome::Pressed {
            self.state.regenerate();
        }
        
        if out_start == ButtonOutcome::Pressed {
            if let Some(tx) = context.cmd_tx {
                let _ = tx.send(crate::core::engine::EngineCommand::ReplaceState {
                    map: self.state.preview_map.clone(),
                    sim: crate::core::sim::SimState::default(),
                });
                let name = if self.state.city_name.text().is_empty() { "New City".to_string() } else { self.state.city_name.text().to_string() };
                let _ = tx.send(crate::core::engine::EngineCommand::SetCityName(name));
            }
            return Some(ScreenTransition::Replace(Box::new(InGameScreen::new())));
        }
        
        if out_back == ButtonOutcome::Pressed {
            return Some(ScreenTransition::Pop);
        }

        None
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition> {
        match action {
            Action::MenuBack => {
                return Some(ScreenTransition::Pop);
            }
            Action::MoveCursor(_dx, dy) => {
                if dy != 0 {
                    if self.state.focused_field == crate::app::NewCityField::SeedInput {
                        self.state.apply_seed_input();
                    }
                    self.state.focused_field = if dy > 0 { self.state.focused_field.next() } else { self.state.focused_field.prev() };
                }
            }
            Action::MenuSelect => {
                match self.state.focused_field {
                    crate::app::NewCityField::SeedInput => self.state.apply_seed_input(),
                    crate::app::NewCityField::RegenerateBtn => self.state.regenerate(),
                    crate::app::NewCityField::StartBtn => {
                        if let Some(tx) = context.cmd_tx {
                            let _ = tx.send(crate::core::engine::EngineCommand::ReplaceState {
                                map: self.state.preview_map.clone(),
                                sim: crate::core::sim::SimState::default(),
                            });
                            let name = if self.state.city_name.text().is_empty() { "New City".to_string() } else { self.state.city_name.text().to_string() };
                            let _ = tx.send(crate::core::engine::EngineCommand::SetCityName(name));
                        }
                        return Some(ScreenTransition::Replace(Box::new(InGameScreen::new())));
                    }
                    crate::app::NewCityField::BackBtn => return Some(ScreenTransition::Pop),
                    _ => {}
                }
            }
            // Mouse clicks are handled by rat_widget, but we need to update our focused_field
            Action::MouseClick { col, row } => {
                // Approximate focus mapping based on y-coordinates (since we render them linearly)
                // We'll rely on the rat_widget components to process the click logic itself via on_event.
                // We just need a rough way to update keyboard focus if they clicked.
                if col >= 30 { // Right panel
                    if row == 2 || row == 3 { self.state.focused_field = crate::app::NewCityField::CityName; }
                    else if row == 5 || row == 6 { self.state.focused_field = crate::app::NewCityField::SeedInput; }
                    else if row == 8 || row == 9 { self.state.focused_field = crate::app::NewCityField::WaterSlider; }
                    else if row == 11 || row == 12 { self.state.focused_field = crate::app::NewCityField::TreesSlider; }
                    else if row == 15 { self.state.focused_field = crate::app::NewCityField::RegenerateBtn; }
                    else if row == 17 { self.state.focused_field = crate::app::NewCityField::StartBtn; }
                    else if row == 19 { self.state.focused_field = crate::app::NewCityField::BackBtn; }
                }
            }
            _ => {}
        }
        None
    }

    fn render(&mut self, frame: &mut Frame, _context: AppContext) {
        let area = frame.area();
        crate::ui::screens::new_city::render_new_city(frame, area, &mut self.state);
    }
}

// ── Load City Screen ─────────────────────────────────────────────────────────

pub struct LoadCityScreen {
    pub state: LoadCityState,
}

impl Screen for LoadCityScreen {
    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition> {
        let count = self.state.saves.len();
        match action {
            Action::MenuBack => return Some(ScreenTransition::Pop),
            Action::MoveCursor(_, dy) => {
                if count > 0 {
                    self.state.selected = if dy > 0 {
                        (self.state.selected + 1) % count
                    } else {
                        self.state.selected.checked_sub(1).unwrap_or(count.saturating_sub(1))
                    };
                }
            }
            Action::MenuSelect => {
                if let Some(entry) = self.state.saves.get(self.state.selected) {
                    match save::load_city(&entry.path) {
                        Ok((map, sim)) => {
                            if let Some(tx) = context.cmd_tx {
                                let _ = tx.send(crate::core::engine::EngineCommand::ReplaceState { map, sim });
                            }
                            return Some(ScreenTransition::Replace(Box::new(InGameScreen::new())));
                        }
                        Err(_) => {
                            // Message handling would go here
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn render(&mut self, frame: &mut Frame, _context: AppContext) {
        let area = frame.area();
        crate::ui::screens::load_city::render_load_city(frame, area, &self.state);
    }
}

// ── In Game Screen ───────────────────────────────────────────────────────────

pub struct InGameScreen {
    pub camera: Camera,
    pub current_tool: Tool,
    pub ui_areas: UiAreas,
    pub paused: bool,
    pub is_budget_open: bool,
    pub menu: MenuState<MenuAction>,
    pub menu_open_idx: Option<usize>,
    /// Drag/immediate message shown in status bar (overrides event queue).
    pub message: Option<String>,
    /// Notification queue — messages fade after their tick count reaches 0.
    pub event_messages: VecDeque<(String, u32)>,
    pub ticks_since_month: u32,
    pub ticks_per_month: u32,
    pub line_drag: Option<LineDrag>,
    pub rect_drag: Option<RectDrag>,
    /// Current heat-map overlay mode (Feature 1).
    pub overlay_mode: OverlayMode,
    /// Tile position open in the inspect popup, if any (Feature 3).
    pub inspect_pos: Option<(usize, usize)>,
    /// Auto-save tick counter (Feature 5).
    pub ticks_since_save: u32,
    pub auto_save_interval_ticks: u32,
    /// One-shot flags for notification events (Feature 4).
    pub first_building_notified: bool,
    pub deficit_warned: bool,
    /// Floating windows — each independently draggable.
    pub map_win:     FloatingWindow,
    pub panel_win:   FloatingWindow,
    pub budget_win:  FloatingWindow,
    pub inspect_win: FloatingWindow,
    /// Which window (if any) the user is currently dragging, plus the grab offset.
    pub window_drag: Option<WindowDrag>,
    /// States for each toolbar button (Feature: rat-widget migration).
    pub toolbar_btn_states: std::collections::HashMap<Tool, rat_widget::button::ButtonState>,
}

impl InGameScreen {
    pub fn new() -> Self {
        let mut toolbar_btn_states = std::collections::HashMap::new();
        for &tool in Tool::ALL.iter() {
            toolbar_btn_states.insert(tool, rat_widget::button::ButtonState::default());
        }

        Self {
            camera: Camera::default(),
            current_tool: Tool::Inspect,
            ui_areas: UiAreas::default(),
            paused: false,
            is_budget_open: false,
            menu: build_menu_state(&SimState::default(), false, 50, OverlayMode::None),
            menu_open_idx: None,
            message: None,
            event_messages: VecDeque::new(),
            ticks_since_month: 0,
            ticks_per_month: 50,
            line_drag: None,
            rect_drag: None,
            overlay_mode: OverlayMode::None,
            inspect_pos: None,
            ticks_since_save: 0,
            auto_save_interval_ticks: AUTO_SAVE_INTERVAL,
            first_building_notified: false,
            deficit_warned: false,
            // Windows. map uses 999 = "fill available". panel uses u16::MAX as
            // "not yet placed" sentinel so clamp_win right-aligns it on first render.
            map_win:     FloatingWindow::new(0,        2, 999, 999),
            panel_win:   FloatingWindow::new(u16::MAX, 4, 24,  35),
            budget_win:  FloatingWindow::new(10,       5, 46,  24),
            inspect_win: FloatingWindow::new(15,       5, 34,  16),
            window_drag: None,
            toolbar_btn_states,
        }
    }

    /// Returns true if the given screen coordinate overlaps any floating UI window.
    pub fn is_over_window(&self, col: u16, row: u16) -> bool {
        self.panel_win.contains(col, row)
            || (self.is_budget_open && self.budget_win.contains(col, row))
            || (self.inspect_pos.is_some() && self.inspect_win.contains(col, row))
    }

    /// Push a notification message into the event queue (Feature 4).
    /// Messages are displayed in the status bar and fade after ~80 ticks.
    pub fn push_message(&mut self, text: String) {
        self.event_messages.push_back((text, 80));
        if self.event_messages.len() > 5 {
            self.event_messages.pop_front();
        }
    }

    /// The message to display in the status bar.
    /// Priority: overlay label > drag message > front of event queue.
    pub fn status_message(&self) -> Option<&str> {
        if self.overlay_mode != OverlayMode::None {
            return Some(self.overlay_mode.label());
        }
        if let Some(ref m) = self.message {
            return Some(m.as_str());
        }
        self.event_messages.front().map(|(s, _)| s.as_str())
    }

    /// Returns a reference to the cached line-drag preview path — zero allocation, called each frame.
    pub fn line_preview(&self) -> &[(usize, usize)] {
        self.line_drag.as_ref().map(|d| d.path.as_slice()).unwrap_or(&[])
    }

    /// Returns a reference to the cached rect-drag preview tiles — zero allocation, called each frame.
    pub fn rect_preview(&self) -> &[(usize, usize)] {
        self.rect_drag.as_ref().map(|d| d.tiles_cache.as_slice()).unwrap_or(&[])
    }

    fn place_current_tool(&mut self, context: &AppContext) {
        let x = self.camera.cursor_x;
        let y = self.camera.cursor_y;
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::PlaceTool {
                tool: self.current_tool,
                x,
                y,
            });
        }
        self.message = None;
    }

    fn screen_to_map_clamped(&self, col: u16, row: u16, context: &AppContext) -> (usize, usize) {
        let sx = col - self.ui_areas.map.x;
        let sy = row - self.ui_areas.map.y;
        let (mx, my) = self.camera.screen_to_map(sx, sy);
        let engine = context.engine.read().unwrap();
        (mx.min(engine.map.width.saturating_sub(1)),
         my.min(engine.map.height.saturating_sub(1)))
    }

    fn commit_line_drag(&mut self, context: &AppContext) {
        let drag = match self.line_drag.take() { Some(d) => d, None => return };
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::PlaceLine {
                tool: drag.tool,
                path: drag.path,
            });
        }
        self.message = None;
    }

    fn commit_rect_drag(&mut self, context: &AppContext) {
        let drag = match self.rect_drag.take() { Some(d) => d, None => return };
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::PlaceRect {
                tool: drag.tool,
                tiles: drag.tiles_cache,
            });
        }
        self.message = None;
    }

    fn handle_menu_action(&mut self, action: MenuAction, context: &AppContext) -> Option<ScreenTransition> {
        match action {
            MenuAction::NewCity => {
                return Some(ScreenTransition::Pop);
            }
            MenuAction::SaveCity => {
                let result = {
                    let engine = context.engine.read().unwrap();
                    save::save_city(&engine.sim, &engine.map)
                };
                match result {
                    Ok(())  => self.push_message("City saved!".to_string()),
                    Err(e)  => self.push_message(format!("Save failed: {e}")),
                }
            }
            MenuAction::Quit => {
                return Some(ScreenTransition::Quit);
            }
            MenuAction::SpeedPause => {
                self.paused = !self.paused;
                if let Some(tx) = context.cmd_tx {
                    let _ = tx.send(EngineCommand::SetPaused(self.paused));
                }
            }
            MenuAction::SpeedSlow   => { self.ticks_per_month = 100; }
            MenuAction::SpeedNormal => { self.ticks_per_month = 50;  }
            MenuAction::SpeedFast   => { self.ticks_per_month = 20;  }
            MenuAction::DisasterFire | MenuAction::DisasterFlood | MenuAction::DisasterTornado => {
                let mut cfg = context.engine.read().unwrap().sim.disasters.clone();
                match action {
                    MenuAction::DisasterFire    => cfg.fire_enabled    = !cfg.fire_enabled,
                    MenuAction::DisasterFlood   => cfg.flood_enabled   = !cfg.flood_enabled,
                    MenuAction::DisasterTornado => cfg.tornado_enabled = !cfg.tornado_enabled,
                    _ => {}
                }
                if let Some(tx) = context.cmd_tx {
                    let _ = tx.send(EngineCommand::SetDisasters(cfg));
                }
            }
            MenuAction::OpenBudget => {
                self.is_budget_open = true;
            }
            MenuAction::OverlayNone       => { self.overlay_mode = OverlayMode::None; }
            MenuAction::OverlayPower      => { self.overlay_mode = OverlayMode::Power; }
            MenuAction::OverlayPollution  => { self.overlay_mode = OverlayMode::Pollution; }
            MenuAction::OverlayLandValue  => { self.overlay_mode = OverlayMode::LandValue; }
            MenuAction::OverlayCrime      => { self.overlay_mode = OverlayMode::Crime; }
            MenuAction::OverlayFireRisk   => { self.overlay_mode = OverlayMode::FireRisk; }
        }
        None
    }

    /// Returns the 0-based index of the top-level menu title clicked, or None.
    /// Mirrors the layout tui-menu uses: leading ' ' then ' {name} ' per group.
    fn calc_menu_click(&self, col: u16) -> Option<usize> {
        let speed_tag = match self.ticks_per_month { 0..=30 => "Fast", 31..=70 => "Normal", _ => "Slow" };
        let names: &[&str] = &["System", "Disasters", "Windows"];
        let speed_name = format!("Speed [{speed_tag}]");

        // Build widths in menu-bar order: System, Speed [...], Disasters, Windows
        let all: Vec<(&str, usize)> = vec![
            ("System",              "System".len()),
            (speed_name.as_str(),  speed_name.len()),
            ("Disasters",          "Disasters".len()),
            ("Windows",            "Windows".len()),
        ];
        let _ = names; // silence unused warning

        let mut x: u16 = 1; // initial leading space rendered by tui-menu
        for (i, (_, name_len)) in all.iter().enumerate() {
            let w = *name_len as u16 + 2; // ' {name} '
            if col >= x && col < x + w {
                return Some(i);
            }
            x += w;
        }
        None
    }

    /// Rebuild + open the dropdown for `menu_idx`, storing the open index.
    fn open_menu_at(&mut self, menu_idx: usize, context: &AppContext) {
        let engine = context.engine.read().unwrap();
        self.menu = build_menu_state(&engine.sim, self.paused, self.ticks_per_month, self.overlay_mode);
        drop(engine);
        self.menu.activate();
        for _ in 0..menu_idx { self.menu.right(); }
        self.menu.down(); // open the dropdown (selects first item)
        self.menu_open_idx = Some(menu_idx);
    }

    fn update_line_drag_message(&mut self, context: &AppContext) {
        if let Some(ref drag) = self.line_drag {
            let tool = drag.tool;
            let engine = context.engine.read().unwrap();
            let placeable = drag.path.iter()
                .filter(|&&(x, y)| x < engine.map.width && y < engine.map.height
                    && tool.can_place(engine.map.get(x, y)))
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

    fn update_rect_drag_message(&mut self, context: &AppContext) {
        if let Some(ref drag) = self.rect_drag {
            let tool = drag.tool;
            let engine = context.engine.read().unwrap();
            let placeable = drag.tiles_cache.iter()
                .filter(|&&(x, y)| x < engine.map.width && y < engine.map.height
                    && tool.can_place(engine.map.get(x, y)))
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

    fn handle_click(&mut self, col: u16, row: u16, is_click: bool, context: &AppContext) {
        if self.ui_areas.pause_btn.contains(col, row) {
            if is_click {
                self.paused = !self.paused;
                if let Some(tx) = context.cmd_tx {
                    let _ = tx.send(EngineCommand::SetPaused(self.paused));
                }
            }
            return;
        }

        // If the click is inside an overlay window but didn't hit a recognised
        // element above, consume it so it never falls through to the map.
        if self.panel_win.contains(col, row)
            || (self.is_budget_open && self.budget_win.contains(col, row))
            || (self.inspect_pos.is_some() && self.inspect_win.contains(col, row))
        {
            return;
        }

        let engine = context.engine.read().unwrap();
        if self.ui_areas.map.contains(col, row) {
            let sx = col - self.ui_areas.map.x;
            let sy = row - self.ui_areas.map.y;
            let (mx, my) = self.camera.screen_to_map(sx, sy);
            let mx = mx.min(engine.map.width.saturating_sub(1));
            let my = my.min(engine.map.height.saturating_sub(1));
            self.camera.cursor_x = mx;
            self.camera.cursor_y = my;

            if self.current_tool == Tool::Inspect {
                // Feature 3: open inspect popup for this tile
                self.inspect_pos = Some((mx, my));
            } else {
                drop(engine);
                self.place_current_tool(context);
            }
            return;
        }

        if self.ui_areas.minimap.contains(col, row) {
            let mm = self.ui_areas.minimap;
            // Row 0 of the minimap area is the title — ignore clicks on it.
            if row == mm.y || mm.height <= 1 { return; }
            let (mw, mh) = (engine.map.width, engine.map.height);
            // Render area starts one row below the title.
            let rc = (col - mm.x) as usize;
            let rr = (row - mm.y - 1) as usize;
            let rw = mm.width as usize;
            let rh = (mm.height - 1) as usize;
            // Endpoint-interpolation matching the minimap tile-sampling formula.
            let tile_x = if rw <= 1 { 0 } else { rc * (mw - 1) / (rw - 1) };
            let tile_y = if rh <= 1 { 0 } else { rr * (mh - 1) / (rh - 1) };
            let new_ox = tile_x as i32 - self.camera.view_w as i32 / 2;
            let new_oy = tile_y as i32 - self.camera.view_h as i32 / 2;
            self.camera.offset_x = new_ox.clamp(0, (mw as i32 - self.camera.view_w as i32).max(0));
            self.camera.offset_y = new_oy.clamp(0, (mh as i32 - self.camera.view_h as i32).max(0));
        }
    }
}

impl Screen for InGameScreen {
    fn on_event(&mut self, event: &crossterm::event::Event, _context: AppContext) -> Option<ScreenTransition> {
        use rat_widget::event::ButtonOutcome;
        
        // Handle toolbar buttons
        for (tool, state) in self.toolbar_btn_states.iter_mut() {
            // Toolbar buttons are always considered "focused" for mouse interaction purposes
            let outcome = rat_widget::button::handle_events(state, true, event);
            if outcome == ButtonOutcome::Pressed {
                self.current_tool = *tool;
                self.line_drag = None;
                self.rect_drag = None;
                // No transition, just internal state change
                return None; 
            }
        }
        
        None
    }

    fn on_tick(&mut self, context: AppContext) {
        if self.paused { return; }

        // Feature 4: fade front event message
        if let Some((_, ticks)) = self.event_messages.front_mut() {
            *ticks = ticks.saturating_sub(1);
            if *ticks == 0 {
                self.event_messages.pop_front();
            }
        }

        // Feature 4: notify when first building appears
        let first_building = {
            let engine = context.engine.read().unwrap();
            engine.sim.population > 0 && !self.first_building_notified
        };
        if first_building {
            self.first_building_notified = true;
            self.push_message("First residents have arrived!".to_string());
        }

        // Feature 4: notify on budget deficit
        let treasury = context.engine.read().unwrap().sim.treasury;
        if treasury < 0 && !self.deficit_warned {
            self.deficit_warned = true;
            self.push_message("Warning: budget deficit!".to_string());
        } else if treasury >= 0 {
            self.deficit_warned = false;
        }

        // Feature 5: auto-save
        self.ticks_since_save += 1;
        if self.ticks_since_save >= self.auto_save_interval_ticks {
            self.ticks_since_save = 0;
            let result = {
                let engine = context.engine.read().unwrap();
                save::save_city(&engine.sim, &engine.map)
            };
            match result {
                Ok(())  => self.push_message("Auto-saved.".to_string()),
                Err(e)  => self.push_message(format!("Auto-save failed: {e}")),
            }
        }

        // Month advance
        self.ticks_since_month += 1;
        if self.ticks_since_month >= self.ticks_per_month {
            self.ticks_since_month = 0;
            if let Some(tx) = context.cmd_tx {
                let _ = tx.send(EngineCommand::AdvanceMonth);
            }
        }
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition> {
        // ── Menu bar + dropdown mouse clicks ──────────────────────────────────
        if let Action::MouseClick { col, row } = action {
            let menu_bar_y = self.ui_areas.menu_bar_y;

            if row == menu_bar_y {
                // Click on the menu bar row: open/toggle the clicked menu title.
                if let Some(idx) = self.calc_menu_click(col) {
                    // Toggle: clicking the already-open menu closes it.
                    if self.menu_open_idx == Some(idx) {
                        self.menu.reset();
                        self.menu_open_idx = None;
                    } else {
                        self.open_menu_at(idx, &context);
                    }
                } else {
                    self.menu.reset();
                    self.menu_open_idx = None;
                }
                return None; // always consume menu-bar-row clicks
            }

            if self.menu_open_idx.is_some() {
                // Click below the bar while a dropdown is open.
                if row > menu_bar_y {
                    // row == menu_bar_y+1 is the dropdown border (top); items start at +2.
                    let below = row - menu_bar_y;
                    if below >= 2 {
                        let item_idx = (below - 2) as usize;
                        if let Some(open_idx) = self.menu_open_idx {
                            // Re-navigate to the clicked item and select it.
                            let engine = context.engine.read().unwrap();
                            self.menu = build_menu_state(&engine.sim, self.paused, self.ticks_per_month, self.overlay_mode);
                            drop(engine);
                            self.menu.activate();
                            for _ in 0..open_idx { self.menu.right(); }
                            self.menu.down(); // selects item 0
                            for _ in 0..item_idx { self.menu.down(); }
                            self.menu.select();

                            let events: Vec<_> = self.menu.drain_events().collect();
                            self.menu.reset();
                            self.menu_open_idx = None;
                            for event in events {
                                let MenuEvent::Selected(a) = event;
                                let t = self.handle_menu_action(a, &context);
                                if t.is_some() { return t; }
                            }
                            return None;
                        }
                    }
                }
                // Click outside the dropdown: close without side-effects.
                self.menu.reset();
                self.menu_open_idx = None;
                return None;
            }
            // ── Window title bar drag — check topmost windows first ───────────
            if self.is_budget_open && self.budget_win.title_bar_contains(col, row) {
                self.window_drag = Some(WindowDrag::Budget(col - self.budget_win.x, row - self.budget_win.y));
                return None;
            }
            if self.inspect_pos.is_some() && self.inspect_win.title_bar_contains(col, row) {
                self.window_drag = Some(WindowDrag::Inspect(col - self.inspect_win.x, row - self.inspect_win.y));
                return None;
            }
            if self.panel_win.title_bar_contains(col, row) {
                self.window_drag = Some(WindowDrag::Panel(col - self.panel_win.x, row - self.panel_win.y));
                return None;
            }
            if self.map_win.title_bar_contains(col, row) {
                self.window_drag = Some(WindowDrag::Map(col - self.map_win.x, row - self.map_win.y));
                return None;
            }

            // If we are over a window, don't trigger map actions (line drag, rect drag, etc.)
            if self.is_over_window(col, row) {
                self.handle_click(col, row, true, &context);
                return None;
            }

            // Menu is closed and click is not on bar: handle normally.
            if Tool::uses_line_drag(self.current_tool) && self.ui_areas.map.contains(col, row) {
                let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                self.camera.cursor_x = mx; self.camera.cursor_y = my;
                self.line_drag = Some(LineDrag::new(self.current_tool, mx, my));
                self.update_line_drag_message(&context);
            } else if Tool::uses_rect_drag(self.current_tool) && self.ui_areas.map.contains(col, row) {
                let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                self.camera.cursor_x = mx; self.camera.cursor_y = my;
                self.rect_drag = Some(RectDrag::new(self.current_tool, mx, my));
                self.update_rect_drag_message(&context);
            } else {
                self.handle_click(col, row, true, &context);
            }
            return None;
        }

        // ── Dropdown keyboard navigation (highest priority) ───────────────────
        if self.menu_open_idx.is_some() {
            match action {
                Action::MoveCursor(dx, dy) => {
                    if dx < 0      { self.menu.left(); }
                    else if dx > 0 { self.menu.right(); }
                    else if dy < 0 { self.menu.up(); }
                    else if dy > 0 { self.menu.down(); }
                }
                Action::MenuSelect => self.menu.select(),
                Action::MenuBack   => { self.menu.reset(); self.menu_open_idx = None; }
                _ => {}
            }
            // Process any actions emitted this frame
            let events: Vec<_> = self.menu.drain_events().collect();
            for event in events {
                let MenuEvent::Selected(action) = event;
                let transition = self.handle_menu_action(action, &context);
                self.menu.reset();
                self.menu_open_idx = None;
                if transition.is_some() { return transition; }
            }
            return None;
        }

        // ── Inspect popup (close on Esc) ──────────────────────────────────────
        if self.inspect_pos.is_some() && matches!(action, Action::MenuBack) {
            self.inspect_pos = None;
            return None;
        }

        // ── Budget popup ──────────────────────────────────────────────────────
        if self.is_budget_open {
            match action {
                Action::MenuBack | Action::CharInput('b') | Action::CharInput('B') => { self.is_budget_open = false; }
                Action::MoveCursor(_, dy) => {
                    let mut new_tax = context.engine.read().unwrap().sim.tax_rate as i32;
                    if dy < 0 { new_tax -= 1; } else if dy > 0 { new_tax += 1; }
                    if (0..=20).contains(&new_tax) {
                        if let Some(tx) = context.cmd_tx {
                            let _ = tx.send(EngineCommand::SetTaxRate(new_tax as u8));
                        }
                    }
                }
                _ => {}
            }
            return None;
        }

        match action {
            Action::MenuBack => {
                if self.rect_drag.is_some() { self.rect_drag = None; self.message = None; }
                else if self.line_drag.is_some() { self.line_drag = None; self.message = None; }
                else { return Some(ScreenTransition::Pop); }
            }
            Action::SaveGame => {
                let result = {
                    let engine = context.engine.read().unwrap();
                    save::save_city(&engine.sim, &engine.map)
                };
                match result {
                    Ok(())  => self.push_message("City saved!".to_string()),
                    Err(e)  => self.push_message(format!("Save failed: {e}")),
                }
            }
            Action::MoveCursor(dx, dy) => {
                let (mw, mh) = { let e = context.engine.read().unwrap(); (e.map.width, e.map.height) };
                self.camera.move_cursor(dx, dy, mw, mh);
                if self.current_tool != Tool::Inspect && !Tool::uses_footprint_preview(self.current_tool) {
                    self.place_current_tool(&context);
                }
            }
            Action::PanCamera(dx, dy) => {
                let (mw, mh) = { let e = context.engine.read().unwrap(); (e.map.width, e.map.height) };
                self.camera.pan(dx, dy, mw, mh);
            }
            Action::MenuActivate => {
                let engine = context.engine.read().unwrap();
                self.menu = build_menu_state(&engine.sim, self.paused, self.ticks_per_month, self.overlay_mode);
                drop(engine);
                self.menu.activate();
                return None;
            }
            Action::CharInput(c) => {
                if c == 'b' || c == 'B' || c == '$' { self.is_budget_open = true; return None; }
                // Tab cycles overlay mode (Feature 1)
                if c == '\t' {
                    self.overlay_mode = self.overlay_mode.next();
                    return None;
                }
                let new_tool = match c {
                    'q' => { *context.running = false; None }
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
            Action::MouseDrag { col, row } => {
                if let Some(ref drag) = self.window_drag {
                    match drag {
                        WindowDrag::Map(ox, oy)     => { self.map_win.x     = col.saturating_sub(*ox); self.map_win.y     = row.saturating_sub(*oy); }
                        WindowDrag::Panel(ox, oy)   => { self.panel_win.x   = col.saturating_sub(*ox); self.panel_win.y   = row.saturating_sub(*oy); }
                        WindowDrag::Budget(ox, oy)  => { self.budget_win.x  = col.saturating_sub(*ox); self.budget_win.y  = row.saturating_sub(*oy); }
                        WindowDrag::Inspect(ox, oy) => { self.inspect_win.x = col.saturating_sub(*ox); self.inspect_win.y = row.saturating_sub(*oy); }
                    }
                    return None;
                }
                
                if self.is_over_window(col, row) {
                    return None;
                }

                if self.line_drag.is_some() && self.ui_areas.map.contains(col, row) {
                    let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                    let (tool, sx, sy) = self.line_drag.as_ref().map(|d| (d.tool, d.start_x, d.start_y)).unwrap();
                    let new_path = { let e = context.engine.read().unwrap(); crate::app::line_drag::line_shortest_path(&e.map, tool, sx, sy, mx, my) };
                    if let Some(ref mut d) = self.line_drag { d.end_x = mx; d.end_y = my; d.path = new_path; }
                    self.camera.cursor_x = mx; self.camera.cursor_y = my;
                    self.update_line_drag_message(&context);
                } else if self.rect_drag.is_some() && self.ui_areas.map.contains(col, row) {
                    let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                    if let Some(ref mut d) = self.rect_drag { d.update_end(mx, my); }
                    self.camera.cursor_x = mx; self.camera.cursor_y = my;
                    self.update_rect_drag_message(&context);
                } else if self.line_drag.is_none() && self.rect_drag.is_none() {
                    self.handle_click(col, row, false, &context);
                }
            }
            Action::MouseUp { col, row } => {
                self.window_drag = None;
                if self.line_drag.is_some() {
                    if self.ui_areas.map.contains(col, row) && !self.is_over_window(col, row) {
                        let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                        let (tool, sx, sy) = self.line_drag.as_ref().map(|d| (d.tool, d.start_x, d.start_y)).unwrap();
                        let final_path = { let e = context.engine.read().unwrap(); crate::app::line_drag::line_shortest_path(&e.map, tool, sx, sy, mx, my) };
                        if let Some(ref mut d) = self.line_drag { d.end_x = mx; d.end_y = my; d.path = final_path; }
                    }
                    self.commit_line_drag(&context);
                } else if self.rect_drag.is_some() {
                    if self.ui_areas.map.contains(col, row) && !self.is_over_window(col, row) {
                        let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                        if let Some(ref mut d) = self.rect_drag { d.update_end(mx, my); }
                    }
                    self.commit_rect_drag(&context);
                }
            }
            Action::MouseMove { col, row } => {
                if Tool::uses_footprint_preview(self.current_tool)
                    && self.ui_areas.map.contains(col, row)
                    && !self.is_over_window(col, row)
                {
                    let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                    self.camera.cursor_x = mx; self.camera.cursor_y = my;
                }
            }
            _ => {}
        }
        None
    }

    fn render(&mut self, frame: &mut Frame, context: AppContext) {
        let area = frame.area();
        let mut mock_app = crate::app::AppState {
            screens: Vec::new(),
            engine: context.engine.clone(),
            cmd_tx: context.cmd_tx.clone(),
            running: *context.running,
        };
        crate::ui::render_game_v2(frame, area, &mut mock_app, self);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::OverlayMode;

    fn fresh_screen() -> InGameScreen {
        InGameScreen::new()
    }

    #[test]
    fn push_message_adds_to_queue() {
        let mut s = fresh_screen();
        s.push_message("hello".to_string());
        assert_eq!(s.event_messages.len(), 1);
        assert_eq!(s.event_messages.front().unwrap().0, "hello");
    }

    #[test]
    fn push_message_caps_queue_at_five() {
        let mut s = fresh_screen();
        for i in 0..10 {
            s.push_message(format!("msg {i}"));
        }
        assert_eq!(s.event_messages.len(), 5);
    }

    #[test]
    fn status_message_prefers_overlay_label() {
        let mut s = fresh_screen();
        s.message = Some("drag msg".to_string());
        s.push_message("event msg".to_string());
        s.overlay_mode = OverlayMode::Pollution;
        assert_eq!(s.status_message(), Some("[Overlay: Pollution]"));
    }

    #[test]
    fn status_message_uses_drag_when_no_overlay() {
        let mut s = fresh_screen();
        s.message = Some("dragging".to_string());
        s.push_message("event".to_string());
        assert_eq!(s.status_message(), Some("dragging"));
    }

    #[test]
    fn status_message_falls_back_to_event_queue() {
        let mut s = fresh_screen();
        s.push_message("notification".to_string());
        assert_eq!(s.status_message(), Some("notification"));
    }

    #[test]
    fn status_message_none_when_all_empty() {
        let s = fresh_screen();
        assert_eq!(s.status_message(), None);
    }

    #[test]
    fn overlay_mode_tab_cycles() {
        let mut s = fresh_screen();
        assert_eq!(s.overlay_mode, OverlayMode::None);
        s.overlay_mode = s.overlay_mode.next();
        assert_eq!(s.overlay_mode, OverlayMode::Power);
        s.overlay_mode = s.overlay_mode.next();
        assert_eq!(s.overlay_mode, OverlayMode::Pollution);
    }

    #[test]
    fn auto_save_interval_constant_is_positive() {
        assert!(AUTO_SAVE_INTERVAL > 0);
        // Should be at least a few seconds of ticks
        assert!(AUTO_SAVE_INTERVAL >= 100);
    }

    #[test]
    fn test_new_city_screen_rat_widget() {
        use crate::app::{NewCityState, NewCityField};
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, KeyEventKind, KeyEventState};

        let mut screen = NewCityScreen { state: NewCityState::new() };
        let engine = std::sync::Arc::new(std::sync::RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10), crate::core::sim::SimState::default()
        )));
        let cmd_tx = None;
        let mut running = true;
        
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };

        // Simulate typing in the seed input
        screen.state.focused_field = NewCityField::SeedInput;
        screen.state.seed_input.set_text("");
        
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        screen.on_event(&ev, context);
        assert_eq!(screen.state.seed_input.text(), "0000000000000001");
        
        screen.state.apply_seed_input(); // Trigger apply manually or via event logic
        assert_eq!(screen.state.seed, 1);
        
        let context_action = AppContext {
            engine: &engine, cmd_tx: &cmd_tx, running: &mut running
        };
        // We were focused on SeedInput, MenuSelect applies seed.
        let transition = screen.on_action(crate::app::input::Action::MenuSelect, context_action);
        assert!(transition.is_none());
        
        // Focus Start button
        screen.state.focused_field = NewCityField::StartBtn;
        let ev_enter = Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        let transition_start = screen.on_event(&ev_enter, AppContext {
            engine: &engine, cmd_tx: &cmd_tx, running: &mut running
        });
        assert!(transition_start.is_some()); // Should trigger Replace
    }
}
