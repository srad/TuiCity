use std::collections::VecDeque;

use crate::{
    app::{input::Action, save, Camera, DesktopState, LineDrag, RectDrag, Tool, UiAreas, WindowId},
    core::engine::EngineCommand,
    ui::{theme::OverlayMode, view::{BudgetViewModel, InGameDesktopView}},
};

use super::{
    ingame_budget::BudgetState,
    AppContext, Screen, ScreenTransition,
};

/// Ticks between auto-saves (50 ticks/month × 12 months × 6 months ≈ every 6 in-game months).
pub const AUTO_SAVE_INTERVAL: u32 = 50 * 12 * 6;

#[derive(Debug, Clone, Copy)]
pub(super) struct MiddlePanDrag {
    pub(super) last_col: u16,
    pub(super) last_row: u16,
    pub(super) carry_cols: i32,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ScrollbarAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ScrollbarDrag {
    pub(super) axis: ScrollbarAxis,
    pub(super) grab_offset: u16,
}

pub struct InGameScreen {
    pub camera: Camera,
    pub current_tool: Tool,
    pub ui_areas: UiAreas,
    pub paused: bool,
    pub menu_active: bool,
    pub menu_selected: usize,
    pub menu_item_selected: usize,
    pub message: Option<String>,
    pub event_messages: VecDeque<(String, u32)>,
    pub ticks_since_month: u32,
    pub ticks_per_month: u32,
    pub line_drag: Option<LineDrag>,
    pub rect_drag: Option<RectDrag>,
    pub overlay_mode: OverlayMode,
    pub inspect_pos: Option<(usize, usize)>,
    pub ticks_since_save: u32,
    pub auto_save_interval_ticks: u32,
    pub first_building_notified: bool,
    pub deficit_warned: bool,
    pub desktop: DesktopState,
    pub(super) map_pan_drag: Option<MiddlePanDrag>,
    pub(super) scrollbar_drag: Option<ScrollbarDrag>,
    pub budget_ui: BudgetState,
}

impl InGameScreen {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            current_tool: Tool::Inspect,
            ui_areas: UiAreas::default(),
            paused: false,
            menu_active: false,
            menu_selected: 0,
            menu_item_selected: 0,
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
            desktop: DesktopState::new_ingame(),
            map_pan_drag: None,
            scrollbar_drag: None,
            budget_ui: BudgetState::new(),
        }
    }

    pub fn is_over_window(&self, col: u16, row: u16) -> bool {
        self.desktop.contains(WindowId::Panel, col, row)
            || self.desktop.contains(WindowId::Budget, col, row)
            || self.desktop.contains(WindowId::Inspect, col, row)
            || self.desktop.contains(WindowId::PowerPicker, col, row)
    }

    pub fn push_message(&mut self, text: String) {
        self.event_messages.push_back((text, 80));
        if self.event_messages.len() > 5 {
            self.event_messages.pop_front();
        }
    }

    pub fn status_message(&self) -> Option<&str> {
        if self.overlay_mode != OverlayMode::None {
            return Some(self.overlay_mode.label());
        }
        if let Some(ref message) = self.message {
            return Some(message.as_str());
        }
        self.event_messages.front().map(|(message, _)| message.as_str())
    }

    pub fn line_preview(&self) -> &[(usize, usize)] {
        self.line_drag.as_ref().map(|drag| drag.path.as_slice()).unwrap_or(&[])
    }

    pub fn rect_preview(&self) -> &[(usize, usize)] {
        self.rect_drag.as_ref().map(|drag| drag.tiles_cache.as_slice()).unwrap_or(&[])
    }

    pub fn is_budget_open(&self) -> bool {
        self.desktop.is_open(WindowId::Budget)
    }

    pub fn is_power_picker_open(&self) -> bool {
        self.desktop.is_open(WindowId::PowerPicker)
    }

    pub fn is_inspect_open(&self) -> bool {
        self.desktop.is_open(WindowId::Inspect)
    }

    pub fn build_view(
        &self,
        sim: &crate::core::sim::SimState,
        map: &crate::core::map::Map,
    ) -> InGameDesktopView {
        let tax_rates = crate::core::sim::TaxRates {
            residential: self.budget_ui.residential_tax as u8,
            commercial: self.budget_ui.commercial_tax as u8,
            industrial: self.budget_ui.industrial_tax as u8,
        };
        InGameDesktopView {
            map: map.clone(),
            sim: sim.clone(),
            camera: self.camera.clone(),
            current_tool: self.current_tool,
            paused: self.paused,
            overlay_mode: self.overlay_mode,
            menu_active: self.menu_active,
            menu_selected: self.menu_selected,
            menu_item_selected: self.menu_item_selected,
            status_message: self.status_message().map(str::to_string),
            line_preview: self.line_preview().to_vec(),
            rect_preview: self.rect_preview().to_vec(),
            inspect_pos: self.inspect_pos,
            budget: BudgetViewModel::from_sim(
                sim,
                self.budget_ui.focused,
                tax_rates,
                self.budget_ui.residential_tax_input.clone(),
                self.budget_ui.commercial_tax_input.clone(),
                self.budget_ui.industrial_tax_input.clone(),
            ),
        }
    }

    pub fn select_tool(&mut self, tool: Tool) {
        self.current_tool = tool;
        self.line_drag = None;
        self.rect_drag = None;
        self.desktop.close(WindowId::PowerPicker);
    }
}

impl Screen for InGameScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_tick(&mut self, context: AppContext) {
        if self.paused {
            return;
        }
        if let Some((_, ticks)) = self.event_messages.front_mut() {
            *ticks = ticks.saturating_sub(1);
            if *ticks == 0 {
                self.event_messages.pop_front();
            }
        }
        let first_building = {
            let engine = context.engine.read().unwrap();
            engine.sim.population > 0 && !self.first_building_notified
        };
        if first_building {
            self.first_building_notified = true;
            self.push_message("First residents have arrived!".to_string());
        }
        let treasury = context.engine.read().unwrap().sim.treasury;
        if treasury < 0 && !self.deficit_warned {
            self.deficit_warned = true;
            self.push_message("Warning: budget deficit!".to_string());
        } else if treasury >= 0 {
            self.deficit_warned = false;
        }
        self.ticks_since_save += 1;
        if self.ticks_since_save >= self.auto_save_interval_ticks {
            self.ticks_since_save = 0;
            let result = {
                let engine = context.engine.read().unwrap();
                save::save_city(&engine.sim, &engine.map)
            };
            match result {
                Ok(()) => self.push_message("Auto-saved.".to_string()),
                Err(e) => self.push_message(format!("Auto-save failed: {e}")),
            }
        }
        self.ticks_since_month += 1;
        if self.ticks_since_month >= self.ticks_per_month {
            self.ticks_since_month = 0;
            if let Some(tx) = context.cmd_tx {
                let _ = tx.send(EngineCommand::AdvanceMonth);
            }
        }
    }

    fn on_action(&mut self, action: Action, context: AppContext) -> Option<ScreenTransition> {
        if let Action::MouseClick { col, row } = action {
            let (consumed, transition) = self.handle_menu_click(col, row, &context);
            if consumed {
                return transition;
            }
        }

        if matches!(action, Action::MenuActivate) {
            if self.menu_active {
                self.close_menu();
            } else {
                self.open_menu(self.menu_selected);
            }
            return None;
        }

        if self.should_block_for_menu(&action) {
            match action {
                Action::MenuBack => {
                    self.close_menu();
                    return None;
                }
                Action::MoveCursor(dx, dy) => {
                    if dx < 0 {
                        self.menu_selected = self.menu_selected.saturating_sub(1);
                        self.menu_item_selected = self
                            .menu_item_selected
                            .min(self.menu_item_count(self.menu_selected).saturating_sub(1));
                    } else if dx > 0 {
                        self.menu_selected = (self.menu_selected + 1).min(4);
                        self.menu_item_selected = self
                            .menu_item_selected
                            .min(self.menu_item_count(self.menu_selected).saturating_sub(1));
                    } else if dy < 0 {
                        self.menu_item_selected = self.menu_item_selected.saturating_sub(1);
                    } else if dy > 0 {
                        self.menu_item_selected = (self.menu_item_selected + 1)
                            .min(self.menu_item_count(self.menu_selected).saturating_sub(1));
                    }
                    return None;
                }
                Action::MenuSelect => {
                    let action = self.current_menu_action();
                    self.close_menu();
                    return self.handle_menu_action(action, &context);
                }
                Action::MouseClick { .. } => {
                    self.close_menu();
                    return None;
                }
                _ => return None,
            }
        }

        if let Action::MouseClick { col, row } = action {
            if self.handle_mouse_click_action(col, row, &context) {
                return None;
            }
        }

        if self.handle_budget_action(&action, &context) {
            return None;
        }

        if self.is_power_picker_open() && matches!(action, Action::MenuBack) {
            self.desktop.close(WindowId::PowerPicker);
            return None;
        }

        if self.is_inspect_open() && matches!(action, Action::MenuBack) {
            self.inspect_pos = None;
            self.desktop.close(WindowId::Inspect);
            return None;
        }

        match action {
            Action::MenuBack => {
                if self.rect_drag.is_some() {
                    self.rect_drag = None;
                    self.message = None;
                } else if self.line_drag.is_some() {
                    self.line_drag = None;
                    self.message = None;
                } else {
                    return Some(ScreenTransition::Pop);
                }
            }
            Action::SaveGame => {
                let result = {
                    let engine = context.engine.read().unwrap();
                    save::save_city(&engine.sim, &engine.map)
                };
                match result {
                    Ok(()) => self.push_message("City saved!".to_string()),
                    Err(e) => self.push_message(format!("Save failed: {e}")),
                }
            }
            Action::MoveCursor(dx, dy) => {
                let (mw, mh) = {
                    let engine = context.engine.read().unwrap();
                    (engine.map.width, engine.map.height)
                };
                self.camera.move_cursor(dx, dy, mw, mh);
                if self.current_tool != Tool::Inspect && !Tool::uses_footprint_preview(self.current_tool) {
                    self.place_current_tool(&context);
                }
            }
            Action::PanCamera(dx, dy) => {
                let (mw, mh) = {
                    let engine = context.engine.read().unwrap();
                    (engine.map.width, engine.map.height)
                };
                self.camera.pan(dx, dy, mw, mh);
            }
            Action::CharInput(c) => {
                if c == 'b' || c == 'B' || c == '$' {
                    self.open_budget(&context);
                    return None;
                }
                if c == '\t' {
                    self.overlay_mode = self.overlay_mode.next();
                    return None;
                }
                let new_tool = match c {
                    'q' => {
                        *context.running = false;
                        None
                    }
                    ' ' => {
                        self.paused = !self.paused;
                        None
                    }
                    '?' => Some(Tool::Inspect),
                    '1' => Some(Tool::ZoneRes),
                    '2' => Some(Tool::ZoneComm),
                    '3' => Some(Tool::ZoneInd),
                    'r' => Some(Tool::Road),
                    'l' => Some(Tool::Rail),
                    'p' => Some(Tool::PowerLine),
                    'e' => {
                        self.desktop.toggle(WindowId::PowerPicker, true);
                        Some(Tool::PowerPlantCoal)
                    }
                    'g' => Some(Tool::PowerPlantGas),
                    'k' => Some(Tool::Park),
                    's' => Some(Tool::Police),
                    'f' => Some(Tool::Fire),
                    'b' => Some(Tool::Bulldoze),
                    _ => None,
                };
                if let Some(tool) = new_tool {
                    self.select_tool(tool);
                }
            }
            Action::MouseMiddleDown { col, row } => {
                if !self.is_over_window(col, row) && self.ui_areas.map.viewport.contains(col, row) {
                    self.start_middle_pan(col, row);
                }
            }
            Action::MouseDrag { col, row } => {
                if self.handle_mouse_drag_action(col, row, &context) {
                    return None;
                }
            }
            Action::MouseMiddleDrag { col, row } => {
                if self.map_pan_drag.is_some() {
                    self.drag_middle_pan(col, row, &context);
                }
            }
            Action::MouseUp { col, row } => {
                if self.handle_mouse_up_action(col, row, &context) {
                    return None;
                }
            }
            Action::MouseMiddleUp => {
                self.map_pan_drag = None;
            }
            Action::MouseMove { col, row } => {
                if Tool::uses_footprint_preview(self.current_tool)
                    && self.ui_areas.map.viewport.contains(col, row)
                    && !self.is_over_window(col, row)
                {
                    let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                    self.camera.cursor_x = mx;
                    self.camera.cursor_y = my;
                }
            }
            _ => {}
        }
        None
    }

    fn build_view(&self, context: AppContext<'_>) -> crate::ui::view::ScreenView {
        let engine = context.engine.read().unwrap();
        crate::ui::view::ScreenView::InGame(self.build_view(&engine.sim, &engine.map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::screens::BudgetFocus;
    use std::sync::{Arc, RwLock};

    fn fresh_screen() -> InGameScreen {
        InGameScreen::new()
    }

    #[test]
    fn push_message_adds_to_queue() {
        let mut screen = fresh_screen();
        screen.push_message("hello".to_string());
        assert_eq!(screen.event_messages.len(), 1);
        assert_eq!(screen.event_messages.front().unwrap().0, "hello");
    }

    #[test]
    fn push_message_caps_queue_at_five() {
        let mut screen = fresh_screen();
        for i in 0..10 {
            screen.push_message(format!("msg {i}"));
        }
        assert_eq!(screen.event_messages.len(), 5);
    }

    #[test]
    fn status_message_prefers_overlay_label() {
        let mut screen = fresh_screen();
        screen.message = Some("drag msg".to_string());
        screen.push_message("event msg".to_string());
        screen.overlay_mode = OverlayMode::Pollution;
        assert_eq!(screen.status_message(), Some("[Overlay: Pollution]"));
    }

    #[test]
    fn status_message_uses_drag_when_no_overlay() {
        let mut screen = fresh_screen();
        screen.message = Some("dragging".to_string());
        screen.push_message("event".to_string());
        assert_eq!(screen.status_message(), Some("dragging"));
    }

    #[test]
    fn status_message_falls_back_to_event_queue() {
        let mut screen = fresh_screen();
        screen.push_message("notification".to_string());
        assert_eq!(screen.status_message(), Some("notification"));
    }

    #[test]
    fn status_message_none_when_all_empty() {
        let screen = fresh_screen();
        assert_eq!(screen.status_message(), None);
    }

    #[test]
    fn overlay_mode_tab_cycles() {
        let mut screen = fresh_screen();
        assert_eq!(screen.overlay_mode, OverlayMode::None);
        screen.overlay_mode = screen.overlay_mode.next();
        assert_eq!(screen.overlay_mode, OverlayMode::Power);
        screen.overlay_mode = screen.overlay_mode.next();
        assert_eq!(screen.overlay_mode, OverlayMode::Pollution);
    }

    #[test]
    fn auto_save_interval_constant_is_positive() {
        assert!(AUTO_SAVE_INTERVAL > 0);
        assert!(AUTO_SAVE_INTERVAL >= 100);
    }

    #[test]
    fn menu_activate_opens_and_closes_menubar() {
        let mut screen = fresh_screen();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = None;
        let mut running = true;

        let open_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let transition = screen.on_action(Action::MenuActivate, open_context);
        assert!(transition.is_none());
        assert!(screen.menu_active);
        assert_eq!(screen.menu_selected, 0);

        let close_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let transition = screen.on_action(Action::MenuActivate, close_context);
        assert!(transition.is_none());
        assert!(!screen.menu_active);
    }

    #[test]
    fn middle_pan_drag_moves_camera() {
        let mut screen = fresh_screen();
        screen.camera.offset_x = 10;
        screen.camera.offset_y = 10;
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(100, 100),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = None;
        let mut running = true;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };

        screen.start_middle_pan(10, 10);
        screen.drag_middle_pan(14, 12, &context);

        assert_eq!(screen.camera.offset_x, 8);
        assert_eq!(screen.camera.offset_y, 8);
    }

    #[test]
    fn horizontal_scrollbar_increment_pans_right() {
        let mut screen = fresh_screen();
        screen.ui_areas.map.viewport = crate::app::ClickArea { x: 0, y: 0, width: 20, height: 10 };
        screen.ui_areas.map.horizontal_bar = crate::app::ClickArea { x: 0, y: 10, width: 20, height: 1 };
        screen.ui_areas.map.horizontal_inc = crate::app::ClickArea { x: 19, y: 10, width: 1, height: 1 };

        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(100, 100),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = None;
        let mut running = true;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };

        let consumed = screen.handle_scrollbar_click(19, 10, &context);
        assert!(consumed);
        assert_eq!(screen.camera.offset_x, 1);
    }

    #[test]
    fn budget_open_syncs_sector_sliders_to_sim_tax_rates() {
        let mut screen = fresh_screen();
        let mut sim = crate::core::sim::SimState::default();
        sim.tax_rates.residential = 13;
        sim.tax_rates.commercial = 11;
        sim.tax_rates.industrial = 7;
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            sim,
        )));
        let cmd_tx = None;
        let mut running = true;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };

        let transition = screen.on_action(Action::CharInput('b'), context);
        assert!(transition.is_none());
        assert!(screen.is_budget_open());
        assert!(screen.desktop.window(WindowId::Budget).center_on_open);
        assert_eq!(screen.budget_ui.residential_tax, 13);
        assert_eq!(screen.budget_ui.commercial_tax, 11);
        assert_eq!(screen.budget_ui.industrial_tax, 7);
        assert_eq!(screen.budget_ui.residential_tax_input, "13");
        assert_eq!(screen.budget_ui.commercial_tax_input, "11");
        assert_eq!(screen.budget_ui.industrial_tax_input, "7");
    }

    #[test]
    fn budget_window_can_be_dragged_while_open() {
        let mut screen = fresh_screen();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = None;
        let mut running = true;

        let open_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::CharInput('b'), open_context);

        let start_x = screen.desktop.window(WindowId::Budget).x;
        let start_y = screen.desktop.window(WindowId::Budget).y;
        let click_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::MouseClick { col: start_x + 2, row: start_y }, click_context);
        let drag_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::MouseDrag { col: start_x + 6, row: start_y + 3 }, drag_context);

        assert_eq!(screen.desktop.window(WindowId::Budget).x, start_x + 4);
        assert_eq!(screen.desktop.window(WindowId::Budget).y, start_y + 3);
    }

    #[test]
    fn budget_tax_text_input_updates_state() {
        let mut screen = fresh_screen();
        let (tx, rx) = std::sync::mpsc::channel();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = Some(tx);
        let mut running = true;

        let open_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::CharInput('b'), open_context);
        screen.budget_ui.focused = BudgetFocus::CommercialTax;
        screen.budget_ui.commercial_tax_input.clear();

        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::CharInput('4'), event_context);
        let cmd = rx.try_recv().expect("budget text input should emit a tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::CharInput('2'), event_context);
        let cmd = rx.try_recv().expect("budget text input should emit a second tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        assert_eq!(screen.budget_ui.commercial_tax_input, "42");
        assert_eq!(screen.budget_ui.commercial_tax, 42);
        assert_eq!(engine.read().unwrap().sim.tax_rates.commercial, 42);
    }

    #[test]
    fn budget_tax_text_input_clamps_to_one_hundred_percent() {
        let mut screen = fresh_screen();
        let (tx, rx) = std::sync::mpsc::channel();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = Some(tx);
        let mut running = true;

        let open_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::CharInput('b'), open_context);
        screen.budget_ui.focused = BudgetFocus::ResidentialTax;
        screen.budget_ui.residential_tax_input.clear();

        for key in ['1', '2', '3'] {
            let event_context = AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
                running: &mut running,
            };
            screen.on_action(Action::CharInput(key), event_context);
            let cmd = rx.try_recv().expect("typed tax input should emit a tax update command");
            engine.write().unwrap().execute_command(cmd).unwrap();
        }

        assert_eq!(screen.budget_ui.residential_tax_input, "100");
        assert_eq!(screen.budget_ui.residential_tax, 100);
        assert_eq!(engine.read().unwrap().sim.tax_rates.residential, 100);
    }

    #[test]
    fn budget_left_right_keys_adjust_focused_tax() {
        let mut screen = fresh_screen();
        let (tx, rx) = std::sync::mpsc::channel();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = Some(tx);
        let mut running = true;

        let open_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::CharInput('b'), open_context);
        screen.budget_ui.focused = BudgetFocus::CommercialTax;
        let start_value = screen.budget_ui.commercial_tax;

        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let transition = screen.on_action(Action::MoveCursor(-1, 0), event_context);
        assert!(transition.is_none());
        let cmd = rx.try_recv().expect("left key adjustment should emit a tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        assert_eq!(screen.budget_ui.commercial_tax, start_value.saturating_sub(1));
        assert_eq!(screen.budget_ui.commercial_tax_input, (start_value.saturating_sub(1)).to_string());
        assert_eq!(engine.read().unwrap().sim.tax_rates.commercial as usize, start_value.saturating_sub(1));
    }
}
