use crate::{
    app::{save, Tool, WindowId},
    core::{engine::EngineCommand, sim::SimState},
    ui::theme::OverlayMode,
};

use super::{AppContext, InGameScreen, ScreenTransition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuAction {
    #[default]
    None,
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
    PowerCoal,
    PowerGas,
    OpenBudget,
    OverlayNone,
    OverlayPower,
    OverlayPollution,
    OverlayLandValue,
    OverlayCrime,
    OverlayFireRisk,
}

#[derive(Debug, Clone, Copy)]
pub struct MenuRow {
    pub label: &'static str,
    pub right: &'static str,
    pub action: MenuAction,
}

pub const MENU_TITLES: [&str; 5] = ["System", "Speed", "Disasters", "Power", "Windows"];

const SYSTEM_ROWS: [MenuRow; 3] = [
    MenuRow { label: "New City", right: "Alt+N", action: MenuAction::NewCity },
    MenuRow { label: "Save City", right: "^S", action: MenuAction::SaveCity },
    MenuRow { label: "Quit", right: "Q", action: MenuAction::Quit },
];
const SPEED_ROWS: [MenuRow; 4] = [
    MenuRow { label: "Pause / Resume", right: "Space", action: MenuAction::SpeedPause },
    MenuRow { label: "Slow", right: "100", action: MenuAction::SpeedSlow },
    MenuRow { label: "Normal", right: "50", action: MenuAction::SpeedNormal },
    MenuRow { label: "Fast", right: "20", action: MenuAction::SpeedFast },
];
const DISASTER_ROWS: [MenuRow; 3] = [
    MenuRow { label: "Fire", right: "", action: MenuAction::DisasterFire },
    MenuRow { label: "Flood", right: "", action: MenuAction::DisasterFlood },
    MenuRow { label: "Tornado", right: "", action: MenuAction::DisasterTornado },
];
const POWER_ROWS: [MenuRow; 2] = [
    MenuRow { label: "Coal Plant", right: "$3000", action: MenuAction::PowerCoal },
    MenuRow { label: "Gas Plant", right: "$6000", action: MenuAction::PowerGas },
];
const WINDOWS_ROWS: [MenuRow; 7] = [
    MenuRow { label: "Budget & Taxes", right: "$", action: MenuAction::OpenBudget },
    MenuRow { label: "Overlay: Off", right: "", action: MenuAction::OverlayNone },
    MenuRow { label: "Overlay: Power", right: "", action: MenuAction::OverlayPower },
    MenuRow { label: "Overlay: Pollution", right: "", action: MenuAction::OverlayPollution },
    MenuRow { label: "Overlay: Land Value", right: "", action: MenuAction::OverlayLandValue },
    MenuRow { label: "Overlay: Crime", right: "", action: MenuAction::OverlayCrime },
    MenuRow { label: "Overlay: Fire Risk", right: "", action: MenuAction::OverlayFireRisk },
];

pub fn menu_rows(menu: usize) -> &'static [MenuRow] {
    match menu {
        0 => &SYSTEM_ROWS,
        1 => &SPEED_ROWS,
        2 => &DISASTER_ROWS,
        3 => &POWER_ROWS,
        4 => &WINDOWS_ROWS,
        _ => &[],
    }
}

impl InGameScreen {
    pub fn handle_menu_click(
        &mut self,
        col: u16,
        row: u16,
        context: &AppContext,
    ) -> (bool, Option<ScreenTransition>) {
        for (idx, area) in self.ui_areas.menu_items.iter().enumerate() {
            if area.contains(col, row) {
                self.menu_selected = idx;
                self.menu_item_selected = 0;
                self.open_menu(idx);
                return (true, None);
            }
        }

        if self.menu_active {
            for (idx, area) in self.ui_areas.menu_popup_items.iter().enumerate() {
                if area.contains(col, row) {
                    self.menu_item_selected = idx;
                    let action = self.current_menu_action();
                    self.close_menu();
                    return (true, self.handle_menu_action(action, context));
                }
            }

            if self.ui_areas.menu_popup.contains(col, row) || self.ui_areas.menu_bar.contains(col, row) {
                return (true, None);
            }

            self.close_menu();
            return (true, None);
        }

        (false, None)
    }

    pub fn handle_menu_action(
        &mut self,
        action: MenuAction,
        context: &AppContext,
    ) -> Option<ScreenTransition> {
        match action {
            MenuAction::None => {}
            MenuAction::NewCity => return Some(ScreenTransition::Pop),
            MenuAction::SaveCity => {
                let result = {
                    let engine = context.engine.read().unwrap();
                    save::save_city(&engine.sim, &engine.map)
                };
                match result {
                    Ok(()) => self.push_message("City saved!".to_string()),
                    Err(e) => self.push_message(format!("Save failed: {e}")),
                }
            }
            MenuAction::Quit => return Some(ScreenTransition::Quit),
            MenuAction::SpeedPause => {
                self.paused = !self.paused;
                if let Some(tx) = context.cmd_tx {
                    let _ = tx.send(EngineCommand::SetPaused(self.paused));
                }
            }
            MenuAction::SpeedSlow => self.ticks_per_month = 100,
            MenuAction::SpeedNormal => self.ticks_per_month = 50,
            MenuAction::SpeedFast => self.ticks_per_month = 20,
            MenuAction::DisasterFire | MenuAction::DisasterFlood | MenuAction::DisasterTornado => {
                let mut cfg = context.engine.read().unwrap().sim.disasters.clone();
                match action {
                    MenuAction::DisasterFire => cfg.fire_enabled = !cfg.fire_enabled,
                    MenuAction::DisasterFlood => cfg.flood_enabled = !cfg.flood_enabled,
                    MenuAction::DisasterTornado => cfg.tornado_enabled = !cfg.tornado_enabled,
                    _ => {}
                }
                if let Some(tx) = context.cmd_tx {
                    let _ = tx.send(EngineCommand::SetDisasters(cfg));
                }
            }
            MenuAction::PowerCoal => {
                self.current_tool = Tool::PowerPlantCoal;
                self.desktop.close(WindowId::PowerPicker);
            }
            MenuAction::PowerGas => {
                self.current_tool = Tool::PowerPlantGas;
                self.desktop.close(WindowId::PowerPicker);
            }
            MenuAction::OpenBudget => self.open_budget(context),
            MenuAction::OverlayNone => self.overlay_mode = OverlayMode::None,
            MenuAction::OverlayPower => self.overlay_mode = OverlayMode::Power,
            MenuAction::OverlayPollution => self.overlay_mode = OverlayMode::Pollution,
            MenuAction::OverlayLandValue => self.overlay_mode = OverlayMode::LandValue,
            MenuAction::OverlayCrime => self.overlay_mode = OverlayMode::Crime,
            MenuAction::OverlayFireRisk => self.overlay_mode = OverlayMode::FireRisk,
        }
        None
    }

    pub fn menu_item_count(&self, menu: usize) -> usize {
        menu_rows(menu).len()
    }

    pub fn menu_row(&self, menu: usize, item: usize, sim: &SimState) -> Option<(String, String, MenuAction)> {
        let base = *menu_rows(menu).get(item)?;
        let right = match base.action {
            MenuAction::DisasterFire => if sim.disasters.fire_enabled { "ON" } else { "OFF" },
            MenuAction::DisasterFlood => if sim.disasters.flood_enabled { "ON" } else { "OFF" },
            MenuAction::DisasterTornado => if sim.disasters.tornado_enabled { "ON" } else { "OFF" },
            MenuAction::OverlayNone => if self.overlay_mode == OverlayMode::None { "ON" } else { "" },
            MenuAction::OverlayPower => if self.overlay_mode == OverlayMode::Power { "ON" } else { "" },
            MenuAction::OverlayPollution => if self.overlay_mode == OverlayMode::Pollution { "ON" } else { "" },
            MenuAction::OverlayLandValue => if self.overlay_mode == OverlayMode::LandValue { "ON" } else { "" },
            MenuAction::OverlayCrime => if self.overlay_mode == OverlayMode::Crime { "ON" } else { "" },
            MenuAction::OverlayFireRisk => if self.overlay_mode == OverlayMode::FireRisk { "ON" } else { "" },
            _ => base.right,
        };
        let label = match base.action {
            MenuAction::SpeedPause => if self.paused { "Resume" } else { "Pause" },
            _ => base.label,
        };
        Some((label.to_string(), right.to_string(), base.action))
    }

    pub fn current_menu_action(&self) -> MenuAction {
        menu_rows(self.menu_selected)
            .get(self.menu_item_selected)
            .map(|row| row.action)
            .unwrap_or(MenuAction::None)
    }

    pub fn open_menu(&mut self, selected: usize) {
        self.menu_active = true;
        self.menu_selected = selected.min(MENU_TITLES.len().saturating_sub(1));
        self.menu_item_selected = self.menu_item_selected.min(self.menu_item_count(self.menu_selected).saturating_sub(1));
    }

    pub fn close_menu(&mut self) {
        self.menu_active = false;
        self.menu_item_selected = 0;
    }

    #[allow(dead_code)]
    pub fn menu_action_for(menu: usize, item: usize) -> MenuAction {
        menu_rows(menu)
            .get(item)
            .map(|row| row.action)
            .unwrap_or(MenuAction::None)
    }

    pub fn should_block_for_menu(&self, action: &crate::app::input::Action) -> bool {
        self.menu_active
            && matches!(
                action,
                crate::app::input::Action::MoveCursor(_, _)
                    | crate::app::input::Action::MouseClick { .. }
                    | crate::app::input::Action::MenuSelect
                    | crate::app::input::Action::MenuBack
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ClickArea;

    #[test]
    fn menu_action_mapping_matches_structure() {
        assert_eq!(InGameScreen::menu_action_for(0, 0), MenuAction::NewCity);
        assert_eq!(InGameScreen::menu_action_for(1, 3), MenuAction::SpeedFast);
        assert_eq!(InGameScreen::menu_action_for(2, 1), MenuAction::DisasterFlood);
        assert_eq!(InGameScreen::menu_action_for(3, 1), MenuAction::PowerGas);
        assert_eq!(InGameScreen::menu_action_for(4, 6), MenuAction::OverlayFireRisk);
        assert_eq!(InGameScreen::menu_action_for(99, 99), MenuAction::None);
    }

    #[test]
    fn first_menu_click_opens_selected_menu() {
        let mut screen = InGameScreen::new();
        screen.ui_areas.menu_items[1] = ClickArea { x: 12, y: 0, width: 8, height: 1 };

        let engine = std::sync::Arc::new(std::sync::RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(10, 10),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = None;
        let mut running = true;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };

        let (consumed, transition) = screen.handle_menu_click(13, 0, &context);
        assert!(consumed);
        assert!(transition.is_none());
        assert!(screen.menu_active);
        assert_eq!(screen.menu_selected, 1);
        assert_eq!(screen.menu_item_selected, 0);
    }
}
