#![allow(unused_imports)]
use std::collections::VecDeque;

use crate::{
    app::{
        config, input::Action, save, Camera, DesktopState, LineDrag, RectDrag, Tool, UiAreas,
        UiRect, WindowId,
    },
    core::engine::EngineCommand,
    game_info::GAME_NAME,
    ui::{
        runtime::ToolChooserKind,
        theme::{self, OverlayMode},
        view::{
            BudgetViewModel, ConfirmDialogButtonRole, ConfirmDialogButtonViewModel,
            ConfirmDialogViewModel, InGameDesktopView, StatisticsWindowViewModel,
            TextWindowViewModel, ToolChooserViewModel, ToolbarPaletteViewModel,
        },
    },
};

use super::{
    confirm_dialog, ingame_budget::BudgetState, ingame_news::CityNewsState, AppContext,
    LoadCityScreen, Screen, ScreenTransition,
};

/// Ticks between auto-saves (50 ticks/month × 12 months × 6 months ≈ every 6 in-game months).
pub const AUTO_SAVE_INTERVAL: u32 = 50 * 12 * 6;

fn build_help_lines() -> Vec<String> {
    vec![
        "Goal".to_string(),
        "".to_string(),
        "Grow a solvent city by giving zones three things: road access, power, and demand."
            .to_string(),
        "".to_string(),
        "Getting started".to_string(),
        "".to_string(),
        "1. Lay a short road spine before zoning.".to_string(),
        "2. Place a power plant and connect it with power lines.".to_string(),
        "3. Zone mostly residential, then add a smaller amount of commercial and industrial."
            .to_string(),
        "4. Unpause and let buildings grow before spending heavily on extras.".to_string(),
        "".to_string(),
        "What makes a city work".to_string(),
        "".to_string(),
        "Residential zones need nearby jobs.".to_string(),
        "Commercial and industrial zones need residents to staff them.".to_string(),
        "Unpowered or disconnected zones will stay empty.".to_string(),
        "Too much industry raises pollution and drags land value down.".to_string(),
        "".to_string(),
        "A safe early layout".to_string(),
        "".to_string(),
        "Use a simple road grid.".to_string(),
        "Keep industry separated from homes.".to_string(),
        "Put commercial between residential and industry or near main roads.".to_string(),
        "Add parks to lift land value around housing.".to_string(),
        "".to_string(),
        "Money and survival".to_string(),
        "".to_string(),
        "Income comes from developed buildings, not empty zones.".to_string(),
        "Do not overbuild roads, rails, or services before tax income arrives.".to_string(),
        "Check the budget window if treasury keeps falling.".to_string(),
        "".to_string(),
        "Disasters and services".to_string(),
        "".to_string(),
        "Fire departments reduce fire risk.".to_string(),
        "Police stations reduce crime.".to_string(),
        "Keep some cash in reserve so one bad month does not stall the city.".to_string(),
        "".to_string(),
        "Useful controls".to_string(),
        "".to_string(),
        "Left click the minimap to jump the camera.".to_string(),
        "Use the toolbox for zoning and infrastructure.".to_string(),
        "Press Ctrl+S to save, and use File to load another city.".to_string(),
    ]
}

fn build_legend_lines() -> Vec<String> {
    let mut lines = Vec::new();
    let overlay = crate::core::map::TileOverlay::default();

    let mut push_legend = |label: &str, tiles: &[crate::core::map::Tile]| {
        let mut sprites = String::new();
        for &tile in tiles {
            let sprite = crate::ui::theme::tile_sprite(tile, overlay);
            sprites.push(sprite.left.ch);
            sprites.push(sprite.right.ch);
            sprites.push(' ');
        }
        lines.push(format!("  {}- {}", sprites, label));
    };

    use crate::core::map::Tile;
    push_legend(
        "Empty Zones (Res, Comm, Ind)",
        &[Tile::ZoneRes, Tile::ZoneComm, Tile::ZoneInd],
    );
    push_legend(
        "Residential (Low, Med, High)",
        &[Tile::ResLow, Tile::ResMed, Tile::ResHigh],
    );
    push_legend("Commercial (Low, High)", &[Tile::CommLow, Tile::CommHigh]);
    push_legend(
        "Industrial (Light, Heavy)",
        &[Tile::IndLight, Tile::IndHeavy],
    );
    push_legend("Power Plant", &[Tile::PowerPlantCoal]);
    push_legend("Police Station", &[Tile::Police]);
    push_legend("Fire Station", &[Tile::Fire]);
    push_legend("Hospital", &[Tile::Hospital]);
    push_legend("Park", &[Tile::Park]);
    push_legend(
        "Water (Pump, Tower, Treatment, Desalination)",
        &[
            Tile::WaterPump,
            Tile::WaterTower,
            Tile::WaterTreatment,
            Tile::Desalination,
        ],
    );
    push_legend(
        "Transit (Bus, Rail, Subway)",
        &[Tile::BusDepot, Tile::RailDepot, Tile::SubwayStation],
    );

    lines
}

const ABOUT_LINES: &[&str] = &[
    GAME_NAME,
    "Terminal city-building simulation in Rust.",
    "Author: Saman Sedighi Rad",
    "Website: https://sedrad.com",
    "GitHub: https://github.com/srad",
];

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

#[derive(Debug, Clone, Copy)]
pub(super) struct WindowScrollbarDrag {
    pub(super) window_id: WindowId,
    pub(super) grab_offset: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfirmPromptAction {
    Quit,
    LoadCity,
    ReturnToStart,
}

pub struct InGameScreen {
    pub camera: Camera,
    pub current_tool: Tool,
    pub open_tool_chooser: Option<ToolChooserKind>,
    pub zone_tool: Tool,
    pub transport_tool: Tool,
    pub utility_tool: Tool,
    pub power_plant_tool: Tool,
    pub building_tool: Tool,
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
    pub view_layer: crate::core::map::ViewLayer,
    pub inspect_pos: Option<(usize, usize)>,
    confirm_prompt_action: Option<ConfirmPromptAction>,
    confirm_prompt_selected: usize,
    pub ticks_since_save: u32,
    pub auto_save_interval_ticks: u32,
    pub first_building_notified: bool,
    pub deficit_warned: bool,
    pub desktop: DesktopState,
    pub(super) map_pan_drag: Option<MiddlePanDrag>,
    pub(super) scrollbar_drag: Option<ScrollbarDrag>,
    pub(super) window_scrollbar_drag: Option<WindowScrollbarDrag>,
    pub budget_ui: BudgetState,
    pub news_ticker: CityNewsState,
}

impl InGameScreen {
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            current_tool: Tool::Inspect,
            open_tool_chooser: None,
            zone_tool: Tool::ZoneResLight,
            transport_tool: Tool::Road,
            utility_tool: Tool::PowerLine,
            power_plant_tool: Tool::PowerPlantCoal,
            building_tool: Tool::Police,
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
            view_layer: crate::core::map::ViewLayer::Surface,
            inspect_pos: None,
            confirm_prompt_action: None,
            confirm_prompt_selected: 0,
            ticks_since_save: 0,
            auto_save_interval_ticks: AUTO_SAVE_INTERVAL,
            first_building_notified: false,
            deficit_warned: false,
            desktop: DesktopState::new_ingame(),
            map_pan_drag: None,
            scrollbar_drag: None,
            window_scrollbar_drag: None,
            budget_ui: BudgetState::new(),
            news_ticker: CityNewsState::default(),
        }
    }

    pub fn is_over_window(&self, col: u16, row: u16) -> bool {
        self.desktop.contains(WindowId::Panel, col, row)
            || self.desktop.contains(WindowId::Budget, col, row)
            || self.desktop.contains(WindowId::Statistics, col, row)
            || self.desktop.contains(WindowId::Inspect, col, row)
            || self.desktop.contains(WindowId::PowerPicker, col, row)
            || self.desktop.contains(WindowId::Help, col, row)
            || self.desktop.contains(WindowId::About, col, row)
            || self.desktop.contains(WindowId::Legend, col, row)
    }

    pub(super) fn window_content_height(&self, id: WindowId) -> u16 {
        match id {
            WindowId::Help => build_help_lines().len() as u16,
            WindowId::About => ABOUT_LINES.len() as u16,
            WindowId::Legend => build_legend_lines().len() as u16,
            _ => 0,
        }
    }

    pub(super) fn scroll_window(&mut self, id: WindowId, delta: i32) {
        let content_h = self.window_content_height(id);
        let win = self.desktop.window_mut(id);
        let view_h = win.height.saturating_sub(4); // 2 borders + 2 padding
        let max_scroll = content_h.saturating_sub(view_h);

        if delta < 0 {
            win.scroll_y = win.scroll_y.saturating_sub(delta.unsigned_abs() as u16);
        } else {
            win.scroll_y = (win.scroll_y + delta as u16).min(max_scroll);
        }
    }

    pub fn push_message(&mut self, text: String) {
        self.event_messages.push_back((text, 80));
        if self.event_messages.len() > 5 {
            self.event_messages.pop_front();
        }
        self.news_ticker.mark_dirty();
    }

    pub(crate) fn view_layer_label(layer: crate::core::map::ViewLayer) -> &'static str {
        match layer {
            crate::core::map::ViewLayer::Surface => "Surface",
            crate::core::map::ViewLayer::Underground => "Underground",
        }
    }

    pub(crate) fn switch_view_layer(
        &mut self,
        layer: crate::core::map::ViewLayer,
        notice: Option<String>,
    ) {
        if self.view_layer == layer {
            return;
        }
        self.view_layer = layer;
        if let Some(notice) = notice {
            self.push_message(notice);
        }
    }

    pub(crate) fn toggle_view_layer(&mut self) {
        let next = match self.view_layer {
            crate::core::map::ViewLayer::Surface => crate::core::map::ViewLayer::Underground,
            crate::core::map::ViewLayer::Underground => crate::core::map::ViewLayer::Surface,
        };
        self.switch_view_layer(
            next,
            Some(format!("View layer: {}", Self::view_layer_label(next))),
        );
    }

    pub fn status_message(&self) -> Option<&str> {
        if self.overlay_mode != OverlayMode::None {
            return Some(self.overlay_mode.label());
        }
        if let Some(ref message) = self.message {
            return Some(message.as_str());
        }
        self.event_messages
            .front()
            .map(|(message, _)| message.as_str())
    }

    pub fn line_preview(&self) -> &[(usize, usize)] {
        self.line_drag
            .as_ref()
            .map(|drag| drag.path.as_slice())
            .unwrap_or(&[])
    }

    pub fn rect_preview(&self) -> &[(usize, usize)] {
        self.rect_drag
            .as_ref()
            .map(|drag| drag.tiles_cache.as_slice())
            .unwrap_or(&[])
    }

    pub fn is_budget_open(&self) -> bool {
        self.desktop.is_open(WindowId::Budget)
    }

    pub fn is_tool_chooser_open(&self) -> bool {
        self.desktop.is_open(WindowId::PowerPicker)
    }

    pub fn is_stats_open(&self) -> bool {
        self.desktop.is_open(WindowId::Statistics)
    }

    pub fn is_inspect_open(&self) -> bool {
        self.desktop.is_open(WindowId::Inspect)
    }

    pub fn is_help_open(&self) -> bool {
        self.desktop.is_open(WindowId::Help)
    }

    pub fn is_about_open(&self) -> bool {
        self.desktop.is_open(WindowId::About)
    }

    pub fn is_legend_open(&self) -> bool {
        self.desktop.is_open(WindowId::Legend)
    }

    #[allow(dead_code)]
    pub fn is_confirm_prompt_open(&self) -> bool {
        self.confirm_prompt_action.is_some()
    }

    pub fn open_inspect_window(&mut self) {
        if self.inspect_pos.is_none() {
            self.inspect_pos = Some((self.camera.cursor_x, self.camera.cursor_y));
        }
        self.desktop.open(WindowId::Inspect, false);
    }

    pub fn close_inspect_window(&mut self) {
        self.desktop.close(WindowId::Inspect);
    }

    pub fn close_window(&mut self, id: WindowId) {
        if id == WindowId::PowerPicker {
            self.close_tool_chooser();
        } else {
            self.desktop.close(id);
        }
    }

    pub fn open_stats_window(&mut self) {
        self.desktop.close(WindowId::Help);
        self.desktop.close(WindowId::About);
        self.desktop.close(WindowId::Legend);
        self.desktop.open(WindowId::Statistics, true);
    }

    pub fn close_stats_window(&mut self) {
        self.desktop.close(WindowId::Statistics);
    }

    pub fn open_help_window(&mut self) {
        self.desktop.close(WindowId::Statistics);
        self.desktop.close(WindowId::About);
        self.desktop.close(WindowId::Legend);
        self.desktop.open(WindowId::Help, true);
    }

    pub fn close_help_window(&mut self) {
        self.desktop.close(WindowId::Help);
    }

    pub fn open_about_window(&mut self) {
        self.desktop.close(WindowId::Statistics);
        self.desktop.close(WindowId::Help);
        self.desktop.close(WindowId::Legend);
        self.desktop.open(WindowId::About, true);
    }

    pub fn close_about_window(&mut self) {
        self.desktop.close(WindowId::About);
    }

    pub fn open_legend_window(&mut self) {
        self.desktop.close(WindowId::Statistics);
        self.desktop.close(WindowId::Help);
        self.desktop.close(WindowId::About);
        self.desktop.open(WindowId::Legend, true);
    }

    pub fn close_legend_window(&mut self) {
        self.desktop.close(WindowId::Legend);
    }

    pub fn toggle_legend_window(&mut self) {
        if self.is_legend_open() {
            self.close_legend_window();
        } else {
            self.open_legend_window();
        }
    }

    pub fn toggle_inspect_window(&mut self) {
        if self.is_inspect_open() {
            self.close_inspect_window();
        } else {
            self.open_inspect_window();
        }
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
            toolbar: self.toolbar_view_model(),
            tool_chooser: self.tool_chooser_view_model(),
            confirm_dialog: self.confirm_dialog_view_model(),
            paused: self.paused,
            overlay_mode: self.overlay_mode,
            view_layer: self.view_layer,
            menu_active: self.menu_active,
            menu_selected: self.menu_selected,
            menu_item_selected: self.menu_item_selected,
            status_message: self.status_message().map(str::to_string),
            news_ticker: self.news_ticker.view_model(),
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
            statistics: self.statistics_view_model(sim),
            help: self.help_view_model(),
            about: self.about_view_model(),
            legend: self.legend_view_model(),
        }
    }

    fn statistics_view_model(
        &self,
        sim: &crate::core::sim::SimState,
    ) -> Option<StatisticsWindowViewModel> {
        self.is_stats_open().then(|| StatisticsWindowViewModel {
            city_name: sim.city_name.clone(),
            current_population: sim.pop.population,
            current_treasury: sim.economy.treasury,
            current_income: sim.economy.last_income,
            current_power_produced: sim.utilities.power_produced_mw,
            current_power_consumed: sim.utilities.power_consumed_mw,
            treasury_history: sim.history.treasury.clone(),
            population_history: sim.history.population.clone(),
            income_history: sim.history.income.clone(),
            power_balance_history: sim.history.power_balance.clone(),
        })
    }

    fn help_view_model(&self) -> Option<TextWindowViewModel> {
        self.is_help_open().then(|| TextWindowViewModel {
            lines: build_help_lines(),
            scroll_y: self.desktop.window(WindowId::Help).scroll_y,
        })
    }

    fn about_view_model(&self) -> Option<TextWindowViewModel> {
        self.is_about_open().then(|| TextWindowViewModel {
            lines: ABOUT_LINES.iter().map(|line| (*line).to_string()).collect(),
            scroll_y: self.desktop.window(WindowId::About).scroll_y,
        })
    }

    fn legend_view_model(&self) -> Option<TextWindowViewModel> {
        self.is_legend_open().then(|| TextWindowViewModel {
            lines: build_legend_lines(),
            scroll_y: self.desktop.window(WindowId::Legend).scroll_y,
        })
    }

    fn toolbar_view_model(&self) -> ToolbarPaletteViewModel {
        ToolbarPaletteViewModel {
            current_tool: self.current_tool,
            zone_tool: self.zone_tool,
            transport_tool: self.transport_tool,
            utility_tool: self.utility_tool,
            power_plant_tool: self.power_plant_tool,
            building_tool: self.building_tool,
            chooser: self.open_tool_chooser,
            view_layer: self.view_layer,
        }
    }

    fn tool_chooser_view_model(&self) -> Option<ToolChooserViewModel> {
        let kind = self.open_tool_chooser?;
        Some(ToolChooserViewModel {
            selected_tool: self.remembered_tool_for_chooser(kind),
            tools: kind.tools().to_vec(),
        })
    }

    fn confirm_dialog_view_model(&self) -> Option<ConfirmDialogViewModel> {
        let action = self.confirm_prompt_action?;
        let (title, message, accept_label, alternate_label) = match action {
            ConfirmPromptAction::Quit => (
                "Exit City".to_string(),
                "Save city before leaving?".to_string(),
                "Save And Quit".to_string(),
                "Quit Without Saving".to_string(),
            ),
            ConfirmPromptAction::LoadCity => (
                "Load City".to_string(),
                "Save current city before loading another?".to_string(),
                "Save And Load".to_string(),
                "Load Without Saving".to_string(),
            ),
            ConfirmPromptAction::ReturnToStart => (
                "Leave City".to_string(),
                "Save city before returning to the start screen?".to_string(),
                "Save And Leave".to_string(),
                "Leave Without Saving".to_string(),
            ),
        };
        let buttons = vec![
            ConfirmDialogButtonViewModel {
                label: accept_label,
                role: ConfirmDialogButtonRole::Accept,
            },
            ConfirmDialogButtonViewModel {
                label: alternate_label,
                role: ConfirmDialogButtonRole::Alternate,
            },
            ConfirmDialogButtonViewModel {
                label: "Cancel".to_string(),
                role: ConfirmDialogButtonRole::Cancel,
            },
        ];
        Some(ConfirmDialogViewModel {
            title,
            message,
            selected: self
                .confirm_prompt_selected
                .min(buttons.len().saturating_sub(1)),
            buttons,
        })
    }

    pub fn remembered_tool_for_chooser(&self, kind: ToolChooserKind) -> Tool {
        match kind {
            ToolChooserKind::Zones => self.zone_tool,
            ToolChooserKind::Transport => self.transport_tool,
            ToolChooserKind::Utilities => self.utility_tool,
            ToolChooserKind::PowerPlants => self.power_plant_tool,
            ToolChooserKind::Buildings => self.building_tool,
        }
    }

    fn set_chooser_tool(&mut self, kind: ToolChooserKind, tool: Tool) {
        match kind {
            ToolChooserKind::Zones => self.zone_tool = tool,
            ToolChooserKind::Transport => self.transport_tool = tool,
            ToolChooserKind::Utilities => self.utility_tool = tool,
            ToolChooserKind::PowerPlants => self.power_plant_tool = tool,
            ToolChooserKind::Buildings => self.building_tool = tool,
        }
    }

    pub fn close_tool_chooser(&mut self) {
        self.open_tool_chooser = None;
        self.desktop.close(WindowId::PowerPicker);
    }

    pub(super) fn open_confirm_prompt(&mut self, action: ConfirmPromptAction) {
        self.confirm_prompt_action = Some(action);
        self.confirm_prompt_selected = 0;
        self.menu_active = false;
        self.close_tool_chooser();
    }

    pub fn close_confirm_prompt(&mut self) {
        self.confirm_prompt_action = None;
        self.confirm_prompt_selected = 0;
    }

    pub fn toggle_tool_chooser(&mut self, kind: ToolChooserKind) {
        if self.open_tool_chooser == Some(kind) && self.desktop.is_open(WindowId::PowerPicker) {
            self.close_tool_chooser();
            return;
        }

        let win = self.desktop.window_mut(WindowId::PowerPicker);
        win.title = kind.title();
        win.width = 34;
        win.height = kind.tools().len() as u16 + 6;

        self.open_tool_chooser = Some(kind);
        self.desktop.open(WindowId::PowerPicker, true);
    }

    pub fn select_tool(&mut self, tool: Tool) {
        self.current_tool = tool;
        if tool.uses_underground_layer() {
            self.switch_view_layer(
                crate::core::map::ViewLayer::Underground,
                Some(format!("{} uses the Underground layer.", tool.label())),
            );
        }
        if let Some(kind) = ToolChooserKind::for_tool(tool) {
            self.set_chooser_tool(kind, tool);
        }
        self.close_tool_chooser();
        self.line_drag = None;
        self.rect_drag = None;
    }

    fn cycle_confirm_prompt_selection(&mut self, delta: i32) {
        if let Some(dialog) = self.confirm_dialog_view_model() {
            confirm_dialog::cycle_selection(
                &mut self.confirm_prompt_selected,
                dialog.button_count(),
                delta,
            );
        }
    }

    fn open_load_city_screen(&self) -> ScreenTransition {
        ScreenTransition::Push(Box::new(LoadCityScreen::new()))
    }

    fn confirm_prompt_transition(&self, action: ConfirmPromptAction) -> ScreenTransition {
        match action {
            ConfirmPromptAction::Quit => ScreenTransition::Quit,
            ConfirmPromptAction::LoadCity => self.open_load_city_screen(),
            ConfirmPromptAction::ReturnToStart => ScreenTransition::Pop,
        }
    }

    fn confirm_prompt(&mut self, context: &AppContext) -> Option<ScreenTransition> {
        let action = self.confirm_prompt_action?;
        let dialog = self.confirm_dialog_view_model()?;

        match dialog.selected_role() {
            Some(ConfirmDialogButtonRole::Accept) => {
                let result = {
                    let engine = context.engine.read().unwrap();
                    save::save_city(&engine.sim, &engine.map)
                };
                match result {
                    Ok(()) => {
                        self.close_confirm_prompt();
                        Some(self.confirm_prompt_transition(action))
                    }
                    Err(e) => {
                        self.push_message(format!("Save failed: {e}"));
                        None
                    }
                }
            }
            Some(ConfirmDialogButtonRole::Alternate) => {
                self.close_confirm_prompt();
                Some(self.confirm_prompt_transition(action))
            }
            Some(ConfirmDialogButtonRole::Cancel) | None => {
                self.close_confirm_prompt();
                None
            }
        }
    }
}

impl Screen for InGameScreen {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn on_tick(&mut self, context: AppContext) {
        {
            let engine = context.engine.read().unwrap();
            self.news_ticker
                .tick(&engine.sim, &engine.map, &self.event_messages);
        }

        // Sync window content heights
        self.desktop.window_mut(WindowId::Help).content_height = build_help_lines().len() as u16;
        self.desktop.window_mut(WindowId::About).content_height = ABOUT_LINES.len() as u16;
        self.desktop.window_mut(WindowId::Legend).content_height =
            build_legend_lines().len() as u16;

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
            engine.sim.pop.population > 0 && !self.first_building_notified
        };
        if first_building {
            self.first_building_notified = true;
            self.push_message("First residents have arrived!".to_string());
        }
        let treasury = context.engine.read().unwrap().sim.economy.treasury;
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
        if self.confirm_prompt_action.is_some() {
            return match action {
                Action::Quit => None,
                Action::MenuBack => {
                    self.close_confirm_prompt();
                    None
                }
                Action::MenuSelect => self.confirm_prompt(&context),
                Action::MoveCursor(dx, dy) => {
                    if dx < 0 || dy < 0 {
                        self.cycle_confirm_prompt_selection(-1);
                    } else if dx > 0 || dy > 0 {
                        self.cycle_confirm_prompt_selection(1);
                    }
                    None
                }
                Action::CharInput('y') | Action::CharInput('Y') => {
                    if let Some(dialog) = self.confirm_dialog_view_model() {
                        if let Some(index) = dialog.index_for_role(ConfirmDialogButtonRole::Accept)
                        {
                            self.confirm_prompt_selected = index;
                            return self.confirm_prompt(&context);
                        }
                    }
                    None
                }
                Action::CharInput('n') | Action::CharInput('N') => {
                    if let Some(dialog) = self.confirm_dialog_view_model() {
                        let index = dialog
                            .index_for_role(ConfirmDialogButtonRole::Alternate)
                            .or_else(|| dialog.index_for_role(ConfirmDialogButtonRole::Cancel));
                        if let Some(index) = index {
                            self.confirm_prompt_selected = index;
                            return self.confirm_prompt(&context);
                        }
                    }
                    None
                }
                Action::CharInput('c') | Action::CharInput('C') => {
                    if let Some(dialog) = self.confirm_dialog_view_model() {
                        if let Some(index) = dialog.index_for_role(ConfirmDialogButtonRole::Cancel)
                        {
                            self.confirm_prompt_selected = index;
                        }
                    }
                    self.confirm_prompt(&context)
                }
                Action::MouseClick { col, row } => {
                    if let Some(index) = self
                        .ui_areas
                        .dialog_items
                        .iter()
                        .position(|area| area.contains(col, row))
                    {
                        self.confirm_prompt_selected = index;
                        self.confirm_prompt(&context)
                    } else {
                        None
                    }
                }
                _ => None,
            };
        }

        if self.is_stats_open()
            || self.is_about_open()
            || self.is_help_open()
            || self.is_legend_open()
        {
            return match action {
                Action::MenuBack => {
                    if self.is_stats_open() {
                        self.close_stats_window();
                    } else if self.is_about_open() {
                        self.close_about_window();
                    } else if self.is_help_open() {
                        self.close_help_window();
                    } else {
                        self.close_legend_window();
                    }
                    None
                }
                Action::MoveCursor(_, dy) if dy != 0 => {
                    let id = if self.is_about_open() {
                        WindowId::About
                    } else if self.is_help_open() {
                        WindowId::Help
                    } else if self.is_legend_open() {
                        WindowId::Legend
                    } else {
                        WindowId::Statistics
                    };
                    let win = self.desktop.window_mut(id);
                    if dy < 0 {
                        win.scroll_y = win.scroll_y.saturating_sub(1);
                    } else {
                        win.scroll_y = win.scroll_y.saturating_add(1);
                    }
                    None
                }
                Action::PanCamera(_, dy) if dy != 0 => {
                    let id = if self.is_about_open() {
                        WindowId::About
                    } else if self.is_help_open() {
                        WindowId::Help
                    } else if self.is_legend_open() {
                        WindowId::Legend
                    } else {
                        WindowId::Statistics
                    };
                    let win = self.desktop.window_mut(id);
                    if dy < 0 {
                        win.scroll_y = win.scroll_y.saturating_sub(3);
                    } else {
                        win.scroll_y = win.scroll_y.saturating_add(3);
                    }
                    None
                }
                Action::MouseClick { col, row } => {
                    self.handle_mouse_click_action(col, row, &context);
                    None
                }
                Action::MouseDrag { col, row } => {
                    self.handle_mouse_drag_action(col, row, &context);
                    None
                }
                Action::MouseUp { col, row } => {
                    self.handle_mouse_up_action(col, row, &context);
                    None
                }
                Action::Quit => {
                    self.open_confirm_prompt(ConfirmPromptAction::Quit);
                    None
                }
                _ => None,
            };
        }

        if let Action::MouseClick { col, row } = action {
            let (consumed, transition) = self.handle_menu_click(col, row, &context);
            if consumed {
                return transition;
            }
        }

        if matches!(action, Action::Quit) {
            self.open_confirm_prompt(ConfirmPromptAction::Quit);
            return None;
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
                        self.menu_selected = (self.menu_selected + 1)
                            .min(crate::app::screens::MENU_TITLES.len().saturating_sub(1));
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

        if self.is_tool_chooser_open() && matches!(action, Action::MenuBack) {
            self.close_tool_chooser();
            return None;
        }

        if self.is_inspect_open() && matches!(action, Action::MenuBack) {
            self.close_inspect_window();
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
                    self.open_confirm_prompt(ConfirmPromptAction::ReturnToStart);
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
                if self.current_tool != Tool::Inspect
                    && !Tool::uses_footprint_preview(self.current_tool)
                {
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
                if c == 'P' {
                    let next = theme::cycle_theme();
                    let _ = config::persist_theme_preference(next);
                    self.push_message(format!("Theme: {}", next.label()));
                    return None;
                }
                if c == '\t' {
                    self.overlay_mode = self.overlay_mode.next();
                    return None;
                }
                if c == 'u' || c == 'U' {
                    self.toggle_view_layer();
                    return None;
                }
                let new_tool = match c {
                    'q' => {
                        self.open_confirm_prompt(ConfirmPromptAction::Quit);
                        None
                    }
                    ' ' => {
                        self.paused = !self.paused;
                        None
                    }
                    '?' => Some(Tool::Inspect),
                    '1' => Some(Tool::ZoneResLight),
                    '2' => Some(Tool::ZoneResDense),
                    '3' => Some(Tool::ZoneCommLight),
                    '4' => Some(Tool::ZoneCommDense),
                    '5' => Some(Tool::ZoneIndLight),
                    '6' => Some(Tool::ZoneIndDense),
                    'r' => Some(Tool::Road),
                    'h' => Some(Tool::Highway),
                    'o' => Some(Tool::Onramp),
                    'l' => Some(Tool::Rail),
                    'p' => Some(Tool::PowerLine),
                    'w' => Some(Tool::WaterPipe),
                    'm' => Some(Tool::Subway),
                    'e' => Some(Tool::PowerPlantCoal),
                    'g' => Some(Tool::PowerPlantGas),
                    'd' => Some(Tool::BusDepot),
                    't' => Some(Tool::RailDepot),
                    'n' => Some(Tool::SubwayStation),
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
                if self.ui_areas.map.viewport.contains(col, row) && !self.is_over_window(col, row) {
                    let (mx, my) = self.screen_to_map_clamped(col, row, &context);
                    self.camera.cursor_x = mx;
                    self.camera.cursor_y = my;
                    if self.current_tool == Tool::Inspect && self.is_inspect_open() {
                        self.inspect_pos = Some((mx, my));
                    }
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
    use crate::{
        app::screens::BudgetFocus,
        core::{engine::SimulationEngine, map::Map, sim::SimState},
        ui::runtime::ToolChooserKind,
    };
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
    fn paused_tick_still_advances_news_ticker() {
        let mut screen = fresh_screen();
        screen.paused = true;
        let engine = Arc::new(RwLock::new(SimulationEngine::new(
            Map::new(4, 4),
            SimState::default(),
        )));
        let mut running = true;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &None,
            running: &mut running,
        };

        screen.on_tick(context);
        let start = screen.news_ticker.view_model().scroll_offset;

        for _ in 0..4 {
            let mut running = true;
            screen.on_tick(AppContext {
                engine: &engine,
                cmd_tx: &None,
                running: &mut running,
            });
        }

        assert!(screen.news_ticker.view_model().scroll_offset > start);
    }

    #[test]
    fn overlay_mode_tab_cycles() {
        let mut screen = fresh_screen();
        assert_eq!(screen.overlay_mode, OverlayMode::None);
        screen.overlay_mode = screen.overlay_mode.next();
        assert_eq!(screen.overlay_mode, OverlayMode::Power);
        screen.overlay_mode = screen.overlay_mode.next();
        assert_eq!(screen.overlay_mode, OverlayMode::Water);
    }

    #[test]
    fn select_tool_updates_chooser_memory_and_closes_popup() {
        let mut screen = fresh_screen();
        screen.open_tool_chooser = Some(ToolChooserKind::Zones);
        screen.desktop.open(WindowId::PowerPicker, false);

        screen.select_tool(Tool::ZoneCommLight);

        assert_eq!(screen.current_tool, Tool::ZoneCommLight);
        assert_eq!(screen.zone_tool, Tool::ZoneCommLight);
        assert_eq!(screen.open_tool_chooser, None);
        assert!(!screen.desktop.is_open(WindowId::PowerPicker));
    }

    #[test]
    fn select_tool_switches_to_underground_with_notice_for_pipes() {
        let mut screen = fresh_screen();

        screen.select_tool(Tool::WaterPipe);

        assert_eq!(screen.view_layer, crate::core::map::ViewLayer::Underground);
        assert_eq!(
            screen
                .event_messages
                .front()
                .map(|(message, _)| message.as_str()),
            Some("Water Pipe uses the Underground layer.")
        );
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
    fn u_shortcut_toggles_layer_without_changing_tool() {
        let mut screen = fresh_screen();
        screen.current_tool = Tool::Road;
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
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

        let transition = screen.on_action(Action::CharInput('u'), context);

        assert!(transition.is_none());
        assert_eq!(screen.view_layer, crate::core::map::ViewLayer::Underground);
        assert_eq!(screen.current_tool, Tool::Road);
        assert_eq!(
            screen
                .event_messages
                .front()
                .map(|(message, _)| message.as_str()),
            Some("View layer: Underground")
        );
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
        screen.ui_areas.map.viewport = crate::app::ClickArea {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };
        screen.ui_areas.map.horizontal_bar = crate::app::ClickArea {
            x: 0,
            y: 10,
            width: 20,
            height: 1,
        };
        screen.ui_areas.map.horizontal_inc = crate::app::ClickArea {
            x: 19,
            y: 10,
            width: 1,
            height: 1,
        };

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
    fn clicking_statusbar_layer_switch_changes_view_layer() {
        let mut screen = fresh_screen();
        screen.ui_areas.layer_underground_btn = crate::app::ClickArea {
            x: 30,
            y: 1,
            width: 12,
            height: 1,
        };
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(20, 20),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = None;
        let mut running = true;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };

        let consumed = screen.handle_mouse_click_action(31, 1, &context);

        assert!(consumed);
        assert_eq!(screen.view_layer, crate::core::map::ViewLayer::Underground);
    }

    #[test]
    fn outside_click_closes_open_tool_chooser() {
        let mut screen = fresh_screen();
        screen.toggle_tool_chooser(ToolChooserKind::Buildings);
        screen.desktop.window_mut(WindowId::Panel).visible = false;
        screen.desktop.window_mut(WindowId::Map).visible = false;

        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
            crate::core::map::Map::new(20, 20),
            crate::core::sim::SimState::default(),
        )));
        let cmd_tx = None;
        let mut running = true;
        let context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };

        let consumed = screen.handle_mouse_click_action(40, 20, &context);
        assert!(consumed);
        assert_eq!(screen.open_tool_chooser, None);
        assert!(!screen.desktop.is_open(WindowId::PowerPicker));
    }

    #[test]
    fn minimap_click_centers_camera_even_inside_panel() {
        let mut screen = fresh_screen();
        screen.camera.view_w = 20;
        screen.camera.view_h = 10;
        screen.ui_areas.minimap = crate::app::ClickArea {
            x: 10,
            y: 10,
            width: 10,
            height: 5,
        };
        let panel = screen.desktop.window_mut(WindowId::Panel);
        panel.visible = true;
        panel.x = 8;
        panel.y = 8;
        panel.width = 20;
        panel.height = 12;

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

        let consumed = screen.handle_mouse_click_action(19, 14, &context);
        assert!(consumed);
        assert!(screen.camera.offset_x > 0);
        assert!(screen.camera.offset_y > 0);
    }

    #[test]
    fn inspect_click_updates_position_without_opening_window() {
        let mut screen = fresh_screen();
        screen.current_tool = Tool::Inspect;
        screen.ui_areas.map.viewport = crate::app::ClickArea {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };
        screen.desktop.window_mut(WindowId::Panel).visible = false;
        screen.desktop.window_mut(WindowId::Map).visible = false;

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

        let consumed = screen.handle_mouse_click_action(6, 4, &context);

        assert!(consumed);
        assert_eq!(screen.inspect_pos, Some((3, 4)));
        assert!(!screen.is_inspect_open());
    }

    #[test]
    fn opening_inspect_window_seeds_current_cursor_tile() {
        let mut screen = fresh_screen();
        screen.camera.cursor_x = 7;
        screen.camera.cursor_y = 9;

        screen.open_inspect_window();

        assert!(screen.is_inspect_open());
        assert_eq!(screen.inspect_pos, Some((7, 9)));
    }

    #[test]
    fn mouse_move_updates_cursor_for_single_tile_tools() {
        let mut screen = fresh_screen();
        screen.current_tool = Tool::WaterPump;
        screen.ui_areas.map.viewport = crate::app::ClickArea {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };
        screen.desktop.window_mut(WindowId::Panel).visible = false;
        screen.desktop.window_mut(WindowId::Map).visible = false;

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

        screen.on_action(Action::MouseMove { col: 8, row: 3 }, context);

        assert_eq!(screen.camera.cursor_x, 4);
        assert_eq!(screen.camera.cursor_y, 3);
    }

    #[test]
    fn quit_action_opens_confirm_prompt_instead_of_quitting() {
        let mut screen = fresh_screen();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
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

        let transition = screen.on_action(Action::Quit, context);

        assert!(transition.is_none());
        assert_eq!(
            screen.confirm_prompt_action,
            Some(ConfirmPromptAction::Quit)
        );
        assert!(running);
    }

    #[test]
    fn menu_back_opens_return_to_start_confirm_prompt() {
        let mut screen = fresh_screen();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
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

        let transition = screen.on_action(Action::MenuBack, context);

        assert!(transition.is_none());
        assert_eq!(
            screen.confirm_prompt_action,
            Some(ConfirmPromptAction::ReturnToStart)
        );
    }

    #[test]
    fn confirming_return_to_start_without_saving_pops_screen() {
        let mut screen = fresh_screen();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
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
        screen.open_confirm_prompt(ConfirmPromptAction::ReturnToStart);
        screen.confirm_prompt_selected = 1;

        let transition = screen.confirm_prompt(&context);

        assert!(matches!(transition, Some(ScreenTransition::Pop)));
        assert_eq!(screen.confirm_prompt_action, None);
    }

    #[test]
    fn load_city_menu_action_opens_confirm_prompt() {
        let mut screen = fresh_screen();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
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

        let transition =
            screen.handle_menu_action(super::super::ingame_menu::MenuAction::LoadCity, &context);

        assert!(transition.is_none());
        assert_eq!(
            screen.confirm_prompt_action,
            Some(ConfirmPromptAction::LoadCity)
        );
    }

    #[test]
    fn confirming_load_city_without_saving_pushes_screen_and_clears_prompt() {
        let mut screen = fresh_screen();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
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
        screen.open_confirm_prompt(ConfirmPromptAction::LoadCity);
        screen.confirm_prompt_selected = 1;

        let transition = screen.confirm_prompt(&context);

        assert!(matches!(transition, Some(ScreenTransition::Push(_))));
        assert_eq!(screen.confirm_prompt_action, None);
    }

    #[test]
    fn pressing_n_uses_the_alternate_confirm_action() {
        let mut screen = fresh_screen();
        let engine = Arc::new(RwLock::new(crate::core::engine::SimulationEngine::new(
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
        screen.open_confirm_prompt(ConfirmPromptAction::LoadCity);

        let transition = screen.on_action(Action::CharInput('n'), context);

        assert!(matches!(transition, Some(ScreenTransition::Push(_))));
        assert_eq!(screen.confirm_prompt_action, None);
    }

    #[test]
    fn budget_open_syncs_sector_sliders_to_sim_tax_rates() {
        let mut screen = fresh_screen();
        let mut sim = crate::core::sim::SimState::default();
        sim.economy.tax_rates.residential = 13;
        sim.economy.tax_rates.commercial = 11;
        sim.economy.tax_rates.industrial = 7;
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
        screen.ui_areas.desktop = screen.desktop.layout(UiRect::new(0, 0, 100, 40));

        let start_x = screen.desktop.window(WindowId::Budget).x;
        let start_y = screen.desktop.window(WindowId::Budget).y;
        let click_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(
            Action::MouseClick {
                col: start_x + 2,
                row: start_y,
            },
            click_context,
        );
        let drag_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(
            Action::MouseDrag {
                col: start_x + 6,
                row: start_y + 3,
            },
            drag_context,
        );

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
        screen.budget_ui.focused = BudgetFocus::Commercial;
        screen.budget_ui.commercial_tax_input.clear();

        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::CharInput('4'), event_context);
        let cmd = rx
            .try_recv()
            .expect("budget text input should emit a tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        screen.on_action(Action::CharInput('2'), event_context);
        let cmd = rx
            .try_recv()
            .expect("budget text input should emit a second tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        assert_eq!(screen.budget_ui.commercial_tax_input, "42");
        assert_eq!(screen.budget_ui.commercial_tax, 42);
        assert_eq!(engine.read().unwrap().sim.economy.tax_rates.commercial, 42);
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
        screen.budget_ui.focused = BudgetFocus::Residential;
        screen.budget_ui.residential_tax_input.clear();

        for key in ['1', '2', '3'] {
            let event_context = AppContext {
                engine: &engine,
                cmd_tx: &cmd_tx,
                running: &mut running,
            };
            screen.on_action(Action::CharInput(key), event_context);
            let cmd = rx
                .try_recv()
                .expect("typed tax input should emit a tax update command");
            engine.write().unwrap().execute_command(cmd).unwrap();
        }

        assert_eq!(screen.budget_ui.residential_tax_input, "100");
        assert_eq!(screen.budget_ui.residential_tax, 100);
        assert_eq!(engine.read().unwrap().sim.economy.tax_rates.residential, 100);
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
        screen.budget_ui.focused = BudgetFocus::Commercial;
        let start_value = screen.budget_ui.commercial_tax;

        let event_context = AppContext {
            engine: &engine,
            cmd_tx: &cmd_tx,
            running: &mut running,
        };
        let transition = screen.on_action(Action::MoveCursor(-1, 0), event_context);
        assert!(transition.is_none());
        let cmd = rx
            .try_recv()
            .expect("left key adjustment should emit a tax update command");
        engine.write().unwrap().execute_command(cmd).unwrap();

        assert_eq!(
            screen.budget_ui.commercial_tax,
            start_value.saturating_sub(1)
        );
        assert_eq!(
            screen.budget_ui.commercial_tax_input,
            (start_value.saturating_sub(1)).to_string()
        );
        assert_eq!(
            engine.read().unwrap().sim.economy.tax_rates.commercial as usize,
            start_value.saturating_sub(1)
        );
    }
}
