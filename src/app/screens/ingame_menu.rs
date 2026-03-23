use crate::{
    app::{save, WindowId},
    core::engine::EngineCommand,
    ui::theme::OverlayMode,
};

use super::{AppContext, InGameScreen, ScreenTransition, SettingsScreen};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuAction {
    #[default]
    None,
    NewCity,
    LoadCity,
    SaveCity,
    ToggleMusic,
    Quit,
    SpeedPause,
    SpeedSlow,
    SpeedNormal,
    SpeedFast,
    DisasterFire,
    DisasterFlood,
    DisasterTornado,
    ToggleToolbar,
    ToggleInspect,
    OpenBudget,
    OpenStatistics,
    ToggleLegend,
    ToggleLayer,
    OverlayNone,
    OverlayPower,
    OverlayWater,
    OverlayTraffic,
    OverlayPollution,
    OverlayLandValue,
    OverlayCrime,
    OverlayFireRisk,
    OpenSettings,
    OpenAdvisor,
    ToggleNewspaper,
    OpenHelp,
    OpenAbout,
}

#[derive(Debug, Clone, Copy)]
pub struct MenuRow {
    pub label: &'static str,
    pub right: &'static str,
    pub action: MenuAction,
}

pub const MENU_TITLES: [&str; 6] = ["File", "Speed", "Disasters", "Windows", "Help", "About"];

const FILE_ROWS: [MenuRow; 6] = [
    MenuRow {
        label: "New City",
        right: "Alt+N",
        action: MenuAction::NewCity,
    },
    MenuRow {
        label: "Load City",
        right: "",
        action: MenuAction::LoadCity,
    },
    MenuRow {
        label: "Save City",
        right: "^S",
        action: MenuAction::SaveCity,
    },
    MenuRow {
        label: "Settings",
        right: "Shift+P",
        action: MenuAction::OpenSettings,
    },
    MenuRow {
        label: "Toggle Music",
        right: "M",
        action: MenuAction::ToggleMusic,
    },
    MenuRow {
        label: "Quit",
        right: "Q",
        action: MenuAction::Quit,
    },
];
const SPEED_ROWS: [MenuRow; 4] = [
    MenuRow {
        label: "Pause / Resume",
        right: "Space",
        action: MenuAction::SpeedPause,
    },
    MenuRow {
        label: "Slow",
        right: "100",
        action: MenuAction::SpeedSlow,
    },
    MenuRow {
        label: "Normal",
        right: "50",
        action: MenuAction::SpeedNormal,
    },
    MenuRow {
        label: "Fast",
        right: "20",
        action: MenuAction::SpeedFast,
    },
];
const DISASTER_ROWS: [MenuRow; 3] = [
    MenuRow {
        label: "Fire",
        right: "",
        action: MenuAction::DisasterFire,
    },
    MenuRow {
        label: "Flood",
        right: "",
        action: MenuAction::DisasterFlood,
    },
    MenuRow {
        label: "Tornado",
        right: "",
        action: MenuAction::DisasterTornado,
    },
];
const WINDOWS_ROWS: [MenuRow; 16] = [
    MenuRow {
        label: "Toolbox",
        right: "",
        action: MenuAction::ToggleToolbar,
    },
    MenuRow {
        label: "Inspect",
        right: "",
        action: MenuAction::ToggleInspect,
    },
    MenuRow {
        label: "Budget & Taxes",
        right: "$",
        action: MenuAction::OpenBudget,
    },
    MenuRow {
        label: "Statistics",
        right: "",
        action: MenuAction::OpenStatistics,
    },
    MenuRow {
        label: "Advisors",
        right: "A",
        action: MenuAction::OpenAdvisor,
    },
    MenuRow {
        label: "Newspaper",
        right: "N",
        action: MenuAction::ToggleNewspaper,
    },
    MenuRow {
        label: "Map Legend",
        right: "",
        action: MenuAction::ToggleLegend,
    },
    MenuRow {
        label: "View Layer",
        right: "U",
        action: MenuAction::ToggleLayer,
    },
    MenuRow {
        label: "Overlay: Off",
        right: "",
        action: MenuAction::OverlayNone,
    },
    MenuRow {
        label: "Overlay: Power",
        right: "",
        action: MenuAction::OverlayPower,
    },
    MenuRow {
        label: "Overlay: Water Service",
        right: "",
        action: MenuAction::OverlayWater,
    },
    MenuRow {
        label: "Overlay: Traffic",
        right: "",
        action: MenuAction::OverlayTraffic,
    },
    MenuRow {
        label: "Overlay: Pollution",
        right: "",
        action: MenuAction::OverlayPollution,
    },
    MenuRow {
        label: "Overlay: Land Value",
        right: "",
        action: MenuAction::OverlayLandValue,
    },
    MenuRow {
        label: "Overlay: Crime",
        right: "",
        action: MenuAction::OverlayCrime,
    },
    MenuRow {
        label: "Overlay: Fire Risk",
        right: "",
        action: MenuAction::OverlayFireRisk,
    },
];

pub fn menu_rows(menu: usize) -> &'static [MenuRow] {
    match menu {
        0 => &FILE_ROWS,
        1 => &SPEED_ROWS,
        2 => &DISASTER_ROWS,
        3 => &WINDOWS_ROWS,
        _ => &[],
    }
}

fn direct_menu_action(menu: usize) -> Option<MenuAction> {
    match menu {
        4 => Some(MenuAction::OpenHelp),
        5 => Some(MenuAction::OpenAbout),
        _ => None,
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
                if let Some(action) = direct_menu_action(idx) {
                    self.close_menu();
                    return (true, self.handle_menu_action(action, context));
                }
                if self.menu_active && self.menu_selected == idx {
                    self.close_menu();
                } else {
                    self.open_menu(idx);
                }
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

            if self.ui_areas.menu_popup.contains(col, row)
                || self.ui_areas.menu_bar.contains(col, row)
            {
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
            MenuAction::NewCity => {
                self.open_confirm_prompt(super::ingame::ConfirmPromptAction::ReturnToStart);
            }
            MenuAction::LoadCity => {
                self.open_confirm_prompt(super::ingame::ConfirmPromptAction::LoadCity);
            }
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
            MenuAction::Quit => {
                self.open_confirm_prompt(super::ingame::ConfirmPromptAction::Quit);
            }
            MenuAction::ToggleMusic => {
                let current = crate::app::config::is_music_enabled();
                let _ = crate::app::config::persist_music_preference(!current);
            }
            MenuAction::OpenSettings => {
                return Some(ScreenTransition::Push(Box::new(SettingsScreen::new())));
            }
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
            MenuAction::ToggleToolbar => {
                self.close_tool_chooser();
                self.desktop.open(WindowId::Panel, false);
                self.desktop.focus(WindowId::Panel);
            }
            MenuAction::ToggleInspect => self.toggle_inspect_window(),
            MenuAction::OpenBudget => self.open_budget(context),
            MenuAction::OpenStatistics => self.open_stats_window(),
            MenuAction::OpenAdvisor => self.toggle_advisor_window(),
            MenuAction::ToggleNewspaper => self.toggle_newspaper_window(context),
            MenuAction::ToggleLegend => self.toggle_legend_window(),
            MenuAction::ToggleLayer => self.toggle_view_layer(),
            MenuAction::OverlayNone => self.overlay_mode = OverlayMode::None,
            MenuAction::OverlayPower => self.overlay_mode = OverlayMode::Power,
            MenuAction::OverlayWater => self.overlay_mode = OverlayMode::Water,
            MenuAction::OverlayTraffic => self.overlay_mode = OverlayMode::Traffic,
            MenuAction::OverlayPollution => self.overlay_mode = OverlayMode::Pollution,
            MenuAction::OverlayLandValue => self.overlay_mode = OverlayMode::LandValue,
            MenuAction::OverlayCrime => self.overlay_mode = OverlayMode::Crime,
            MenuAction::OverlayFireRisk => self.overlay_mode = OverlayMode::FireRisk,
            MenuAction::OpenHelp => self.open_help_window(),
            MenuAction::OpenAbout => self.open_about_window(),
        }
        None
    }

    pub fn menu_item_count(&self, menu: usize) -> usize {
        menu_rows(menu).len()
    }

    pub fn current_menu_action(&self) -> MenuAction {
        if let Some(action) = direct_menu_action(self.menu_selected) {
            return action;
        }
        menu_rows(self.menu_selected)
            .get(self.menu_item_selected)
            .map(|row| row.action)
            .unwrap_or(MenuAction::None)
    }

    pub fn open_menu(&mut self, selected: usize) {
        self.menu_active = true;
        self.menu_selected = selected.min(MENU_TITLES.len().saturating_sub(1));
        self.menu_item_selected = self
            .menu_item_selected
            .min(self.menu_item_count(self.menu_selected).saturating_sub(1));
    }

    pub fn close_menu(&mut self) {
        self.menu_active = false;
        self.menu_item_selected = 0;
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
impl InGameScreen {
    pub fn menu_row(
        &self,
        menu: usize,
        item: usize,
        sim: &crate::core::sim::SimState,
    ) -> Option<(String, String, MenuAction)> {
        let base = *menu_rows(menu).get(item)?;
        let right = match base.action {
            MenuAction::DisasterFire => {
                if sim.disasters.fire_enabled {
                    "ON"
                } else {
                    "OFF"
                }
            }
            MenuAction::DisasterFlood => {
                if sim.disasters.flood_enabled {
                    "ON"
                } else {
                    "OFF"
                }
            }
            MenuAction::DisasterTornado => {
                if sim.disasters.tornado_enabled {
                    "ON"
                } else {
                    "OFF"
                }
            }
            MenuAction::OverlayNone => {
                if self.overlay_mode == OverlayMode::None {
                    "ON"
                } else {
                    ""
                }
            }
            MenuAction::OverlayPower => {
                if self.overlay_mode == OverlayMode::Power {
                    "ON"
                } else {
                    ""
                }
            }
            MenuAction::OverlayWater => {
                if self.overlay_mode == OverlayMode::Water {
                    "ON"
                } else {
                    ""
                }
            }
            MenuAction::OverlayTraffic => {
                if self.overlay_mode == OverlayMode::Traffic {
                    "ON"
                } else {
                    ""
                }
            }
            MenuAction::OverlayPollution => {
                if self.overlay_mode == OverlayMode::Pollution {
                    "ON"
                } else {
                    ""
                }
            }
            MenuAction::OverlayLandValue => {
                if self.overlay_mode == OverlayMode::LandValue {
                    "ON"
                } else {
                    ""
                }
            }
            MenuAction::OverlayCrime => {
                if self.overlay_mode == OverlayMode::Crime {
                    "ON"
                } else {
                    ""
                }
            }
            MenuAction::OverlayFireRisk => {
                if self.overlay_mode == OverlayMode::FireRisk {
                    "ON"
                } else {
                    ""
                }
            }
            MenuAction::ToggleToolbar => {
                if self.desktop.is_open(WindowId::Panel) {
                    "ON"
                } else {
                    "OFF"
                }
            }
            MenuAction::ToggleInspect => {
                if self.desktop.is_open(WindowId::Inspect) {
                    "ON"
                } else {
                    "OFF"
                }
            }
            MenuAction::OpenStatistics => {
                if self.desktop.is_open(WindowId::Statistics) {
                    "ON"
                } else {
                    "OFF"
                }
            }
            MenuAction::ToggleLegend => {
                if self.desktop.is_open(WindowId::Legend) {
                    "ON"
                } else {
                    "OFF"
                }
            }
            MenuAction::ToggleLayer => InGameScreen::view_layer_label(self.view_layer),
            _ => base.right,
        };
        let label = match base.action {
            MenuAction::SpeedPause => {
                if self.paused {
                    "Resume"
                } else {
                    "Pause"
                }
            }
            _ => base.label,
        };
        Some((label.to_string(), right.to_string(), base.action))
    }

    pub fn menu_action_for(menu: usize, item: usize) -> MenuAction {
        menu_rows(menu)
            .get(item)
            .map(|row| row.action)
            .unwrap_or(MenuAction::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ClickArea;
    use crate::core::sim::SimState;

    #[test]
    fn menu_action_mapping_matches_structure() {
        assert_eq!(InGameScreen::menu_action_for(0, 0), MenuAction::NewCity);
        assert_eq!(InGameScreen::menu_action_for(0, 1), MenuAction::LoadCity);
        assert_eq!(InGameScreen::menu_action_for(1, 3), MenuAction::SpeedFast);
        assert_eq!(
            InGameScreen::menu_action_for(2, 1),
            MenuAction::DisasterFlood
        );
        assert_eq!(
            InGameScreen::menu_action_for(3, 0),
            MenuAction::ToggleToolbar
        );
        assert_eq!(
            InGameScreen::menu_action_for(3, 1),
            MenuAction::ToggleInspect
        );
        assert_eq!(InGameScreen::menu_action_for(3, 4), MenuAction::OpenAdvisor);
        assert_eq!(
            InGameScreen::menu_action_for(3, 5),
            MenuAction::ToggleNewspaper
        );
        assert_eq!(
            InGameScreen::menu_action_for(3, 15),
            MenuAction::OverlayFireRisk
        );
        assert_eq!(MENU_TITLES[4], "Help");
        assert_eq!(MENU_TITLES[5], "About");
        assert_eq!(InGameScreen::menu_action_for(99, 99), MenuAction::None);
    }

    #[test]
    fn first_menu_click_opens_selected_menu() {
        let mut screen = InGameScreen::new();
        screen.ui_areas.menu_items[1] = ClickArea {
            x: 12,
            y: 0,
            width: 8,
            height: 1,
        };

        let engine = std::sync::Arc::new(std::sync::RwLock::new(
            crate::core::engine::SimulationEngine::new(
                crate::core::map::Map::new(10, 10),
                crate::core::sim::SimState::default(),
            ),
        ));
        let cmd_tx = None;
        let tg = crate::textgen::TextGenService::start(std::path::PathBuf::from("/nonexistent"));

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &tg,
        };

        let (consumed, transition) = screen.handle_menu_click(13, 0, &context);
        assert!(consumed);
        assert!(transition.is_none());
        assert!(screen.menu_active);
        assert_eq!(screen.menu_selected, 1);
        assert_eq!(screen.menu_item_selected, 0);
    }

    #[test]
    fn direct_help_menu_click_opens_help_window() {
        let mut screen = InGameScreen::new();
        screen.ui_areas.menu_items[4] = ClickArea {
            x: 70,
            y: 0,
            width: 6,
            height: 1,
        };

        let engine = std::sync::Arc::new(std::sync::RwLock::new(
            crate::core::engine::SimulationEngine::new(
                crate::core::map::Map::new(10, 10),
                crate::core::sim::SimState::default(),
            ),
        ));
        let cmd_tx = None;
        let tg = crate::textgen::TextGenService::start(std::path::PathBuf::from("/nonexistent"));

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &tg,
        };

        let (consumed, transition) = screen.handle_menu_click(71, 0, &context);
        assert!(consumed);
        assert!(transition.is_none());
        assert!(screen.is_help_open());
        assert!(!screen.menu_active);
    }

    #[test]
    fn new_city_menu_action_opens_return_to_start_prompt() {
        let mut screen = InGameScreen::new();
        let engine = std::sync::Arc::new(std::sync::RwLock::new(
            crate::core::engine::SimulationEngine::new(
                crate::core::map::Map::new(10, 10),
                crate::core::sim::SimState::default(),
            ),
        ));
        let cmd_tx = None;
        let tg = crate::textgen::TextGenService::start(std::path::PathBuf::from("/nonexistent"));

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &tg,
        };

        let transition = screen.handle_menu_action(MenuAction::NewCity, &context);

        assert!(transition.is_none());
        assert!(screen.is_confirm_prompt_open());
    }

    #[test]
    fn windows_menu_uses_clear_layer_and_water_service_labels() {
        let screen = InGameScreen::new();
        let sim = SimState::default();

        let (layer_label, layer_value, _) = screen
            .menu_row(3, 7, &sim)
            .expect("windows layer row should exist");
        let (water_label, _, _) = screen
            .menu_row(3, 10, &sim)
            .expect("windows water overlay row should exist");

        assert_eq!(layer_label, "View Layer");
        assert_eq!(layer_value, "Surface");
        assert_eq!(water_label, "Overlay: Water Service");
    }

    #[test]
    fn toolbox_menu_action_keeps_panel_open() {
        let mut screen = InGameScreen::new();
        screen.desktop.close(WindowId::Panel);
        let engine = std::sync::Arc::new(std::sync::RwLock::new(
            crate::core::engine::SimulationEngine::new(
                crate::core::map::Map::new(10, 10),
                crate::core::sim::SimState::default(),
            ),
        ));
        let cmd_tx = None;
        let tg = crate::textgen::TextGenService::start(std::path::PathBuf::from("/nonexistent"));

        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            textgen: &tg,
        };

        let transition = screen.handle_menu_action(MenuAction::ToggleToolbar, &context);

        assert!(transition.is_none());
        assert!(screen.desktop.is_open(WindowId::Panel));
    }
}
