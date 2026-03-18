use std::collections::VecDeque;

use crate::{
    app::{input::Action, save, Camera, FloatingWindow, LineDrag, RectDrag, Tool, UiAreas, WindowDrag},
    core::engine::EngineCommand,
    ui::theme::OverlayMode,
};
use ratatui::Frame;
use rat_widget::menu::MenubarState;

use super::{
    ingame_budget::BudgetUiState,
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
    pub is_budget_open: bool,
    pub budget_needs_center: bool,
    pub menu: MenubarState,
    pub menu_active: bool,
    pub menu_consumed_input: bool,
    pub budget_input_consumed: bool,
    pub popup_input_consumed: bool,
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
    pub map_win: FloatingWindow,
    pub panel_win: FloatingWindow,
    pub budget_win: FloatingWindow,
    pub inspect_win: FloatingWindow,
    pub window_drag: Option<WindowDrag>,
    pub(super) map_pan_drag: Option<MiddlePanDrag>,
    pub(super) scrollbar_drag: Option<ScrollbarDrag>,
    pub budget_ui: BudgetUiState,
    pub toolbar_btn_states: std::collections::HashMap<Tool, rat_widget::button::ButtonState>,
    pub show_plant_info: bool,
    pub plant_close_btn: rat_widget::button::ButtonState,
    pub budget_close_btn: rat_widget::button::ButtonState,
    pub inspect_close_btn: rat_widget::button::ButtonState,
    pub coal_picker_btn: rat_widget::button::ButtonState,
    pub gas_picker_btn: rat_widget::button::ButtonState,
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
            budget_needs_center: false,
            menu: Self::new_menu_state(),
            menu_active: false,
            menu_consumed_input: false,
            budget_input_consumed: false,
            popup_input_consumed: false,
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
            map_win: FloatingWindow::new(0, 2, 999, 999),
            panel_win: FloatingWindow::new(u16::MAX, 4, 24, 35),
            budget_win: FloatingWindow::new(8, 4, 74, 29),
            inspect_win: FloatingWindow::new(15, 5, 34, 16),
            window_drag: None,
            map_pan_drag: None,
            scrollbar_drag: None,
            budget_ui: BudgetUiState::new(),
            toolbar_btn_states,
            show_plant_info: false,
            plant_close_btn: rat_widget::button::ButtonState::default(),
            budget_close_btn: rat_widget::button::ButtonState::default(),
            inspect_close_btn: rat_widget::button::ButtonState::default(),
            coal_picker_btn: rat_widget::button::ButtonState::default(),
            gas_picker_btn: rat_widget::button::ButtonState::default(),
        }
    }

    pub fn is_over_window(&self, col: u16, row: u16) -> bool {
        self.panel_win.contains(col, row)
            || (self.is_budget_open && self.budget_win.contains(col, row))
            || (self.inspect_pos.is_some() && self.inspect_win.contains(col, row))
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

    fn take_budget_consumed_input(&mut self) -> bool {
        std::mem::take(&mut self.budget_input_consumed)
    }

    fn take_popup_consumed_input(&mut self) -> bool {
        std::mem::take(&mut self.popup_input_consumed)
    }

    pub fn rect_contains(area: ratatui::layout::Rect, col: u16, row: u16) -> bool {
        area.width > 0
            && area.height > 0
            && col >= area.x
            && col < area.x + area.width
            && row >= area.y
            && row < area.y + area.height
    }
}

impl Screen for InGameScreen {
    fn on_event(&mut self, event: &crossterm::event::Event, context: AppContext) -> Option<ScreenTransition> {
        use rat_widget::event::MenuOutcome;

        let menu_was_active = self.menu_active;
        let menu_outcome = rat_widget::menu::menubar::handle_popup_events(&mut self.menu, self.menu_active, event);
        if Self::menu_outcome_consumed(menu_outcome) || menu_was_active {
            self.menu_consumed_input = true;
        }

        match menu_outcome {
            MenuOutcome::MenuActivated(menu_idx, item_idx) => {
                let action = Self::menu_action_for(menu_idx, item_idx);
                self.close_menu();
                return self.handle_menu_action(action, &context);
            }
            MenuOutcome::Selected(menu_idx) if !menu_was_active => {
                self.open_menu(menu_idx);
                return None;
            }
            MenuOutcome::Selected(_) | MenuOutcome::Activated(_) | MenuOutcome::MenuSelected(_, _) => {
                self.menu_active = self.menu.popup_active() || self.menu.bar.selected().is_some();
                self.menu.bar.focus.set(self.menu_active);
                return None;
            }
            MenuOutcome::Hide => {
                self.close_menu();
                return None;
            }
            MenuOutcome::Changed | MenuOutcome::Unchanged => {
                self.menu_active = self.menu.popup_active() || menu_was_active;
                self.menu.bar.focus.set(self.menu_active);
                return None;
            }
            MenuOutcome::Continue => {}
        }

        if self.menu_active {
            return None;
        }

        if self.handle_popup_close_event(event) {
            return None;
        }

        if self.handle_budget_widget_event(event, &context) {
            return None;
        }

        if self.handle_power_popup_event(event) {
            return None;
        }

        if self.handle_toolbar_event(event) {
            return None;
        }

        None
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
        if self.take_menu_consumed_input() || self.take_budget_consumed_input() || self.take_popup_consumed_input() {
            return None;
        }

        if matches!(action, Action::MenuActivate) {
            if self.menu_active {
                self.close_menu();
            } else {
                self.open_menu(self.menu.bar.selected().unwrap_or(0));
            }
            return None;
        }

        if self.should_block_for_menu(&action) {
            if matches!(action, Action::MenuBack) {
                self.close_menu();
            }
            return None;
        }

        if let Action::MouseClick { col, row } = action {
            if self.handle_mouse_click_action(col, row, &context) {
                return None;
            }
        }

        if self.handle_budget_action(&action, &context) {
            return None;
        }

        if self.inspect_pos.is_some() && matches!(action, Action::MenuBack) {
            self.inspect_pos = None;
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
                        self.show_plant_info = !self.show_plant_info;
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
        assert!(screen.menu.popup_active());
        assert_eq!(screen.menu.bar.selected(), Some(0));

        let close_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let transition = screen.on_action(Action::MenuActivate, close_context);
        assert!(transition.is_none());
        assert!(!screen.menu_active);
        assert!(!screen.menu.popup_active());
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
        assert!(screen.is_budget_open);
        assert!(screen.budget_needs_center);
        assert_eq!(screen.budget_ui.residential_tax.value(), 13);
        assert_eq!(screen.budget_ui.commercial_tax.value(), 11);
        assert_eq!(screen.budget_ui.industrial_tax.value(), 7);
        assert_eq!(screen.budget_ui.residential_tax_input.text(), "13");
        assert_eq!(screen.budget_ui.commercial_tax_input.text(), "11");
        assert_eq!(screen.budget_ui.industrial_tax_input.text(), "7");
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

        let start_x = screen.budget_win.x;
        let start_y = screen.budget_win.y;
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

        assert_eq!(screen.budget_win.x, start_x + 4);
        assert_eq!(screen.budget_win.y, start_y + 3);
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
        screen.budget_ui.commercial_tax_input.set_text("");

        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let type_four = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('4'),
            crossterm::event::KeyModifiers::empty(),
        ));
        screen.on_event(&type_four, event_context);
        let cmd = rx.try_recv().expect("budget text input should emit a tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let type_two = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('2'),
            crossterm::event::KeyModifiers::empty(),
        ));
        screen.on_event(&type_two, event_context);
        let cmd = rx.try_recv().expect("budget text input should emit a second tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        assert_eq!(screen.budget_ui.commercial_tax_input.text(), "42");
        assert_eq!(screen.budget_ui.commercial_tax.value(), 42);
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
        screen.budget_ui.residential_tax_input.set_text("");

        for key in ['1', '2', '3'] {
            let event_context = AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
                running: &mut running,
            };
            let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char(key),
                crossterm::event::KeyModifiers::empty(),
            ));
            screen.on_event(&event, event_context);
            let cmd = rx.try_recv().expect("typed tax input should emit a tax update command");
            engine.write().unwrap().execute_command(cmd).unwrap();
        }

        assert_eq!(screen.budget_ui.residential_tax_input.text(), "100");
        assert_eq!(screen.budget_ui.residential_tax.value(), 100);
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
        let start_value = screen.budget_ui.commercial_tax.value();

        let left_event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::empty(),
        ));
        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let transition = screen.on_event(&left_event, event_context);
        assert!(transition.is_none());

        let action_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let transition = screen.on_action(Action::MoveCursor(-1, 0), action_context);
        assert!(transition.is_none());
        let cmd = rx.try_recv().expect("left key adjustment should emit a tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        assert_eq!(screen.budget_ui.commercial_tax.value(), start_value.saturating_sub(1));
        assert_eq!(screen.budget_ui.commercial_tax_input.text(), (start_value.saturating_sub(1)).to_string());
        assert_eq!(engine.read().unwrap().sim.tax_rates.commercial as usize, start_value.saturating_sub(1));
    }
}
