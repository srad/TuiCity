use crate::{
    app::{save, Tool},
    core::{
        engine::EngineCommand,
        sim::SimState,
    },
    ui::theme::OverlayMode,
};
use rat_widget::menu::{MenuBuilder, MenuItem, MenuStructure, MenubarState, Separator};

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
pub struct InGameMenu {
    pub paused: bool,
    pub ticks_per_month: u32,
    pub fire_enabled: bool,
    pub flood_enabled: bool,
    pub tornado_enabled: bool,
    pub overlay_mode: OverlayMode,
}

impl InGameMenu {
    pub fn from_screen(screen: &InGameScreen, sim: &SimState) -> Self {
        Self {
            paused: screen.paused,
            ticks_per_month: screen.ticks_per_month,
            fire_enabled: sim.disasters.fire_enabled,
            flood_enabled: sim.disasters.flood_enabled,
            tornado_enabled: sim.disasters.tornado_enabled,
            overlay_mode: screen.overlay_mode,
        }
    }

    fn speed_tag(self) -> &'static str {
        match self.ticks_per_month {
            0..=30 => "Fast",
            31..=70 => "Normal",
            _ => "Slow",
        }
    }

    fn pause_label(self) -> &'static str {
        if self.paused { "Resume" } else { "Pause" }
    }
}

fn nav_item(text: impl Into<String>, nav_char: char) -> MenuItem<'static> {
    let text = text.into();
    let lower = nav_char.to_ascii_lowercase();
    let start = text
        .char_indices()
        .find_map(|(idx, ch)| (ch.to_ascii_lowercase() == lower).then_some(idx))
        .unwrap_or(0);
    let end = text[start..]
        .chars()
        .next()
        .map(|ch| start + ch.len_utf8())
        .unwrap_or(start);

    let mut marked = String::with_capacity(text.len() + 1);
    marked.push_str(&text[..start]);
    marked.push('_');
    marked.push_str(&text[start..]);

    MenuItem::new_nav_string(marked, (start + 1)..(end + 1), nav_char)
}

fn nav_item_right(
    text: impl Into<String>,
    nav_char: char,
    right: impl Into<String>,
) -> MenuItem<'static> {
    let mut item = nav_item(text, nav_char);
    item.right = right.into().into();
    item
}

impl<'a> MenuStructure<'a> for InGameMenu {
    fn menus(&'a self, menu: &mut MenuBuilder<'a>) {
        menu.item(nav_item("System ", 's'));
        menu.item(nav_item(format!("Speed {} ", self.speed_tag()), 's'));
        menu.item(nav_item("Disasters ", 'd'));
        menu.item(nav_item("Power ", 'p'));
        menu.item(nav_item("Windows ", 'w'));
    }

    fn submenu(&'a self, n: usize, submenu: &mut MenuBuilder<'a>) {
        match n {
            0 => {
                submenu.item(nav_item_right("New City", 'n', "Alt+N"));
                submenu.item(nav_item_right("Save City", 's', "^S"));
                submenu.separator(Separator::Plain);
                submenu.item(nav_item_right("Quit", 'q', "Q"));
            }
            1 => {
                submenu.item(nav_item_right(
                    self.pause_label(),
                    if self.paused { 'r' } else { 'p' },
                    "Space",
                ));
                submenu.separator(Separator::Plain);
                submenu.item(nav_item_right("Slow", 's', "100"));
                submenu.item(nav_item_right("Normal", 'n', "50"));
                submenu.item(nav_item_right("Fast", 'f', "20"));
            }
            2 => {
                submenu.item(nav_item_right("Fire", 'f', if self.fire_enabled { "ON" } else { "OFF" }));
                submenu.item(nav_item_right("Flood", 'l', if self.flood_enabled { "ON" } else { "OFF" }));
                submenu.item(nav_item_right("Tornado", 't', if self.tornado_enabled { "ON" } else { "OFF" }));
            }
            3 => {
                submenu.item(nav_item_right("Coal Plant", 'c', "$3000"));
                submenu.item(nav_item_right("Gas Plant", 'g', "$6000"));
            }
            4 => {
                submenu.item(nav_item_right("Budget & Taxes", 'b', "$"));
                submenu.separator(Separator::Plain);
                submenu.item(nav_item_right("Overlay: Off", 'o', if self.overlay_mode == OverlayMode::None { "ON" } else { "" }));
                submenu.item(nav_item_right("Overlay: Power", 'p', if self.overlay_mode == OverlayMode::Power { "ON" } else { "" }));
                submenu.item(nav_item_right("Overlay: Pollution", 'l', if self.overlay_mode == OverlayMode::Pollution { "ON" } else { "" }));
                submenu.item(nav_item_right("Overlay: Land Value", 'v', if self.overlay_mode == OverlayMode::LandValue { "ON" } else { "" }));
                submenu.item(nav_item_right("Overlay: Crime", 'c', if self.overlay_mode == OverlayMode::Crime { "ON" } else { "" }));
                submenu.item(nav_item_right("Overlay: Fire Risk", 'r', if self.overlay_mode == OverlayMode::FireRisk { "ON" } else { "" }));
            }
            _ => {}
        }
    }
}

impl InGameScreen {
    pub fn new_menu_state() -> MenubarState {
        MenubarState::default()
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
                self.show_plant_info = false;
            }
            MenuAction::PowerGas => {
                self.current_tool = Tool::PowerPlantGas;
                self.show_plant_info = false;
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

    pub fn menu_action_for(menu: usize, item: usize) -> MenuAction {
        match (menu, item) {
            (0, 0) => MenuAction::NewCity,
            (0, 1) => MenuAction::SaveCity,
            (0, 2) => MenuAction::Quit,
            (1, 0) => MenuAction::SpeedPause,
            (1, 1) => MenuAction::SpeedSlow,
            (1, 2) => MenuAction::SpeedNormal,
            (1, 3) => MenuAction::SpeedFast,
            (2, 0) => MenuAction::DisasterFire,
            (2, 1) => MenuAction::DisasterFlood,
            (2, 2) => MenuAction::DisasterTornado,
            (3, 0) => MenuAction::PowerCoal,
            (3, 1) => MenuAction::PowerGas,
            (4, 0) => MenuAction::OpenBudget,
            (4, 1) => MenuAction::OverlayNone,
            (4, 2) => MenuAction::OverlayPower,
            (4, 3) => MenuAction::OverlayPollution,
            (4, 4) => MenuAction::OverlayLandValue,
            (4, 5) => MenuAction::OverlayCrime,
            (4, 6) => MenuAction::OverlayFireRisk,
            _ => MenuAction::None,
        }
    }

    pub fn menu_outcome_consumed(outcome: rat_widget::event::MenuOutcome) -> bool {
        !matches!(outcome, rat_widget::event::MenuOutcome::Continue)
    }

    pub fn open_menu(&mut self, selected: usize) {
        self.menu_active = true;
        self.menu.bar.focus.set(true);
        self.menu.bar.select(Some(selected));
        self.menu.popup.select(None);
        self.menu.set_popup_active(true);
    }

    pub fn close_menu(&mut self) {
        self.menu_active = false;
        self.menu.bar.focus.set(false);
        self.menu.set_popup_active(false);
        self.menu.popup.select(None);
    }

    pub fn take_menu_consumed_input(&mut self) -> bool {
        std::mem::take(&mut self.menu_consumed_input)
    }

    pub fn should_block_for_menu(&self, action: &crate::app::input::Action) -> bool {
        self.menu_active
            && matches!(
                action,
                crate::app::input::Action::MoveCursor(_, _)
                    | crate::app::input::Action::PanCamera(_, _)
                    | crate::app::input::Action::MouseClick { .. }
                    | crate::app::input::Action::MouseDrag { .. }
                    | crate::app::input::Action::MouseUp { .. }
                    | crate::app::input::Action::MouseMiddleDown { .. }
                    | crate::app::input::Action::MouseMiddleDrag { .. }
                    | crate::app::input::Action::MouseMiddleUp
                    | crate::app::input::Action::MouseMove { .. }
                    | crate::app::input::Action::MenuSelect
                    | crate::app::input::Action::MenuBack
                    | crate::app::input::Action::CharInput(_)
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_item_marks_first_character_safely() {
        let item = nav_item("System", 's');
        assert_eq!(item.item.as_ref(), "_System");
        assert_eq!(item.highlight, Some(1..2));
        assert_eq!(item.navchar, Some('s'));
    }

    #[test]
    fn menu_action_mapping_matches_structure() {
        assert_eq!(InGameScreen::menu_action_for(0, 0), MenuAction::NewCity);
        assert_eq!(InGameScreen::menu_action_for(1, 3), MenuAction::SpeedFast);
        assert_eq!(InGameScreen::menu_action_for(2, 1), MenuAction::DisasterFlood);
        assert_eq!(InGameScreen::menu_action_for(3, 1), MenuAction::PowerGas);
        assert_eq!(InGameScreen::menu_action_for(4, 6), MenuAction::OverlayFireRisk);
        assert_eq!(InGameScreen::menu_action_for(99, 99), MenuAction::None);
    }
}
